use std::collections::HashMap;

use serde::Deserialize;

fn _vec_string_new() -> Vec<String> {
    Vec::new()
}

fn _string_new() -> String {
    ".".into()
}

fn _u32_one() -> u32 {
    1
}

fn _bool_true() -> bool {
    true
}

fn _bool_false() -> bool {
    false
}

#[derive(Debug, Deserialize)]
pub struct AppSpec {
    pub exec: String,
    #[serde(default = "_vec_string_new")]
    pub args: Vec<String>,
    #[serde(default = "_vec_string_new")]
    pub env: Vec<String>,
    #[serde(default = "_string_new")]
    pub workdir: String,
    #[serde(default = "_bool_true")]
    pub stdout: bool,
    #[serde(default = "_bool_true")]
    pub stderr: bool,
    #[serde(default = "_bool_true")]
    pub restart: bool,
    #[serde(default = "_u32_one", rename = "restartDelay")]
    pub restart_delay: u32,
    #[serde(default = "_bool_false")]
    disable: bool,
}

#[derive(Debug, Deserialize)]
pub struct Spec {
    pub apps: HashMap<String, AppSpec>,
}

#[derive(Debug)]
pub struct AppInfo {
    pub name: String,
    pub exec: String,
    pub args: Vec<String>,
    pub env: Vec<String>,
    pub workdir: String,
    pub stdout: bool,
    pub stderr: bool,
    pub restart: bool,
    pub restart_delay: u32,
    pub disable: bool,
}

impl AppInfo {
    pub fn new(name: String, app_spec: AppSpec) -> AppInfo {
        AppInfo {
            name: name,
            exec: app_spec.exec,
            args: app_spec.args,
            env: app_spec.env,
            workdir: app_spec.workdir,
            stdout: app_spec.stdout,
            stderr: app_spec.stderr,
            restart: app_spec.restart,
            restart_delay: app_spec.restart_delay,
            disable: app_spec.disable,
        }
    }
}
