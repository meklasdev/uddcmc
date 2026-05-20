//! Cross-platform discovery of running Minecraft instances.

use std::path::Path;

use sysinfo::System;

/// A Minecraft process the user can inject into.
#[derive(Debug, Clone)]
pub struct ProcessInfo {
    /// Operating-system process id.
    pub pid: u32,
    /// Human-readable label — the game version when it can be parsed off the
    /// command line, otherwise a generic description.
    pub info: String,
}

/// Scans running processes and returns every Java process that looks like a
/// Minecraft client, sorted by pid.
pub fn find_minecraft_processes() -> Vec<ProcessInfo> {
    let mut sys = System::new_all();
    sys.refresh_all();

    let mut found: Vec<ProcessInfo> = sys
        .processes()
        .iter()
        .filter_map(|(pid, process)| {
            let exe = process.name().to_string_lossy();
            let args: Vec<String> = process
                .cmd()
                .iter()
                .map(|a| a.to_string_lossy().into_owned())
                .collect();
            classify(&exe, &args).map(|info| ProcessInfo {
                pid: pid.as_u32(),
                info,
            })
        })
        .collect();

    found.sort_by_key(|p| p.pid);
    found
}

/// Pure classification: given a process executable name and its command-line
/// arguments, decide whether it is a Minecraft client and, if so, produce a
/// display label. Free of `sysinfo` types so it can be unit-tested.
fn classify(exe_name: &str, args: &[String]) -> Option<String> {
    if !is_java_executable(exe_name) {
        return None;
    }
    if !args.join(" ").to_lowercase().contains("minecraft") {
        return None;
    }
    Some(extract_version(args).unwrap_or_else(|| "Minecraft instance".to_string()))
}

/// True if `exe_name` is a Java launcher binary, ignoring case and any
/// platform extension (`java`, `javaw`, `java.exe`, `javaw.exe`).
fn is_java_executable(exe_name: &str) -> bool {
    let stem = Path::new(exe_name)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(exe_name);
    stem.eq_ignore_ascii_case("java") || stem.eq_ignore_ascii_case("javaw")
}

/// Pulls the value following a `--version` argument, if present.
fn extract_version(args: &[String]) -> Option<String> {
    let idx = args.iter().position(|a| a == "--version")?;
    args.get(idx + 1).cloned()
}
