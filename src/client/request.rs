use crate::client::module::ServiceOrTaskDefinitionV1;
use crate::daemon::api::*;
use anyhow::{bail, Result};

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
pub enum DeploymentResponse {
    Ok(ApiDeploymentResponse),
    Err(ErrorResponse),
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
pub enum TaskDeploymentResponse {
    Ok(ApiTaskDeploymentResponse),
    Err(ErrorResponse),
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
pub enum OperationResponse {
    Ok(ApiOperationResponse),
    Err(ErrorResponse),
}

pub fn deploy_modules(
    services_to_deploy: &Vec<&str>,
    module_definitions: &Vec<ServiceOrTaskDefinitionV1>,
    daemon_url: &String,
) -> Result<ApiDeploymentResponse> {
    let client = reqwest::blocking::Client::new();
    let command = ApiDeploymentCommand {
        to_deploy: services_to_deploy.iter().map(|s| s.to_string()).collect(),
        module_definitions: module_definitions
            .iter()
            .map(|m| ApiModuleDefinition {
                name: m.name.clone(),
                command: m.command.clone(),
                environment: m.environment.clone(),
                log_file_path: m.log_file_path.clone(),
                dependencies: m.dependencies.clone(),
                working_dir: m.working_dir.clone(),
            })
            .collect(),
    };

    let deployment_result: DeploymentResponse = client
        .post(&(daemon_url.to_owned() + "/deploy"))
        .json(&command)
        .send()?
        .json()?;

    match deployment_result {
        DeploymentResponse::Ok(r) => Ok(r),
        DeploymentResponse::Err(e) => bail!(e.message),
    }
}

pub fn deploy_task(
    task_definition: &ServiceOrTaskDefinitionV1,
    daemon_url: &String,
) -> Result<ApiTaskDeploymentResponse> {
    let client = reqwest::blocking::Client::new();
    let command = ApiTaskDeploymentCommand {
        task_definition: ApiModuleDefinition {
            name: task_definition.name.clone(),
            command: task_definition.command.clone(),
            environment: task_definition.environment.clone(),
            log_file_path: task_definition.log_file_path.clone(),
            dependencies: task_definition.dependencies.clone(),
            working_dir: task_definition.working_dir.clone(),
        },
    };

    let deployment_result: TaskDeploymentResponse = client
        .post(&(daemon_url.to_owned() + "/tasks/deploy"))
        .json(&command)
        .send()?
        .json()?;

    match deployment_result {
        TaskDeploymentResponse::Ok(r) => Ok(r),
        TaskDeploymentResponse::Err(e) => bail!(e.message),
    }
}

pub fn stop_module(
    module_name: &str,
    daemon_url: &String,
) -> Result<ApiOperationResponse> {
    let client = reqwest::blocking::Client::new();
    let command = ApiOperationCommand {
        operation: ApiModuleOperation::STOP,
        module_name: module_name.to_string(),
    };

    let operation_result: OperationResponse = client
        .post(&(daemon_url.to_owned() + "/operation"))
        .json(&command)
        .send()?
        .json()?;

    match operation_result {
        OperationResponse::Ok(r) => Ok(r),
        OperationResponse::Err(e) => bail!(e.message),
    }
}

pub fn restart_module(
    module_name: &str,
    daemon_url: &String,
) -> Result<ApiOperationResponse> {
    let client = reqwest::blocking::Client::new();
    let command = ApiOperationCommand {
        operation: ApiModuleOperation::RESTART,
        module_name: module_name.to_string(),
    };

    let operation_result: OperationResponse = client
        .post(&(daemon_url.to_owned() + "/operation"))
        .json(&command)
        .send()?
        .json()?;

    match operation_result {
        OperationResponse::Ok(r) => Ok(r),
        OperationResponse::Err(e) => bail!(e.message),
    }
}

pub fn list_modules(daemon_url: &String) -> Result<ApiModuleStatusResponse> {
    let client = reqwest::blocking::Client::new();
    let status = client
        .get(&(daemon_url.to_owned() + "/status"))
        .send()?
        .json()?;

    Ok(status)
}

pub fn log_info(
    module_name: &str,
    daemon_url: &String,
) -> Result<ApiLogResponse> {
    let client = reqwest::blocking::Client::new();
    let status = client
        .get(&(daemon_url.to_owned() + "/log/" + module_name))
        .send()?
        .json()?;

    Ok(status)
}
