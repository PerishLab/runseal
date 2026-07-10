use super::config::Config;

pub trait Env: Send + Sync {
    fn var(&self, key: &str) -> Option<String>;
}

pub trait Context: Send + Sync {
    fn config(&self) -> &Config;
    fn env(&self) -> &dyn Env;
}

pub struct Process;

impl Env for Process {
    fn var(&self, key: &str) -> Option<String> {
        std::env::var(key).ok()
    }
}

pub struct App {
    config: Config,
    env: Process,
}

impl App {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            env: Process,
        }
    }
}

impl Context for App {
    fn config(&self) -> &Config {
        &self.config
    }

    fn env(&self) -> &dyn Env {
        &self.env
    }
}
