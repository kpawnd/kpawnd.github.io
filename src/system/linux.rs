use super::System;

#[derive(Clone)]
struct UserEntry {
    name: String,
    uid: u32,
    gid: u32,
}

#[derive(Clone)]
struct GroupEntry {
    name: String,
    gid: u32,
    members: Vec<String>,
}

impl System {
    fn parse_users(&self) -> Vec<UserEntry> {
        let mut out = Vec::new();
        let Some(node) = self.kernel.fs.resolve("/etc/passwd") else {
            return out;
        };

        for line in node.data.lines() {
            let parts: Vec<&str> = line.split(':').collect();
            if parts.len() < 4 {
                continue;
            }
            let Ok(uid) = parts[2].parse::<u32>() else {
                continue;
            };
            let Ok(gid) = parts[3].parse::<u32>() else {
                continue;
            };
            out.push(UserEntry {
                name: parts[0].to_string(),
                uid,
                gid,
            });
        }

        out
    }

    fn parse_groups(&self) -> Vec<GroupEntry> {
        let mut out = Vec::new();
        let Some(node) = self.kernel.fs.resolve("/etc/group") else {
            return out;
        };

        for line in node.data.lines() {
            let parts: Vec<&str> = line.split(':').collect();
            if parts.len() < 4 {
                continue;
            }
            let Ok(gid) = parts[2].parse::<u32>() else {
                continue;
            };
            let members = if parts[3].trim().is_empty() {
                Vec::new()
            } else {
                parts[3]
                    .split(',')
                    .map(|v| v.trim().to_string())
                    .filter(|v| !v.is_empty())
                    .collect()
            };
            out.push(GroupEntry {
                name: parts[0].to_string(),
                gid,
                members,
            });
        }

        out
    }

    fn lookup_user<'a>(&self, users: &'a [UserEntry], name: &str) -> Option<&'a UserEntry> {
        users.iter().find(|u| u.name == name)
    }

    fn lookup_group_by_gid<'a>(
        &self,
        groups: &'a [GroupEntry],
        gid: u32,
    ) -> Option<&'a GroupEntry> {
        groups.iter().find(|g| g.gid == gid)
    }

    fn lookup_group_by_name<'a>(
        &self,
        groups: &'a [GroupEntry],
        name: &str,
    ) -> Option<&'a GroupEntry> {
        groups.iter().find(|g| g.name == name)
    }

    fn groups_for_user(&self, user: &UserEntry, groups: &[GroupEntry]) -> Vec<GroupEntry> {
        let mut out: Vec<GroupEntry> = Vec::new();

        if let Some(primary) = self.lookup_group_by_gid(groups, user.gid) {
            out.push(primary.clone());
        }

        for g in groups {
            if g.members.iter().any(|m| m == &user.name) && !out.iter().any(|e| e.gid == g.gid) {
                out.push(g.clone());
            }
        }

        out.sort_by_key(|g| g.gid);
        out
    }

    fn parse_mode_oct(mode: &str) -> Option<[u8; 3]> {
        let trimmed = if mode.len() == 4 && mode.starts_with('0') {
            &mode[1..]
        } else {
            mode
        };

        if trimmed.len() != 3 || !trimmed.chars().all(|c| ('0'..='7').contains(&c)) {
            return None;
        }

        let bytes = trimmed.as_bytes();
        Some([(bytes[0] - b'0'), (bytes[1] - b'0'), (bytes[2] - b'0')])
    }

    fn bits_to_triplet(bits: u8) -> [char; 3] {
        [
            if bits & 4 != 0 { 'r' } else { '-' },
            if bits & 2 != 0 { 'w' } else { '-' },
            if bits & 1 != 0 { 'x' } else { '-' },
        ]
    }

    fn triplet_to_bits(chars: &[char]) -> u8 {
        let mut bits = 0u8;
        if chars.first() == Some(&'r') {
            bits |= 4;
        }
        if chars.get(1) == Some(&'w') {
            bits |= 2;
        }
        if chars.get(2) == Some(&'x') {
            bits |= 1;
        }
        bits
    }

    fn mode_to_octal(mode: &str) -> String {
        let chars: Vec<char> = mode.chars().collect();
        if chars.len() < 10 {
            return "000".into();
        }
        let u = Self::triplet_to_bits(&chars[1..4]);
        let g = Self::triplet_to_bits(&chars[4..7]);
        let o = Self::triplet_to_bits(&chars[7..10]);
        format!("{}{}{}", u, g, o)
    }

    fn apply_symbolic_mode(mode: &str, perm: &str) -> Option<String> {
        let mut chars: Vec<char> = perm.chars().collect();
        if chars.len() < 10 {
            return None;
        }

        let op_pos = mode.find(['+', '-', '='])?;
        let (who, rest) = mode.split_at(op_pos);
        let mut rest_chars = rest.chars();
        let op = rest_chars.next()?;
        let rights: Vec<char> = rest_chars.collect();
        if rights.is_empty() || rights.iter().any(|c| !matches!(c, 'r' | 'w' | 'x')) {
            return None;
        }

        let mut classes: Vec<char> = if who.is_empty() {
            vec!['u', 'g', 'o']
        } else {
            who.chars().collect()
        };
        classes.sort_unstable();
        classes.dedup();
        if classes.iter().any(|c| !matches!(c, 'u' | 'g' | 'o' | 'a')) {
            return None;
        }
        if classes.contains(&'a') {
            classes = vec!['u', 'g', 'o'];
        }

        for class in classes {
            let base = match class {
                'u' => 1,
                'g' => 4,
                'o' => 7,
                _ => continue,
            };

            let mut bits = Self::triplet_to_bits(&chars[base..base + 3]);
            let mut delta = 0u8;
            for r in &rights {
                match *r {
                    'r' => delta |= 4,
                    'w' => delta |= 2,
                    'x' => delta |= 1,
                    _ => {}
                }
            }

            bits = match op {
                '+' => bits | delta,
                '-' => bits & !delta,
                '=' => delta,
                _ => return None,
            };

            let tri = Self::bits_to_triplet(bits);
            chars[base] = tri[0];
            chars[base + 1] = tri[1];
            chars[base + 2] = tri[2];
        }

        Some(chars.into_iter().collect())
    }

    pub(super) fn cmd_id(&self, args: &[&str]) -> String {
        let users = self.parse_users();
        let groups = self.parse_groups();

        let mut username = self.current_user();
        let mut flag = "";

        for arg in args {
            if arg.starts_with('-') {
                flag = arg;
            } else {
                username = (*arg).to_string();
            }
        }

        let Some(user) = self.lookup_user(&users, &username) else {
            return format!("id: '{}' no such user", username);
        };

        let primary_group = self
            .lookup_group_by_gid(&groups, user.gid)
            .map(|g| g.name.clone())
            .unwrap_or_else(|| user.name.clone());
        let group_list = self.groups_for_user(user, &groups);

        match flag {
            "-u" => format!("{}", user.uid),
            "-g" => format!("{}", user.gid),
            "-un" => user.name.clone(),
            "-gn" => primary_group,
            "" => {
                let rendered = group_list
                    .iter()
                    .map(|g| format!("{}({})", g.gid, g.name))
                    .collect::<Vec<_>>()
                    .join(",");
                format!(
                    "uid={}({}) gid={}({}) groups={}",
                    user.uid, user.name, user.gid, primary_group, rendered
                )
            }
            _ => "usage: id [-u|-g|-un|-gn] [USER]".into(),
        }
    }

    pub(super) fn cmd_groups(&self, args: &[&str]) -> String {
        let users = self.parse_users();
        let groups = self.parse_groups();
        let username = if args.is_empty() {
            self.current_user()
        } else {
            args[0].to_string()
        };

        let Some(user) = self.lookup_user(&users, &username) else {
            return format!("groups: '{}' no such user", username);
        };

        let names = self
            .groups_for_user(user, &groups)
            .into_iter()
            .map(|g| g.name)
            .collect::<Vec<_>>()
            .join(" ");

        format!("{} : {}", user.name, names)
    }

    pub(super) fn cmd_who(&self, _args: &[&str]) -> String {
        let d = js_sys::Date::new_0();
        let user = self.current_user();
        let host = self
            .kernel
            .fs
            .resolve("/etc/hostname")
            .map(|n| n.data.trim().to_string())
            .unwrap_or_else(|| "localhost".into());

        format!(
            "{} tty1 {} ({})",
            user,
            d.to_locale_string("en-GB", &wasm_bindgen::JsValue::NULL),
            host
        )
    }

    pub(super) fn cmd_stat(&self, args: &[&str]) -> String {
        if args.is_empty() {
            return "usage: stat FILE".into();
        }

        let path = args[0];
        let Some(node) = self.kernel.fs.resolve(path) else {
            return format!("stat: cannot stat '{}': No such file or directory", path);
        };

        let file_type = if node.permissions.starts_with('l') {
            "symbolic link"
        } else if node.is_dir {
            "directory"
        } else {
            "regular file"
        };
        let blocks = node.size.div_ceil(512);
        let inode_like = (node.name.len() as u64) * 131 + (node.size as u64);

        format!(
            "  File: {}\n  Size: {}\tBlocks: {}\tIO Block: 4096\t{}\nDevice: 00:00\tInode: {}\tLinks: 1\nAccess: ({}/{})\tUid: ({}/{})\tGid: ({}/{})",
            path,
            node.size,
            blocks,
            file_type,
            inode_like,
            Self::mode_to_octal(&node.permissions),
            node.permissions,
            node.owner,
            node.owner,
            node.group,
            node.group
        )
    }

    pub(super) fn cmd_mount(&mut self, args: &[&str]) -> String {
        if args.is_empty() {
            return self
                .kernel
                .fs
                .resolve("/proc/mounts")
                .map(|n| n.data.clone())
                .unwrap_or_else(|| "/dev/sda1 / ext4 rw,relatime 0 0".into());
        }

        let mut fs_type = "ext4".to_string();
        let mut values: Vec<&str> = Vec::new();
        let mut i = 0;
        while i < args.len() {
            match args[i] {
                "-t" => {
                    if i + 1 >= args.len() {
                        return "mount: option requires an argument -- 't'".into();
                    }
                    fs_type = args[i + 1].to_string();
                    i += 2;
                }
                flag if flag.starts_with('-') => {
                    return format!("mount: unsupported option '{}'", flag);
                }
                val => {
                    values.push(val);
                    i += 1;
                }
            }
        }

        if values.len() != 2 {
            return "usage: mount [-t TYPE] SOURCE TARGET".into();
        }

        let source = values[0];
        let target = values[1];

        match self.kernel.fs.resolve(target) {
            Some(n) if !n.is_dir => {
                return format!("mount: {}: mount point is not a directory", target)
            }
            Some(_) => {}
            None => return format!("mount: {}: mount point does not exist", target),
        }

        let existing = self
            .kernel
            .fs
            .resolve("/proc/mounts")
            .map(|n| n.data.clone())
            .unwrap_or_default();

        if existing.lines().any(|line| {
            let cols: Vec<&str> = line.split_whitespace().collect();
            cols.len() >= 2 && cols[1] == target
        }) {
            return format!("mount: {}: already mounted", target);
        }

        let mut next = existing;
        if !next.is_empty() && !next.ends_with('\n') {
            next.push('\n');
        }
        next.push_str(&format!(
            "{} {} {} rw,relatime 0 0\n",
            source, target, fs_type
        ));

        let _ = self.kernel.fs.write_file("/proc/mounts", &next);
        format!("mounted {} on {} type {}", source, target, fs_type)
    }

    pub(super) fn cmd_umount(&mut self, args: &[&str]) -> String {
        if args.len() != 1 {
            return "usage: umount TARGET".into();
        }

        let target = args[0];
        let current = self
            .kernel
            .fs
            .resolve("/proc/mounts")
            .map(|n| n.data.clone())
            .unwrap_or_default();

        let kept: Vec<&str> = current
            .lines()
            .filter(|line| {
                let cols: Vec<&str> = line.split_whitespace().collect();
                !(cols.len() >= 2 && cols[1] == target)
            })
            .collect();

        if kept.len() == current.lines().count() {
            return format!("umount: {}: not mounted", target);
        }

        let mut next = kept.join("\n");
        if !next.is_empty() {
            next.push('\n');
        }
        let _ = self.kernel.fs.write_file("/proc/mounts", &next);
        format!("unmounted {}", target)
    }

    pub(super) fn cmd_chmod(&mut self, args: &[&str]) -> String {
        if args.len() < 2 {
            return "usage: chmod MODE FILE...".into();
        }

        let mode = args[0];
        let targets = &args[1..];
        let mut errors = Vec::new();

        for path in targets {
            let Some(node) = self.kernel.fs.resolve(path) else {
                errors.push(format!(
                    "chmod: cannot access '{}': No such file or directory",
                    path
                ));
                continue;
            };

            let mut next_perm = node.permissions.clone();
            if let Some(oct) = Self::parse_mode_oct(mode) {
                let mut chars: Vec<char> = next_perm.chars().collect();
                if chars.len() < 10 {
                    errors.push(format!(
                        "chmod: failed to update '{}': invalid mode string",
                        path
                    ));
                    continue;
                }
                let u = Self::bits_to_triplet(oct[0]);
                let g = Self::bits_to_triplet(oct[1]);
                let o = Self::bits_to_triplet(oct[2]);
                chars[1] = u[0];
                chars[2] = u[1];
                chars[3] = u[2];
                chars[4] = g[0];
                chars[5] = g[1];
                chars[6] = g[2];
                chars[7] = o[0];
                chars[8] = o[1];
                chars[9] = o[2];
                next_perm = chars.into_iter().collect();
            } else if let Some(updated) = Self::apply_symbolic_mode(mode, &next_perm) {
                next_perm = updated;
            } else {
                return format!("chmod: invalid mode: '{}'", mode);
            }

            if let Some(node_mut) = self.kernel.fs.resolve_mut(path) {
                node_mut.permissions = next_perm;
            } else {
                errors.push(format!(
                    "chmod: failed to update '{}': path disappeared",
                    path
                ));
            }
        }

        if errors.is_empty() {
            String::new()
        } else {
            errors.join("\n")
        }
    }

    pub(super) fn cmd_chown(&mut self, args: &[&str]) -> String {
        if args.len() < 2 {
            return "usage: chown OWNER[:GROUP] FILE...".into();
        }

        let spec = args[0];
        let targets = &args[1..];
        let users = self.parse_users();
        let groups = self.parse_groups();

        let (owner_spec, group_spec) = if let Some((owner, group)) = spec.split_once(':') {
            (owner, Some(group))
        } else {
            (spec, None)
        };

        let resolved_owner = if owner_spec.is_empty() {
            None
        } else if self.lookup_user(&users, owner_spec).is_some() {
            Some(owner_spec.to_string())
        } else {
            return format!("chown: invalid user: '{}'", owner_spec);
        };

        let resolved_group = if let Some(g) = group_spec {
            if g.is_empty() {
                None
            } else if self.lookup_group_by_name(&groups, g).is_some() {
                Some(g.to_string())
            } else {
                return format!("chown: invalid group: '{}'", g);
            }
        } else {
            None
        };

        let mut errors = Vec::new();
        for path in targets {
            let Some(existing) = self.kernel.fs.resolve(path) else {
                errors.push(format!(
                    "chown: cannot access '{}': No such file or directory",
                    path
                ));
                continue;
            };

            let next_owner = resolved_owner
                .clone()
                .unwrap_or_else(|| existing.owner.clone());
            let next_group = if let Some(g) = &resolved_group {
                g.clone()
            } else if resolved_owner.is_some() {
                next_owner.clone()
            } else {
                existing.group.clone()
            };

            if let Some(node_mut) = self.kernel.fs.resolve_mut(path) {
                node_mut.owner = next_owner;
                node_mut.group = next_group;
            } else {
                errors.push(format!(
                    "chown: failed to update '{}': path disappeared",
                    path
                ));
            }
        }

        if errors.is_empty() {
            String::new()
        } else {
            errors.join("\n")
        }
    }
}
