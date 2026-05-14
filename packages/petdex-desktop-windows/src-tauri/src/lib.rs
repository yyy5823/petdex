use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use serde::{Deserialize, Serialize};
use tauri::State;

// ── Pet types ─────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PetMeta {
    pub slug: String,
    pub name: String,
    pub sprite_path: String,
}

// ── Sidecar state ─────────────────────────────────────────────────────────────

pub struct SidecarState {
    pub child: Option<std::process::Child>,
    pub port: u16,
    pub token: String,
}

impl Default for SidecarState {
    fn default() -> Self {
        Self { child: None, port: 0, token: String::new() }
    }
}

// ── Pet scanner ───────────────────────────────────────────────────────────────

fn pet_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Some(home) = dirs::home_dir() {
        roots.push(home.join(".petdex").join("pets"));
        roots.push(home.join(".codex").join("pets"));
    }
    roots
}

fn load_pet_from_dir(slug: &str, dir: &PathBuf) -> Option<PetMeta> {
    let pet_dir = dir.join(slug);
    let json_path = pet_dir.join("pet.json");
    if !json_path.exists() {
        return None;
    }
    let raw = fs::read_to_string(&json_path).ok()?;
    let val: serde_json::Value = serde_json::from_str(&raw).ok()?;

    let name = val.get("displayName")
        .or_else(|| val.get("name"))
        .and_then(|v| v.as_str())
        .unwrap_or(slug)
        .to_string();

    let sprite_path = ["spritesheet.webp", "spritesheet.png", "sprite.webp", "sprite.png"]
        .iter()
        .find_map(|fname| {
            let p = pet_dir.join(fname);
            if p.exists() { Some(p.to_string_lossy().to_string()) } else { None }
        })?;

    Some(PetMeta { slug: slug.to_string(), name, sprite_path })
}

// ── Sidecar helpers ───────────────────────────────────────────────────────────

/// Resolve the node executable — tries common install locations so we work
/// even when app.exe inherits a PATH that doesn't include nodejs.
fn find_node() -> PathBuf {
    // Try PATH first (works in dev when launched from a node-aware shell)
    if let Ok(out) = std::process::Command::new("where.exe").arg("node").output() {
        if out.status.success() {
            if let Ok(s) = std::str::from_utf8(&out.stdout) {
                if let Some(line) = s.lines().next() {
                    let p = PathBuf::from(line.trim());
                    if p.exists() { return p; }
                }
            }
        }
    }
    // Common fixed install paths on Windows
    for candidate in &[
        r"C:\Program Files\nodejs\node.exe",
        r"C:\Program Files (x86)\nodejs\node.exe",
    ] {
        let p = PathBuf::from(candidate);
        if p.exists() { return p; }
    }
    // Final fallback — let OS resolve it
    PathBuf::from("node")
}

fn find_sidecar_js() -> Option<PathBuf> {
    // Production install path (set by `petdex install desktop`)
    if let Some(home) = dirs::home_dir() {
        let installed = home.join(".petdex").join("sidecar").join("server.js");
        if installed.exists() {
            return Some(installed);
        }
    }
    // Dev/CI override — set PETDEX_SIDECAR_PATH to the absolute path of server.js
    if let Ok(env_path) = std::env::var("PETDEX_SIDECAR_PATH") {
        let p = PathBuf::from(&env_path);
        if p.exists() {
            return Some(p);
        }
    }
    None
}

/// Port is fixed at 7777 (or PETDEX_PORT env var).
/// Token is written by the sidecar to ~/.petdex/runtime/update-token as plain text.
fn read_runtime_info() -> Option<(u16, String)> {
    let port: u16 = std::env::var("PETDEX_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(7777);

    let token_path = dirs::home_dir()?
        .join(".petdex")
        .join("runtime")
        .join("update-token");

    let token = if token_path.exists() {
        fs::read_to_string(&token_path)
            .ok()
            .map(|t| t.trim().to_string())
            .unwrap_or_default()
    } else {
        String::new()
    };

    Some((port, token))
}

// ── Tauri commands — pet ──────────────────────────────────────────────────────

/// Return the slugs of all pets found under ~/.petdex/pets and ~/.codex/pets.
#[tauri::command]
fn list_pets() -> Vec<String> {
    let mut slugs = Vec::new();
    for root in pet_roots() {
        if let Ok(entries) = fs::read_dir(&root) {
            for entry in entries.flatten() {
                if entry.path().is_dir() {
                    if let Some(name) = entry.file_name().to_str() {
                        if entry.path().join("pet.json").exists() {
                            slugs.push(name.to_string());
                        }
                    }
                }
            }
        }
    }
    slugs.sort();
    slugs.dedup();
    slugs
}

/// Load metadata for a specific pet by slug; returns None if not installed.
#[tauri::command]
fn get_pet(slug: String) -> Option<PetMeta> {
    for root in pet_roots() {
        if let Some(meta) = load_pet_from_dir(&slug, &root) {
            return Some(meta);
        }
    }
    None
}

/// Return the active pet from ~/.petdex/runtime/state.json, or the first installed pet.
#[tauri::command]
fn get_active_pet() -> Option<PetMeta> {
    let runtime_path = dirs::home_dir()?.join(".petdex").join("runtime").join("state.json");
    let active_slug: Option<String> = if runtime_path.exists() {
        let raw = fs::read_to_string(&runtime_path).ok()?;
        let val: serde_json::Value = serde_json::from_str(&raw).ok()?;
        val.get("active_pet").and_then(|v| v.as_str()).map(|s| s.to_string())
    } else {
        None
    };

    if let Some(slug) = active_slug {
        if let Some(meta) = get_pet(slug) {
            return Some(meta);
        }
    }

    let slugs = list_pets();
    slugs.first().and_then(|s| get_pet(s.clone()))
}

/// Read a file and return its contents as a base64 string.
/// Restricted to paths inside the user's home directory to prevent path traversal.
#[tauri::command]
fn read_file_as_base64(path: String) -> Option<String> {
    use std::io::Read;
    // Security: restrict reads to the user's home directory.
    // This command is exposed to WebView JS; without this guard any
    // JS executing in the window (or injected) could read arbitrary files.
    let home = dirs::home_dir()?;
    let canonical = fs::canonicalize(&path).ok()?;
    if !canonical.starts_with(&home) {
        return None;
    }
    let mut f = fs::File::open(&canonical).ok()?;
    let mut buf = Vec::new();
    f.read_to_end(&mut buf).ok()?;
    Some(base64_encode(&buf))
}

fn base64_encode(input: &[u8]) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for chunk in input.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
        let n = (b0 << 16) | (b1 << 8) | b2;
        out.push(CHARS[((n >> 18) & 63) as usize] as char);
        out.push(CHARS[((n >> 12) & 63) as usize] as char);
        out.push(if chunk.len() > 1 { CHARS[((n >> 6) & 63) as usize] as char } else { '=' });
        out.push(if chunk.len() > 2 { CHARS[(n & 63) as usize] as char } else { '=' });
    }
    out
}

// ── Tauri commands — sidecar ──────────────────────────────────────────────────

/// Spawn the sidecar server; kill any stale instance first. Returns the port (default 7777).
#[tauri::command]
fn spawn_sidecar(state: State<Mutex<SidecarState>>) -> Result<u16, String> {
    let sidecar_path = find_sidecar_js()
        .ok_or_else(|| "sidecar server.js not found in ~/.petdex/sidecar/ or repo".to_string())?;

    // Kill any stale sidecar first so port 7777 is free
    {
        let mut s = state.lock().unwrap();
        if let Some(mut old) = s.child.take() {
            let _ = old.kill();
        }
    }

    let node = find_node();
    let child = std::process::Command::new(&node)
        .arg(&sidecar_path)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|e| format!("failed to spawn sidecar (node={:?}): {e}", node))?;

    let (port, token) = read_runtime_info()
        .ok_or_else(|| "could not determine port/token".to_string())?;

    let mut s = state.lock().unwrap();
    s.child = Some(child);
    s.port = port;
    s.token = token;

    // Return port immediately — JS polls /health until sidecar is ready
    Ok(port)
}

/// Return the port the sidecar is currently listening on (0 if not running).
#[tauri::command]
fn get_sidecar_port(state: State<Mutex<SidecarState>>) -> u16 {
    state.lock().unwrap().port
}

/// Kill the sidecar process and clear its port/token from state.
#[tauri::command]
fn stop_sidecar(state: State<Mutex<SidecarState>>) {
    let mut s = state.lock().unwrap();
    if let Some(mut child) = s.child.take() {
        let _ = child.kill();
    }
    s.port = 0;
    s.token = String::new();
}

// ── Tauri commands — app ──────────────────────────────────────────────────────

/// Exit the application cleanly (triggered by right-click).
#[tauri::command]
fn quit_app(app: tauri::AppHandle) {
    app.exit(0);
}

// ── Entry point ───────────────────────────────────────────────────────────────

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(Mutex::new(SidecarState::default()))
        .invoke_handler(tauri::generate_handler![
            list_pets,
            get_pet,
            get_active_pet,
            read_file_as_base64,
            quit_app,
            spawn_sidecar,
            get_sidecar_port,
            stop_sidecar,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
