use crate::client::cli::ClientConfig;
use crate::client::config::read_module_definitions;
use crate::client::emoji::{LINK, LOOKING_GLASS, SUCCESS, TEXTBOOK, VAN};
use crate::client::module::{module_names_set, remove_checks};
use crate::client::module::{
    CheckDefinition, GroupDefinition, InnerDefinition, ModuleDefinition,
    ModuleMarker, ServiceOrTaskDefinition,
};
use crate::client::process::run_check;
use crate::client::progress::{SpinnerOptions, WaitResult, WaitUntil};
use crate::client::request;
use crate::client::validation::validate_modules_selected;
use crate::daemon::api::ApiHealthStatus;
use crate::dependency::{DependencyGraph, DependencyNode};
use anyhow::{anyhow, bail, Result};
use clap::ArgMatches;
use console::style;
use std::collections::HashMap;
use std::thread;
use std::time::Duration;

pub struct DeployOptions {
    force_deploy: bool,
    skip_checks: bool,
    skip_healthchecks: bool,
}

impl DeployOptions {
    pub fn from(opts: &ArgMatches) -> DeployOptions {
        let force_deploy = opts.is_present("force");
        let skip_healthchecks = opts.is_present("skip_healthchecks");
        let skip_checks = opts.is_present("skip_checks");
        Self {
            force_deploy,
            skip_healthchecks,
            skip_checks,
        }
    }
}

pub fn deploy_cmd(
    modules_to_deploy: Vec<&str>,
    cfg: &ClientConfig,
    deploy_opts: &DeployOptions,
) -> Result<()> {
    tprintstep!("Looking for module definitions...", 1, 5, LOOKING_GLASS);
    let mut module_defs = read_module_definitions(&cfg)?;
    let checks_map = remove_checks(&mut module_defs);
    let module_names = module_names_set(&module_defs);

    tprintstep!("Resolving dependencies...", 2, 5, LINK);

    validate_modules_selected(&module_names, &modules_to_deploy)?;

    let dependency_graph =
        DependencyGraph::from(&module_defs, &modules_to_deploy);
    let ordered = dependency_graph.dependency_sort()?;

    run_checks(checks_map, &ordered, deploy_opts)?;

    tprintstep!("Deploying...", 4, 5, VAN);

    for m in &ordered {
        match m.value.inner {
            InnerDefinition::Task(ref task) => deploy_task(task, cfg),
            InnerDefinition::Service(ref service) => {
                deploy_and_maybe_wait_service(
                    service,
                    m.marker,
                    cfg,
                    deploy_opts,
                )
            }
            InnerDefinition::Group(ref group) => {
                deploy_group(group);
                Ok(())
            }
            InnerDefinition::Check(_) => Ok(()),
        }?;
    }

    let deploy_txt = format!(
        "{}: {:?}",
        style("Deployed modules").bold().green(),
        &ordered.iter().map(|m| &m.value.name).collect::<Vec<_>>()
    );
    tprintstep!(deploy_txt, 5, 5, SUCCESS);
    Ok(())
}

fn run_checks(
    checks_map: HashMap<String, CheckDefinition>,
    modules: &[&DependencyNode<&ModuleDefinition, ModuleMarker>],
    deploy_opts: &DeployOptions,
) -> Result<()> {
    if deploy_opts.skip_checks {
        let msg = format!(
            "Running checks... {}",
            style("(Skip)").bold().white().dim()
        );
        tprintstep!(msg, 3, 5, TEXTBOOK);
    } else {
        tprintstep!("Running checks...", 3, 5, TEXTBOOK);
        for m in modules {
            let checks = match &m.value.inner {
                InnerDefinition::Group(grp) => grp.checks.as_slice(),
                InnerDefinition::Service(srvc) => srvc.checks.as_slice(),
                InnerDefinition::Task(tsk) => tsk.checks.as_slice(),
                _ => &[],
            };

            for check in checks {
                let check = checks_map
                    .get(check)
                    .ok_or_else(|| anyhow!("Check '{}' not defined", check))?;

                perform_check(check)?;
            }
        }
    }
    Ok(())
}

fn perform_check(check_def: &CheckDefinition) -> Result<()> {
    let message = format!(
        "Check {} ({})",
        style(&check_def.about).white().bold(),
        check_def.name
    );
    let spin_opt = SpinnerOptions::new(message).clear_on_finish(false);
    let mut wu = WaitUntil::new(&spin_opt);

    let check_result = wu.spin_until_status(|| {
        let check_result = run_check(check_def)?;
        let status = if check_result.success() {
            style("(OK)").green().bold()
        } else {
            style("(FAIL)").red().bold()
        };
        Ok(WaitResult::from(check_result, status.to_string()))
    })?;

    if !check_result.success() {
        bail!(
            "The {} check has failed\n\
            {}: {}",
            style(&check_def.about).white().bold(),
            style("Message").white().bold(),
            check_def.help
        )
    }
    Ok(())
}

fn deploy_and_maybe_wait_service(
    service: &ServiceOrTaskDefinition,
    marker: Option<ModuleMarker>,
    cfg: &ClientConfig,
    deploy_opts: &DeployOptions,
) -> Result<()> {
    let monitor_handle = deploy_service(service, cfg, deploy_opts)?;
    if let Some(handle) = monitor_handle {
        if (marker == Some(ModuleMarker::WaitHealthcheck)
            || service.always_wait_healthcheck)
            && !deploy_opts.skip_healthchecks
        {
            wait_until_healthy(service.name.as_str(), handle.as_str(), cfg)?;
        }
    }
    Ok(())
}

fn deploy_service(
    module: &ServiceOrTaskDefinition,
    cfg: &ClientConfig,
    deploy_opts: &DeployOptions,
) -> Result<Option<String>> {
    let message = format!("Deploying {}", style(&module.name).white().bold());
    let spin_opt = SpinnerOptions::new(message).clear_on_finish(false);

    let mut wu = WaitUntil::new(&spin_opt);
    let deploy_result = wu.spin_until_status(|| {
        let result = request::deploy_module(
            module,
            deploy_opts.force_deploy,
            &cfg.daemon_url,
        )?;

        let deploy_status = if result.deployed {
            style("(Deployed)").green().bold()
        } else {
            style("(Already deployed)").white().dim().bold()
        };
        Ok(WaitResult::from(result, deploy_status.to_string()))
    })?;

    let monitor_handle = deploy_result.monitor;
    Ok(monitor_handle)
}

fn wait_until_healthy(
    module_name: &str,
    monitor_handle: &str,
    cfg: &ClientConfig,
) -> Result<()> {
    let message = format!(
        "Waiting {} to be healthy",
        style(module_name).white().bold()
    );
    let spin_opt = SpinnerOptions::new(message).clear_on_finish(false);
    let mut wu = WaitUntil::new(&spin_opt);

    wu.spin_until_status(|| loop {
        let status = style("(Done)").green().bold().to_string();
        match request::poll_health(monitor_handle, &cfg.daemon_url)?
            .healthcheck_status
        {
            Some(ApiHealthStatus::Successful) => {
                break Ok(WaitResult::from((), status))
            }
            Some(ApiHealthStatus::RetriesExceeded) => {
                bail!(
                    "The service did not complete its healthcheck in time.\n\
                       Check the logs for more details."
                )
            }
            Some(ApiHealthStatus::Error) => {
                bail!(
                    "An error occured while waiting for the service \
                    healthcheck to complete.\nThis is usually a mistake in \
                    the healthcheck configuration, ensure the command or \
                    condition is correct."
                )
            }
            Some(ApiHealthStatus::Pending) | None => {
                thread::sleep(Duration::from_secs(2));
            }
        }
    })?;

    Ok(())
}

fn deploy_task(
    module: &ServiceOrTaskDefinition,
    cfg: &ClientConfig,
) -> Result<()> {
    let message =
        format!("Running task {}", style(&module.name).white().bold());
    let spin_opt = SpinnerOptions::new(message).clear_on_finish(false);

    let mut wu = WaitUntil::new(&spin_opt);
    wu.spin_until_status(|| {
        let result = request::deploy_task(module, &cfg.daemon_url)?;
        let status = style("(Done)").green().bold().to_string();
        Ok(WaitResult::from(result, status))
    })?;

    Ok(())
}

fn deploy_group(module: &GroupDefinition) {
    let message = format!("Group {}", style(&module.name).white().bold());
    tiprint!(
        10, // indent level
        "{} {}",
        message,
        style("(Done)").green().bold()
    );
}
