use std::collections::HashMap;
use std::error::Error;
use std::process;
use std::sync::Arc;

use async_trait::async_trait;
use milton::sys::u3m_pack;
use rustyline::error::ReadlineError;
use rustyline::history::FileHistory;
use rustyline::{DefaultEditor, Editor};
use shrine_core::Care;
use shrine_core::driver::{
    DriverAck, DriverCard, DriverID, DriverNote, IntoTale, ShrineDriver, Subscription,
};
use shrine_storage::core::path::{PathIdx, path_noun_from_string};
use shrine_storage::core::types::slot;
use shrine_storage::store::Namespace;
use shrine_storage::{DiskPail, Gift, Mode, Observe, Tale};
use tokio::sync::mpsc;
use tracing::{error, info};

#[derive(Debug, Clone)]
pub enum ShellPailType {
    Text, // ASCII text
    Hoon, // Hoon code (ASCII text)
    Mesh, // HashMap of path to path
    Duct, // List of paths
    Atom, // Bigint
}

#[derive(Debug, Clone)]
pub struct ShellPail {
    pub typ: ShellPailType,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct ShellTale {
    pub slots: HashMap<String, ShellPail>,
}

impl IntoTale for ShellTale {
    fn into_tale(&self, _namespace: Arc<Namespace>) -> Tale {
        let mut tale = Tale::empty();

        for (slot, shell_pail) in &self.slots {
            let disk_pail = match &shell_pail.typ {
                ShellPailType::Text => DiskPail::Text {
                    data: shell_pail.data.clone(),
                },
                ShellPailType::Hoon => DiskPail::Hoon {
                    data: shell_pail.data.clone(),
                },
                ShellPailType::Atom => DiskPail::Atom {
                    data: shell_pail.data.clone(),
                },
                ShellPailType::Mesh => DiskPail::Mesh {
                    data: DiskPail::mesh_from_bytes(&shell_pail.data),
                },
                ShellPailType::Duct => DiskPail::Duct {
                    data: shell_pail
                        .data
                        .chunks_exact(4)
                        .map(|c| PathIdx::new(u32::from_le_bytes(c.try_into().unwrap())))
                        .collect(),
                },
            };
            tale.insert(slot.clone(), disk_pail);
        }

        tale
    }
}

/// IntoTale for export requests — creates jammed pith on main thread.
#[derive(Debug)]
struct ExportRequestTale {
    path: String,
}

impl IntoTale for ExportRequestTale {
    fn into_tale(&self, _namespace: Arc<Namespace>) -> Tale {
        let mut tale = Tale::empty();
        let pith = path_noun_from_string(&self.path);
        let jammed = pith.jam();
        tale.insert(
            slot::REQ,
            DiskPail::Jam {
                typ: "pith".to_string(),
                data: jammed,
            },
        );
        tale
    }
}

pub struct ShellHelper {
    pub path: String,
    pub bindings: HashMap<String, String>, // Maps #alias -> /path/to/slots
}

impl ShellHelper {
    pub fn new() -> Self {
        Self {
            path: "/".to_string(),
            bindings: HashMap::new(),
        }
    }
}

pub struct ShellWorker {
    driver_id: DriverID,
    tx: mpsc::Sender<Vec<DriverCard>>,
    namespace: Arc<Namespace>,
    pub editor: Editor<(), FileHistory>,
    pub helper: ShellHelper,
}

impl ShellWorker {
    fn new(
        driver_id: DriverID,
        tx: mpsc::Sender<Vec<DriverCard>>,
        namespace: Arc<Namespace>,
    ) -> Self {
        Self {
            driver_id,
            editor: DefaultEditor::new().expect("failed to create default editor"),
            namespace,
            tx,
            helper: ShellHelper::new(),
        }
    }

    fn format_pail(pail: &DiskPail) -> String {
        match pail {
            DiskPail::Text { data } | DiskPail::Hoon { data } => match std::str::from_utf8(data) {
                Ok(s) => {
                    let tag = pail.typ_str();
                    if s.len() > 120 {
                        format!("[{}] {}...", tag, &s[..120])
                    } else {
                        format!("[{}] {}", tag, s)
                    }
                }
                Err(_) => format!("[{}] <{} bytes, invalid utf8>", pail.typ_str(), data.len()),
            },
            DiskPail::Wain { data } => {
                let max = data.len().min(10);
                data[0..max].join("\n")
            }
            DiskPail::Atom { data } => {
                if data.len() <= 8 {
                    // Show as decimal for small atoms
                    let mut val: u64 = 0;
                    for (i, &b) in data.iter().enumerate() {
                        val |= (b as u64) << (i * 8);
                    }
                    format!("[atom] {}", val)
                } else {
                    format!("[atom] <{} bytes>", data.len())
                }
            }
            DiskPail::Jam { typ, data } => {
                format!("[jam:{}] <{} bytes>", typ, data.len())
            }
            DiskPail::Jim { typ, data } => {
                format!("[jim:{}] <{} bytes>", typ, data.len())
            }
            DiskPail::Mesh { data } => {
                format!("[mesh] <{} bytes>", data.len())
            }
            DiskPail::Duct { data } => {
                format!("[duct] <{} paths>", data.len())
            }
        }
    }

    fn normalize_path(&self, path: &str) -> String {
        let raw = if path.starts_with('/') {
            path.to_string()
        } else {
            let base = if self.helper.path == "/" {
                "".to_string()
            } else {
                self.helper.path.clone()
            };
            format!("{}/{}", base, path)
        };

        // Resolve . and .. segments
        let mut parts: Vec<&str> = Vec::new();
        for seg in raw.split('/') {
            match seg {
                "" | "." => {}
                ".." => {
                    parts.pop();
                }
                s => parts.push(s),
            }
        }
        if parts.is_empty() {
            "/".to_string()
        } else {
            format!("/{}", parts.join("/"))
        }
    }

    fn resolve_slot_path(&self, slot: &str) -> String {
        // Check if slot uses #alias syntax
        if let Some(rest) = slot.strip_prefix('#') {
            // Split into alias and path
            if let Some((alias, path)) = rest.split_once('/') {
                let alias_key = format!("#{}", alias);
                if let Some(base_path) = self.helper.bindings.get(&alias_key) {
                    return format!("{}/{}", base_path, path);
                }
            }
        }
        // Return as-is if no binding or already a full path
        slot.to_string()
    }

    fn parse_slot_type(type_str: &str) -> ShellPailType {
        match type_str {
            "text" => ShellPailType::Text,
            "hoon" => ShellPailType::Hoon,
            "mesh" => ShellPailType::Mesh,
            "duct" => ShellPailType::Duct,
            "atom" => ShellPailType::Atom,
            _ => {
                println!("Warning: unknown type '{}', using 'text'", type_str);
                ShellPailType::Text
            }
        }
    }

    fn create_shell_tale(&self, slots: HashMap<String, (ShellPailType, Vec<u8>)>) -> ShellTale {
        let mut shell_pails = HashMap::new();
        for (key, (typ, value)) in slots {
            // Resolve slot path (handles #alias/path syntax)
            let resolved_key = self.resolve_slot_path(&key);

            let shell_pail = ShellPail { typ, data: value };
            shell_pails.insert(resolved_key, shell_pail);
        }
        ShellTale { slots: shell_pails }
    }

    fn get_children(&self, path: &str) -> Result<Vec<String>, Box<dyn Error + Send>> {
        let res = self.namespace.look(Care::Y, path).unwrap();
        let mut ret = vec![];
        match res {
            Observe::Found(map) => {
                for (k, _v) in map {
                    let Some(pax) = self.namespace.path_idx_to_str(&k).unwrap() else {
                        continue;
                    };
                    ret.push(pax);
                }
            }
            Observe::Null => {}
            Observe::Unknown => {}
        };
        Ok(ret)
    }

    fn get_subtree(&self, path: &str) -> Result<Vec<String>, Box<dyn Error + Send>> {
        let mut ret = vec![];
        match self.namespace.look(Care::Z, path).unwrap() {
            Observe::Found(e) => {
                for (k, _v) in &e {
                    let Some(name) = self.namespace.path_idx_to_str(k).unwrap() else {
                        error!("weird idx {k:?}");
                        continue;
                    };
                    ret.push(name);
                }
                Ok(ret)
            }
            r => {
                info!("missing subtree {r:?}");
                Ok(ret)
            }
        }
    }

    fn list_tree(&self, path: &str, prefix: &str) -> Result<(), Box<dyn Error + Send>> {
        println!("pax {path:?} {prefix:?}");
        let children = self.get_subtree(path)?;
        println!("children: {children:?}");

        for (i, child) in children.iter().enumerate() {
            let is_last = i == children.len() - 1;
            let branch = if is_last { "└── " } else { "├── " };
            println!("{}{}{}", prefix, branch, child);

            let _child_path = if path == "/" {
                format!("{}", child)
            } else {
                format!("{}/{}", path, child)
            };

            let _child_prefix = if is_last {
                format!("{}    ", prefix)
            } else {
                format!("{}│   ", prefix)
            };

            // if let Ok(_) = self.list_tree(&child_path, &child_prefix) {
            //     // Continue
            // }
        }
        Ok(())
    }

    async fn handle_command(&mut self, line: &str) -> Result<(), Box<dyn Error + Send>> {
        let parts: Vec<&str> = line.trim().split_whitespace().collect();
        if parts.is_empty() {
            return Ok(());
        }

        match parts[0] {
            "pwd" => {
                println!("{}", self.helper.path);
            }
            "cd" => {
                if parts.len() < 2 {
                    self.helper.path = "/".to_string();
                } else {
                    let new_path = self.normalize_path(parts[1]);
                    // Verify path exists
                    match self.namespace.get_path_id(&new_path) {
                        Ok(_) => {
                            self.helper.path = new_path;
                        }
                        Err(_e) => {
                            println!("Error: path not found: {}", new_path);
                        }
                    }
                }
            }
            "ls" => {
                let path = if parts.len() > 1 {
                    self.normalize_path(parts[1])
                } else {
                    self.helper.path.clone()
                };

                match self.get_children(&path) {
                    Ok(children) => {
                        if children.is_empty() {
                            println!("(empty)");
                        } else {
                            for child in children {
                                println!("{}", child);
                            }
                        }
                    }
                    Err(e) => {
                        println!("Error listing {}: {}", path, e);
                    }
                }
            }
            "tree" => {
                let path = if parts.len() > 1 {
                    self.normalize_path(parts[1])
                } else {
                    self.helper.path.clone()
                };

                println!("{}", path);
                if let Err(e) = self.list_tree(&path, "") {
                    println!("Error: {}", e);
                }
            }
            "cat" => {
                if parts.len() < 2 {
                    println!("Usage: cat <path>");
                    return Ok(());
                }
                let path = self.normalize_path(parts[1]);

                // NOTE: we cannot call disk_saga.to_saga() here because that
                // creates Nouns via the C runtime, which is thread-local and
                // must run on the main thread.  Display DiskSaga info directly.
                match self.namespace.grab(&path) {
                    Ok(observe) => match observe {
                        Some(saga) => {
                            println!("\nSaga at {}:", path);
                            println!("  Ever: {:?}", saga.ever());
                            println!("  Tale:");
                            for (slot, pail) in saga.slots().iter().zip(saga.pails().iter()) {
                                let display = Self::format_pail(pail);
                                println!("    {}: {}", slot, display);
                            }
                        }
                        None => {
                            println!("Path not found");
                        }
                    },
                    Err(e) => {
                        println!("Error reading {}: {}", path, e);
                    }
                }
            }
            "pack" => {
                unsafe {
                    let mem_w = u3m_pack();
                    eprintln!("Packed: {mem_w:?}");
                }
            }
            "edit" => {
                if parts.len() < 2 {
                    println!("Usage: edit <path> [slot]");
                    println!("  Opens slot content in $EDITOR (default: vim)");
                    println!("  slot defaults to /sys/slot/content");
                    return Ok(());
                }
                let path = self.normalize_path(parts[1]);
                let slot = if parts.len() > 2 {
                    self.resolve_slot_path(parts[2])
                } else {
                    "/sys/slot/content".to_string()
                };

                // Read current content and extension
                let (existing, file_ext) = match self.namespace.grab(&path) {
                    Ok(Some(saga)) => {
                        let tale = saga.tale();
                        let content = tale.get(&slot).map(|p| p.data().to_vec());
                        let ext = tale
                            .get("/sys/slot/extension")
                            .and_then(|p| std::str::from_utf8(p.data()).ok())
                            .unwrap_or("")
                            .to_string();
                        (content, ext)
                    }
                    Ok(None) => (None, String::new()),
                    Err(e) => {
                        println!("Error reading {}: {}", path, e);
                        return Ok(());
                    }
                };
                let original = existing.unwrap_or_default();

                // Write to temp file with correct extension
                let temp_name = if file_ext.is_empty() {
                    format!("shrine-edit-{}", std::process::id())
                } else {
                    format!("shrine-edit-{}.{}", std::process::id(), file_ext)
                };
                let temp_path = std::env::temp_dir().join(temp_name);
                if let Err(e) = std::fs::write(&temp_path, &original) {
                    println!("Error writing temp file: {}", e);
                    return Ok(());
                }

                // Spawn editor
                let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vim".to_string());
                let status = tokio::process::Command::new(&editor)
                    .arg(&temp_path)
                    .status()
                    .await;

                match status {
                    Ok(s) if s.success() => {}
                    Ok(s) => {
                        println!("Editor exited with: {}", s);
                        let _ = std::fs::remove_file(&temp_path);
                        return Ok(());
                    }
                    Err(e) => {
                        println!("Failed to launch editor '{}': {}", editor, e);
                        let _ = std::fs::remove_file(&temp_path);
                        return Ok(());
                    }
                }

                // Read back
                let updated = match std::fs::read(&temp_path) {
                    Ok(data) => data,
                    Err(e) => {
                        println!("Error reading temp file: {}", e);
                        let _ = std::fs::remove_file(&temp_path);
                        return Ok(());
                    }
                };
                let _ = std::fs::remove_file(&temp_path);

                if updated == original {
                    println!("No changes");
                    return Ok(());
                }
                println!("pax {path:?}");

                // Poke update
                let mut slots = HashMap::new();
                slots.insert(slot.clone(), (ShellPailType::Text, updated));
                let shell_tale = self.create_shell_tale(slots);

                // Use make if path didn't exist, poke if it did
                let note =
                    if original.is_empty() && self.namespace.grab(&path).ok().flatten().is_none() {
                        DriverNote::make(&path, shell_tale)
                    } else {
                        DriverNote::poke(&path, shell_tale)
                    };
                let card = DriverCard {
                    id: self.driver_id,
                    note,
                };

                match self.tx.send(vec![card]).await {
                    Ok(_) => {
                        println!("Updated {} [{}]", path, slot);
                    }
                    Err(e) => {
                        println!("Error sending update: {}", e);
                    }
                }
            }
            "touch" => {
                if parts.len() < 2 {
                    println!("Usage: touch <path> [slot[:type]=value ...]");
                    println!("  Slots can be: /full/path or #alias/path");
                    println!("  Types: text (default), hoon, mesh, duct, atom");
                    println!("  Example: touch /foo /content:text=hello /code:hoon='|= a'");
                    return Ok(());
                }
                let path = self.normalize_path(parts[1]);

                // Parse slot[:type]=value pairs
                let mut slots = HashMap::new();
                for i in 2..parts.len() {
                    if let Some((slot_part, value)) = parts[i].split_once('=') {
                        // Parse slot and optional type
                        let (slot, typ) = if let Some((s, t)) = slot_part.split_once(':') {
                            (s, Self::parse_slot_type(t))
                        } else {
                            (slot_part, ShellPailType::Text)
                        };

                        // Validate slot format
                        if !slot.starts_with('/') && !slot.starts_with('#') {
                            println!(
                                "Warning: slot '{}' should start with / or #, using as-is",
                                slot
                            );
                        }
                        slots.insert(slot.to_string(), (typ, value.as_bytes().to_vec()));
                    }
                }

                let shell_tale = self.create_shell_tale(slots);
                let note = DriverNote::make(&path, shell_tale);
                let card = DriverCard {
                    id: self.driver_id,
                    note,
                };

                match self.tx.send(vec![card]).await {
                    Ok(_) => {
                        println!("Created: {}", path);
                    }
                    Err(e) => {
                        println!("Error sending create request: {}", e);
                    }
                }
            }
            "rm" => {
                if parts.len() < 2 {
                    println!("Usage: rm <path>");
                    return Ok(());
                }
                let path = self.normalize_path(parts[1]);

                let note = DriverNote {
                    path: path.clone(),
                    mode: Mode::Del,
                    slots: Arc::new(ShellTale {
                        slots: HashMap::new(),
                    }),
                };
                let card = DriverCard {
                    id: self.driver_id,
                    note,
                };

                match self.tx.send(vec![card]).await {
                    Ok(_) => {
                        println!("Deleted: {}", path);
                    }
                    Err(e) => {
                        println!("Error sending delete request: {}", e);
                    }
                }
            }
            "update" => {
                if parts.len() < 3 {
                    println!("Usage: update <path> slot[:type]=value [slot2[:type2]=value2 ...]");
                    println!("  Slots can be: /full/path or #alias/path");
                    println!("  Types: text (default), hoon, mesh, duct, atom");
                    return Ok(());
                }
                let path = self.normalize_path(parts[1]);

                // Parse slot[:type]=value pairs
                let mut slots = HashMap::new();
                for i in 2..parts.len() {
                    if let Some((slot_part, value)) = parts[i].split_once('=') {
                        // Parse slot and optional type
                        let (slot, typ) = if let Some((s, t)) = slot_part.split_once(':') {
                            (s, Self::parse_slot_type(t))
                        } else {
                            (slot_part, ShellPailType::Text)
                        };

                        // Validate slot format
                        if !slot.starts_with('/') && !slot.starts_with('#') {
                            println!(
                                "Warning: slot '{}' should start with / or #, using as-is",
                                slot
                            );
                        }
                        slots.insert(slot.to_string(), (typ, value.as_bytes().to_vec()));
                    }
                }

                if slots.is_empty() {
                    println!("No slots provided. Usage: update <path> slot[:type]=value");
                    return Ok(());
                }

                let shell_tale = self.create_shell_tale(slots);
                let note = DriverNote::poke(&path, shell_tale);
                let card = DriverCard {
                    id: self.driver_id,
                    note,
                };

                match self.tx.send(vec![card]).await {
                    Ok(_) => {
                        println!("Updated: {}", path);
                    }
                    Err(e) => {
                        println!("Error sending update request: {}", e);
                    }
                }
            }
            "bind" => {
                if parts.len() < 3 {
                    println!("Usage: bind #alias /path/to/slots");
                    return Ok(());
                }
                let alias = parts[1];
                let path = parts[2];

                if !alias.starts_with('#') {
                    println!("Error: alias must start with #");
                    return Ok(());
                }

                if !path.starts_with('/') {
                    println!("Error: path must start with /");
                    return Ok(());
                }

                self.helper
                    .bindings
                    .insert(alias.to_string(), path.to_string());
                println!("Bound {} to {}", alias, path);
            }
            "unbind" => {
                if parts.len() < 2 {
                    println!("Usage: unbind #alias");
                    return Ok(());
                }
                let alias = parts[1];

                if !alias.starts_with('#') {
                    println!("Error: alias must start with #");
                    return Ok(());
                }

                if self.helper.bindings.remove(alias).is_some() {
                    println!("Unbound {}", alias);
                } else {
                    println!("No binding found for {}", alias);
                }
            }
            "bindings" => {
                if self.helper.bindings.is_empty() {
                    println!("No bindings configured");
                } else {
                    println!("Current bindings:");
                    let mut bindings: Vec<_> = self.helper.bindings.iter().collect();
                    bindings.sort_by_key(|(k, _)| *k);
                    for (alias, path) in bindings {
                        println!("  {} -> {}", alias, path);
                    }
                }
            }
            "mount" => {
                if parts.len() < 5 {
                    println!(
                        "Usage: mount <name> <unix-path> <shrine-path> <extension> [projection-slot]"
                    );
                    println!("  Creates a filesync mount at /drivers/clay/<name>");
                    println!("  extension: file extension for unix files (e.g. hoon, txt)");
                    println!("  projection-slot defaults to /sys/slot/content");
                    println!();
                    println!("  Example: mount code ~/code /src hoon");
                    println!("  Example: mount docs ~/docs /docs txt /sys/slot/content");
                    return Ok(());
                }

                let name = parts[1];
                let unix_path = if parts[2].starts_with("~/") {
                    let home = dirs::home_dir().expect("no home dir");
                    home.join(&parts[2][2..]).to_string_lossy().to_string()
                } else {
                    parts[2].to_string()
                };
                let shrine_path = parts[3];
                let extension = parts[4];
                let projection = if parts.len() > 5 {
                    parts[5]
                } else {
                    "/sys/slot/content"
                };

                let clay_path = format!("/drivers/clay/{}", name);

                let mut slots = HashMap::new();
                slots.insert(
                    "/sys/slot/unix".to_string(),
                    (ShellPailType::Text, unix_path.as_bytes().to_vec()),
                );
                slots.insert(
                    "/sys/slot/shrine".to_string(),
                    (ShellPailType::Text, shrine_path.as_bytes().to_vec()),
                );
                slots.insert(
                    "/sys/slot/projection".to_string(),
                    (ShellPailType::Text, projection.as_bytes().to_vec()),
                );
                slots.insert(
                    "/sys/slot/extension".to_string(),
                    (ShellPailType::Text, extension.as_bytes().to_vec()),
                );

                let shell_tale = self.create_shell_tale(slots);
                let note = DriverNote::make(&clay_path, shell_tale);
                let card = DriverCard {
                    id: self.driver_id,
                    note,
                };

                match self.tx.send(vec![card]).await {
                    Ok(_) => {
                        println!(
                            "Mounted '{}': {} <-> {} [{}] .{}",
                            name, unix_path, shrine_path, projection, extension
                        );
                    }
                    Err(e) => {
                        println!("Error creating mount: {}", e);
                    }
                }
            }
            "mount-src" => {
                if parts.len() < 4 {
                    println!("Usage: mount-src <name> <unix-path> <shrine-path>");
                    println!(
                        "  Like mount, but hardcodes projection=/sys/slot/src, extension=hoon,"
                    );
                    println!(
                        "  and adds /sys/slot/kook pointing to /std/kok/boot on each synced file."
                    );
                    println!();
                    println!("  Example: mount-src base ~/base /src");
                    return Ok(());
                }

                let name = parts[1];
                let unix_path = if parts[2].starts_with("~/") {
                    let home = dirs::home_dir().expect("no home dir");
                    home.join(&parts[2][2..]).to_string_lossy().to_string()
                } else {
                    parts[2].to_string()
                };
                let shrine_path = parts[3];

                let clay_path = format!("/drivers/clay/{}", name);

                let mut slots = HashMap::new();
                slots.insert(
                    "/sys/slot/unix".to_string(),
                    (ShellPailType::Text, unix_path.as_bytes().to_vec()),
                );
                slots.insert(
                    "/sys/slot/shrine".to_string(),
                    (ShellPailType::Text, shrine_path.as_bytes().to_vec()),
                );
                slots.insert(
                    "/sys/slot/projection".to_string(),
                    (ShellPailType::Text, "/sys/slot/src".as_bytes().to_vec()),
                );
                slots.insert(
                    "/sys/slot/extension".to_string(),
                    (ShellPailType::Text, "hoon".as_bytes().to_vec()),
                );
                // slots.insert(
                //     "/sys/slot/kook".to_string(),
                //     (ShellPailType::Text, "/std/kok/boot".as_bytes().to_vec()),
                // );

                let shell_tale = self.create_shell_tale(slots);
                let note = DriverNote::make(&clay_path, shell_tale);
                let card = DriverCard {
                    id: self.driver_id,
                    note,
                };

                match self.tx.send(vec![card]).await {
                    Ok(_) => {
                        println!(
                            "Mounted '{}': {} <-> {} [/sys/slot/src] .hoon (kook: /std/kok/boot)",
                            name, unix_path, shrine_path
                        );
                    }
                    Err(e) => {
                        println!("Error creating mount: {}", e);
                    }
                }
            }
            "unmount" => {
                if parts.len() < 2 {
                    println!("Usage: unmount <name>");
                    return Ok(());
                }

                let name = parts[1];
                let clay_path = format!("/drivers/clay/{}", name);

                let note = DriverNote {
                    path: clay_path,
                    mode: Mode::Del,
                    slots: Arc::new(ShellTale {
                        slots: HashMap::new(),
                    }),
                };
                let card = DriverCard {
                    id: self.driver_id,
                    note,
                };

                match self.tx.send(vec![card]).await {
                    Ok(_) => {
                        println!("Unmounted '{}'", name);
                    }
                    Err(e) => {
                        println!("Error removing mount: {}", e);
                    }
                }
            }
            "mounts" => match self.namespace.look(Care::Y, "/drivers/clay") {
                Ok(Observe::Found(epic)) => {
                    let mut count = 0;
                    for (path_idx, disk_saga) in epic.iter() {
                        let path_str = match self.namespace.path_idx_to_str(path_idx) {
                            Ok(Some(s)) => s,
                            _ => continue,
                        };
                        if path_str == "/drivers/clay" {
                            continue;
                        }

                        let tale = disk_saga.tale();
                        let unix = tale
                            .get("/sys/slot/unix")
                            .and_then(|p| std::str::from_utf8(p.data()).ok())
                            .unwrap_or("?");
                        let shrine = tale
                            .get("/sys/slot/shrine")
                            .and_then(|p| std::str::from_utf8(p.data()).ok())
                            .unwrap_or("?");
                        let proj = tale
                            .get("/sys/slot/projection")
                            .and_then(|p| std::str::from_utf8(p.data()).ok())
                            .unwrap_or("?");

                        let name = path_str.rsplit('/').next().unwrap_or(&path_str);
                        println!("  {} : {} <-> {} [{}]", name, unix, shrine, proj);
                        count += 1;
                    }
                    if count == 0 {
                        println!("No mounts configured");
                    }
                }
                _ => {
                    println!("No mounts configured");
                }
            },
            "export" => {
                if parts.len() < 2 {
                    println!("Usage: export <namespace-path>");
                    println!("  Exports content at <path> to a .jam file in CWD");
                    return Ok(());
                }
                let path = self.normalize_path(parts[1]);
                let ts = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_millis();
                let name = format!("export-{}", ts);
                let req_path = format!("/drv/jam-sync/{}", name);

                let tale = ExportRequestTale { path: path.clone() };
                let note = DriverNote::make(&req_path, tale);
                let card = DriverCard {
                    id: self.driver_id,
                    note,
                };

                match self.tx.send(vec![card]).await {
                    Ok(_) => {
                        println!("Export requested: {} -> {}", path, req_path);
                        println!("  Check result: cat {}", req_path);
                    }
                    Err(e) => {
                        println!("Error: {}", e);
                    }
                }
            }
            "import" => {
                if parts.len() < 2 {
                    println!("Usage: import <unix-file-path>");
                    println!("  Imports a .jam file into /std/slot/res on the request entry");
                    return Ok(());
                }
                let unix_path = if parts[1].starts_with("~/") {
                    let home = dirs::home_dir().expect("no home dir");
                    home.join(&parts[1][2..]).to_string_lossy().to_string()
                } else if parts[1].starts_with("./") || !parts[1].starts_with('/') {
                    // Resolve relative to CWD
                    let cwd = std::env::current_dir().unwrap();
                    let p = if let Some(rel) = parts[1].strip_prefix("./") {
                        cwd.join(rel)
                    } else {
                        cwd.join(parts[1])
                    };
                    p.to_string_lossy().to_string()
                } else {
                    parts[1].to_string()
                };

                let ts = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_millis();
                let name = format!("import-{}", ts);
                let req_path = format!("/drv/jam-sync/{}", name);

                let mut slots = HashMap::new();
                slots.insert(
                    slot::REQ.to_string(),
                    (ShellPailType::Text, unix_path.as_bytes().to_vec()),
                );
                let shell_tale = self.create_shell_tale(slots);
                let note = DriverNote::make(&req_path, shell_tale);
                let card = DriverCard {
                    id: self.driver_id,
                    note,
                };

                match self.tx.send(vec![card]).await {
                    Ok(_) => {
                        println!("Import requested: {} -> {}", unix_path, req_path);
                        println!("  Check result: cat {}", req_path);
                    }
                    Err(e) => {
                        println!("Error: {}", e);
                    }
                }
            }
            "debug" => {
                self.namespace.debug_current();
            }
            "check" => match self.namespace.check_invariants() {
                Ok(violations) => {
                    if violations.is_empty() {
                        println!("No invariant violations found.");
                    } else {
                        println!("{} violation(s):", violations.len());
                        for v in &violations {
                            println!("  {}", v);
                        }
                    }
                }
                Err(e) => {
                    println!("Error running invariant check: {}", e);
                }
            },
            "help" => {
                println!("Available commands:");
                println!("  pwd                          - Print working directory");
                println!("  cd [path]                    - Change directory (no path = root)");
                println!(
                    "  ls [path]                    - List immediate children (shows relative paths)"
                );
                println!("  tree [path]                  - List entire subtree");
                println!("  cat <path>                   - Show file contents and slots");
                println!(
                    "  edit <path> [slot]           - Edit slot content in $EDITOR (default: vim)"
                );
                println!(
                    "  touch <path> [slot[:type]=value ...] - Create file with optional slot data"
                );
                println!("  update <path> slot[:type]=value [...] - Update existing file slots");
                println!("  rm <path>                    - Delete path");
                println!("  mount <name> <unix> <shrine> <ext> [slot] - Create filesync mount");
                println!(
                    "  mount-src <name> <unix> <shrine> - Mount with /sys/slot/src, .hoon, kook=/std/kok/boot"
                );
                println!("  unmount <name>               - Remove filesync mount");
                println!("  mounts                       - List filesync mounts");
                println!("  export <path>                - Export namespace content to .jam file");
                println!("  import <unix-file>           - Import .jam file into namespace");
                println!("  bind #alias /path            - Bind slot namespace alias");
                println!("  unbind #alias                - Remove slot namespace binding");
                println!("  bindings                     - List all bindings");
                println!(
                    "  debug                        - Dump raw current/ancestry/why/zed tables"
                );
                println!(
                    "  check                        - Check namespace invariants (x/y/z consistency)"
                );
                println!("  help                         - Show this help");
                println!("  exit / quit                  - Exit shell");
                println!();
                println!("Slot types:");
                println!("  text  - ASCII text (default)");
                println!("  hoon  - Hoon code");
                println!("  atom  - Bigint");
                println!("  mesh  - HashMap of path to path");
                println!("  duct  - List of paths");
                println!();
                println!("Examples:");
                println!("  mount myapp /home/user/myapp /apps/myapp");
                println!("  mount code ~/src /src /sys/slot/src");
                println!("  bind #std /std/slots");
                println!("  touch /foo/bar /content:text=hello #std/author:text=alice");
                println!("  update /foo/bar /code:hoon='|= a=@ud a' /content:text=updated");
            }
            "exit" | "quit" => {
                println!("Goodbye!");
                process::exit(0);
            }
            "" => {}
            _ => {
                println!(
                    "Unknown command: {}. Type 'help' for available commands.",
                    parts[0]
                );
            }
        }

        Ok(())
    }

    async fn run(&mut self) {
        println!("Namespace Shell - Type 'help' for commands");
        loop {
            let prompt = format!("{}> ", self.helper.path);
            let readline = self.editor.readline(&prompt);
            match readline {
                Ok(line) => {
                    if !line.trim().is_empty() {
                        self.editor.add_history_entry(line.clone()).ok();
                        if let Err(e) = self.handle_command(&line).await {
                            println!("Error: {}", e);
                        }
                    }
                }
                Err(ReadlineError::Interrupted) => {
                    println!("CTRL-C");
                    break;
                }
                Err(ReadlineError::Eof) => {
                    println!("CTRL-D");
                    process::exit(0);
                }
                Err(err) => {
                    println!("Error: {:?}", err);
                    break;
                }
            }
        }
    }
}

pub struct ShellDriver {
    pub _editor: Editor<(), FileHistory>,
    id: DriverID,
    namespace: Arc<Namespace>,
    tx: Option<mpsc::Sender<Vec<DriverCard>>>,
}

impl ShellDriver {
    pub fn new(id: DriverID, namespace: Arc<Namespace>) -> Self {
        Self {
            _editor: DefaultEditor::new().expect("failed to create default editor"),
            tx: None,
            namespace,
            id,
        }
    }
}

#[async_trait]
impl ShrineDriver for ShellDriver {
    fn id(&self) -> shrine_core::driver::DriverID {
        self.id
    }
    fn subscribe(&self) -> Subscription {
        Subscription {
            path: self.namespace.get_path_id("/").unwrap().raw(),
            care: Care::Z,
        }
    }
    fn set_tx(&mut self, tx: mpsc::Sender<Vec<DriverCard>>) {
        self.tx = Some(tx);
    }

    async fn on_start(&mut self) -> Result<(), Box<dyn std::error::Error + Send>> {
        let inner_tx = self.tx.clone().unwrap();
        let ns = self.namespace.clone();
        let driver_id = self.id;
        tokio::spawn(async move {
            let mut worker = ShellWorker::new(driver_id, inner_tx, ns);
            worker.run().await;
        });
        Ok(())
    }

    async fn on_dirty(&mut self, _dirty: &[Gift]) -> Result<(), Box<dyn std::error::Error + Send>> {
        Ok(())
    }

    async fn on_done(&mut self, _ack: &DriverAck) -> Result<(), Box<dyn Error + Send>> {
        Ok(())
    }
}
