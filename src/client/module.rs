use crate::dependency::WithDependencies;
use serde::Deserialize;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

/// The type of the module
#[derive(Debug, Deserialize)]
pub enum ModuleKindV1 {
    /// A task is a module with a limited lifetime, used to perform some
    /// temporary operation or some setup.
    Task,
    /// A service is a longer running module. It's lifetime will be managed and
    /// can be started, stopped independently.
    Service,
}

/// A definition of a module for version 1 (V1) of the daemon.
#[derive(Debug, Deserialize)]
pub struct ModuleDefinitionV1 {
    pub kind: ModuleKindV1,
    pub name: String,
    pub command: Vec<String>,
    pub environment: HashMap<String, String>,
    pub log_file_path: Option<String>,
    pub dependencies: Vec<String>,
}

impl ModuleDefinitionV1 {
    pub fn new(
        kind: ModuleKindV1,
        name: String,
        command: Vec<String>,
        environment: HashMap<String, String>,
        log_file_path: Option<String>,
        dependencies: Vec<String>,
    ) -> ModuleDefinitionV1 {
        ModuleDefinitionV1 {
            kind,
            name,
            command,
            environment,
            log_file_path,
            dependencies,
        }
    }
}

impl Hash for ModuleDefinitionV1 {
    fn hash<S: Hasher>(&self, state: &mut S) {
        self.name.hash(state);
    }
}

impl PartialEq for ModuleDefinitionV1 {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

impl Eq for ModuleDefinitionV1 {}

impl WithDependencies for ModuleDefinitionV1 {
    fn key(&self) -> String {
        self.name.clone()
    }

    fn key_ref(&self) -> &str {
        self.name.as_str()
    }

    fn dependencies(&self) -> &Vec<String> {
        &self.dependencies
    }
}
