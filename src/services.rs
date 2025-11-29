use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ServiceState {
    Stopped,
    Starting,
    Running,
    Stopping,
    Failed,
}

pub struct Service {
    pub name: String,
    pub state: ServiceState,
    pub auto_start: bool,
    pub dependencies: Vec<String>,
    pub pid: Option<u32>,
}

impl Service {
    pub fn new(name: &str, auto_start: bool, dependencies: Vec<String>) -> Self {
        Service {
            name: name.to_string(),
            state: ServiceState::Stopped,
            auto_start,
            dependencies,
            pid: None,
        }
    }

    pub fn start(&mut self, pid: u32) -> bool {
        if self.state != ServiceState::Stopped {
            return false;
        }
        self.state = ServiceState::Starting;
        self.pid = Some(pid);
        self.state = ServiceState::Running;
        true
    }

    pub fn stop(&mut self) -> bool {
        if self.state != ServiceState::Running {
            return false;
        }
        self.state = ServiceState::Stopping;
        self.pid = None;
        self.state = ServiceState::Stopped;
        true
    }

    pub fn fail(&mut self) {
        self.state = ServiceState::Failed;
        self.pid = None;
    }
}

pub struct ServiceManager {
    services: HashMap<String, Service>,
}

impl Default for ServiceManager {
    fn default() -> Self {
        ServiceManager::new()
    }
}

impl ServiceManager {
    pub fn new() -> Self {
        let mut manager = ServiceManager {
            services: HashMap::new(),
        };

        // Register core system services
        manager.register("init", true, vec![]);
        manager.register("network", true, vec!["init".to_string()]);
        manager.register("shell", true, vec!["init".to_string()]);
        manager.register("logger", true, vec!["init".to_string()]);
        manager.register("scheduler", true, vec!["init".to_string()]);

        manager
    }

    pub fn register(&mut self, name: &str, auto_start: bool, dependencies: Vec<String>) {
        let service = Service::new(name, auto_start, dependencies);
        self.services.insert(name.to_string(), service);
    }

    pub fn start(&mut self, name: &str, pid: u32) -> Result<(), String> {
        // Check dependencies
        if let Some(service) = self.services.get(name) {
            for dep in &service.dependencies {
                if let Some(dep_service) = self.services.get(dep) {
                    if dep_service.state != ServiceState::Running {
                        return Err(format!("Dependency {} not running", dep));
                    }
                } else {
                    return Err(format!("Dependency {} not found", dep));
                }
            }
        }

        if let Some(service) = self.services.get_mut(name) {
            if service.start(pid) {
                Ok(())
            } else {
                Err(format!(
                    "Service {} cannot be started from current state",
                    name
                ))
            }
        } else {
            Err(format!("Service {} not found", name))
        }
    }

    pub fn stop(&mut self, name: &str) -> Result<(), String> {
        // Check if any running services depend on this one
        for (svc_name, svc) in &self.services {
            if svc.state == ServiceState::Running && svc.dependencies.contains(&name.to_string()) {
                return Err(format!("Service {} depends on {}", svc_name, name));
            }
        }

        if let Some(service) = self.services.get_mut(name) {
            if service.stop() {
                Ok(())
            } else {
                Err(format!(
                    "Service {} cannot be stopped from current state",
                    name
                ))
            }
        } else {
            Err(format!("Service {} not found", name))
        }
    }

    pub fn restart(&mut self, name: &str, new_pid: u32) -> Result<(), String> {
        self.stop(name)?;
        self.start(name, new_pid)?;
        Ok(())
    }

    pub fn list(&self) -> Vec<String> {
        let mut result = Vec::new();
        let mut services: Vec<_> = self.services.values().collect();
        services.sort_by_key(|s| &s.name);

        for service in services {
            let state_str = match service.state {
                ServiceState::Stopped => "stopped",
                ServiceState::Starting => "starting",
                ServiceState::Running => "running",
                ServiceState::Stopping => "stopping",
                ServiceState::Failed => "failed",
            };
            let auto = if service.auto_start { "[auto]" } else { "" };
            let pid_str = service
                .pid
                .map(|p| format!(" (pid: {})", p))
                .unwrap_or_default();

            result.push(format!(
                "{:<15} {:<10} {}{}",
                service.name, state_str, auto, pid_str
            ));
        }
        result
    }

    pub fn get_state(&self, name: &str) -> Option<ServiceState> {
        self.services.get(name).map(|s| s.state)
    }

    pub fn auto_start_services(&mut self, spawn_pid_fn: &mut dyn FnMut(&str) -> u32) {
        let auto_start_services: Vec<String> = self
            .services
            .values()
            .filter(|s| s.auto_start)
            .map(|s| s.name.clone())
            .collect();

        for name in auto_start_services {
            // Start in dependency order
            if let Err(e) = self.start_service_recursive(&name, spawn_pid_fn) {
                eprintln!("Failed to auto-start {}: {}", name, e);
            }
        }
    }

    fn start_service_recursive(
        &mut self,
        name: &str,
        spawn_pid_fn: &mut dyn FnMut(&str) -> u32,
    ) -> Result<(), String> {
        if let Some(service) = self.services.get(name) {
            if service.state == ServiceState::Running {
                return Ok(());
            }

            // Start dependencies first
            let deps = service.dependencies.clone();
            for dep in deps {
                self.start_service_recursive(&dep, spawn_pid_fn)?;
            }
        }

        let pid = spawn_pid_fn(name);
        self.start(name, pid)
    }
}
