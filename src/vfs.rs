use std::collections::HashMap;

#[derive(Clone)]
pub struct Inode {
    pub name: String,
    pub is_dir: bool,
    pub data: String,
    pub children: HashMap<String, Inode>,
}
impl Inode {
    pub fn dir(name: &str) -> Self {
        Inode {
            name: name.into(),
            is_dir: true,
            data: String::new(),
            children: HashMap::new(),
        }
    }
    pub fn file(name: &str, data: &str) -> Self {
        Inode {
            name: name.into(),
            is_dir: false,
            data: data.into(),
            children: HashMap::new(),
        }
    }
}

#[derive(Clone)]
pub struct VfsHandle {
    pub path: String,
    pub offset: usize,
    pub writable: bool,
}

pub struct Vfs {
    root: Inode,
    pub cwd: String,
    handles: HashMap<u32, VfsHandle>,
    next_handle: u32,
}
impl Vfs {
    pub fn new() -> Self {
        Vfs {
            root: Inode::dir("/"),
            cwd: "/".into(),
            handles: HashMap::new(),
            next_handle: 1,
        }
    }
    pub fn init(&mut self) {
        for d in ["bin", "dev", "etc", "home", "proc", "tmp", "var"].iter() {
            self.root.children.insert((*d).into(), Inode::dir(d));
        }
        if let Some(etc) = self.root.children.get_mut("etc") {
            etc.children
                .insert("hostname".into(), Inode::file("hostname", "kpawnd"));
            etc.children.insert(
                "github".into(),
                Inode::file("github", "https://www.github.com/kpawnd"),
            );
        }
        if let Some(home) = self.root.children.get_mut("home") {
            let mut user = Inode::dir("user");
            user.children.insert(
                "readme.txt".into(),
                Inode::file("readme.txt", "Echo github to open profile."),
            );
            home.children.insert("user".into(), user);
        }
    }
    pub fn normalize(&self, path: &str) -> String {
        if path.starts_with('/') {
            path.into()
        } else {
            let base = self.cwd.trim_end_matches('/');
            format!("{}/{}", base, path)
        }
    }
    pub fn resolve(&self, path: &str) -> Option<&Inode> {
        let norm = self.normalize(path);
        let mut node = &self.root;
        for part in norm.split('/').filter(|s| !s.is_empty()) {
            node = node.children.get(part)?;
        }
        Some(node)
    }
    pub fn resolve_mut(&mut self, path: &str) -> Option<&mut Inode> {
        let norm = self.normalize(path);
        let mut node = &mut self.root;
        for part in norm.split('/').filter(|s| !s.is_empty()) {
            node = node.children.get_mut(part)?;
        }
        Some(node)
    }
    pub fn cd(&mut self, path: &str) -> Result<(), &'static str> {
        let target = if path == ".." {
            let mut parts: Vec<_> = self.cwd.split('/').filter(|s| !s.is_empty()).collect();
            parts.pop();
            if parts.is_empty() {
                "/".into()
            } else {
                format!("/{}", parts.join("/"))
            }
        } else {
            self.normalize(path)
        };
        match self.resolve(&target) {
            Some(node) if node.is_dir => {
                self.cwd = if target == "/" {
                    "/".into()
                } else {
                    target.trim_end_matches('/').into()
                };
                Ok(())
            }
            Some(_) => Err("not a directory"),
            None => Err("no such directory"),
        }
    }
    pub fn open(&mut self, path: &str, write: bool) -> Result<u32, &'static str> {
        if let Some(node) = self.resolve(path) {
            if node.is_dir {
                return Err("is directory");
            }
            let h = self.next_handle;
            self.next_handle += 1;
            self.handles.insert(
                h,
                VfsHandle {
                    path: self.normalize(path),
                    offset: 0,
                    writable: write,
                },
            );
            Ok(h)
        } else {
            Err("no such file")
        }
    }
    pub fn read(&mut self, handle: u32, size: usize) -> Result<String, &'static str> {
        let (path, offset) = {
            let h = self.handles.get(&handle).ok_or("bad handle")?;
            (h.path.clone(), h.offset)
        };
        let inode = self.resolve(&path).ok_or("gone")?;
        let start = offset;
        let end = (start + size).min(inode.data.len());
        let out = inode.data[start..end].to_string();
        if let Some(h) = self.handles.get_mut(&handle) {
            h.offset = end;
        }
        Ok(out)
    }
    pub fn write(&mut self, handle: u32, data: &str) -> Result<(), &'static str> {
        let (path, writable) = {
            let h = self.handles.get(&handle).ok_or("bad handle")?;
            (h.path.clone(), h.writable)
        };
        if !writable {
            return Err("not writable");
        }
        let new_len = if let Some(inode) = self.resolve_mut(&path) {
            inode.data.push_str(data);
            inode.data.len()
        } else {
            return Err("gone");
        };
        if let Some(h) = self.handles.get_mut(&handle) {
            h.offset = new_len;
        }
        Ok(())
    }
    pub fn close(&mut self, handle: u32) {
        self.handles.remove(&handle);
    }
}
