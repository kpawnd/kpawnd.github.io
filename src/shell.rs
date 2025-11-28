use crate::kernel::Kernel;
use std::collections::{BTreeMap, HashMap};

pub enum ProgramKind {
    BuiltIn,
}
pub struct ProgramRegistry {
    progs: BTreeMap<String, ProgramKind>,
}
impl Default for ProgramRegistry {
    fn default() -> Self {
        Self::new()
    }
}
impl ProgramRegistry {
    pub fn new() -> Self {
        let mut r = ProgramRegistry {
            progs: BTreeMap::new(),
        };
        for name in [
            "echo", "cat", "ls", "pwd", "uname", "hostname", "id", "whoami", "free",
        ]
        .iter()
        {
            r.progs.insert((*name).into(), ProgramKind::BuiltIn);
        }
        r
    }
    pub fn has(&self, name: &str) -> bool {
        self.progs.contains_key(name)
    }
}

pub struct Shell {
    pub history: Vec<String>,
    pub env: HashMap<String, String>,
    pub registry: ProgramRegistry,
}
impl Default for Shell {
    fn default() -> Self {
        Self::new()
    }
}
impl Shell {
    pub fn new() -> Self {
        let mut env = HashMap::new();
        env.insert("HOME".into(), "/home/user".into());
        env.insert("PATH".into(), "/bin".into());
        env.insert("USER".into(), "user".into());
        env.insert("GITHUB".into(), "https://github.com/kpawnd".into());
        Shell {
            history: Vec::new(),
            env,
            registry: ProgramRegistry::new(),
        }
    }
}

pub fn prompt(kernel: &Kernel) -> String {
    let cwd = &kernel.fs.cwd;
    let display = if cwd == "/home/user" {
        "~"
    } else if let Some(rest) = cwd.strip_prefix("/home/user/") {
        &format!("~/{}", rest)
    } else {
        cwd
    };
    format!("user@kpawnd:{}$ ", display)
}
