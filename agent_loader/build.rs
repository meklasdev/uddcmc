// build.rs
// This build script is only relevant on Windows with MSVC toolchain.
// It finds the `jvm.lib` import library that is required to link JNI functions.
// On Linux, this is unnecessary because the linker can directly use libjvm.so.

#[cfg(windows)]
fn main() {
    use std::path::PathBuf;
    use std::{env, fs};

    println!("cargo:rerun-if-env-changed=JAVA_HOME");
    println!("cargo:rerun-if-env-changed=JVM_LIB_DIR");

    // --- 1) Explicit override: JVM_LIB_DIR environment variable ---
    if let Ok(dir) = env::var("JVM_LIB_DIR") {
        let p = PathBuf::from(&dir);
        if p.exists() {
            link_jvm(&p);
            println!("cargo:warning=Using JVM_LIB_DIR={}", dir);
            return;
        }
    }

    // --- 2) JAVA_HOME environment variable ---
    if let Ok(java_home) = env::var("JAVA_HOME") {
        let p = PathBuf::from(java_home);
        if let Some(found) = search_jvm_in(&p) {
            link_jvm(&found);
            println!(
                "cargo:warning=Found jvm.lib via JAVA_HOME at {}",
                found.display()
            );
            return;
        }
    }

    // --- 3) Search via system PATH (using java.exe/javac.exe location) ---
    if let Ok(path_env) = env::var("PATH") {
        for path_dir in env::split_paths(&path_env) {
            let java_exe = path_dir.join("java.exe");
            if java_exe.exists() {
                if let Some(parent) = path_dir.parent() {
                    if let Some(found) = search_jvm_in(&parent.to_path_buf()) {
                        link_jvm(&found);
                        println!(
                            "cargo:warning=Found jvm.lib via PATH (java.exe) at {}",
                            found.display()
                        );
                        return;
                    }
                }
            }
            let javac_exe = path_dir.join("javac.exe");
            if javac_exe.exists() {
                if let Some(parent) = path_dir.parent() {
                    if let Some(found) = search_jvm_in(&parent.to_path_buf()) {
                        link_jvm(&found);
                        println!(
                            "cargo:warning=Found jvm.lib via PATH (javac.exe) at {}",
                            found.display()
                        );
                        return;
                    }
                }
            }
        }
    }

    // --- 4) Common installation directories ---
    let mut common_roots = vec![
        PathBuf::from(r"C:\Program Files\Java"),
        PathBuf::from(r"C:\Program Files (x86)\Java"),
        PathBuf::from(r"C:\Program Files\Eclipse Adoptium"),
        PathBuf::from(r"C:\Program Files\Eclipse Foundation"),
        PathBuf::from(r"C:\Program Files\Amazon Corretto"),
        PathBuf::from(r"C:\Program Files\Microsoft"),
        PathBuf::from(r"C:\Program Files\Semeru"),
        PathBuf::from(r"C:\Program Files\BellSoft"),
        PathBuf::from(r"C:\Program Files\Zulu"),
        PathBuf::from(r"C:\tools"),
        PathBuf::from(r"C:\tools\openjdk"),
    ];

    if let Ok(user_profile) = env::var("USERPROFILE") {
        let profile_p = PathBuf::from(user_profile);
        common_roots.push(profile_p.join(".jdks"));
        common_roots.push(profile_p.join("scoop").join("apps"));
        common_roots.push(profile_p.join(".gradle").join("jdks"));
    }

    if let Ok(local_appdata) = env::var("LOCALAPPDATA") {
        let local_p = PathBuf::from(local_appdata);
        common_roots.push(local_p.join("Programs").join("Adoptium"));
        common_roots.push(local_p.join("Programs").join("Eclipse Foundation"));
        common_roots.push(local_p.join("Programs").join("Eclipse Adoptium"));
    }

    for root in &common_roots {
        if !root.exists() {
            continue;
        }
        // Check root itself
        if let Some(found) = search_jvm_in(root) {
            link_jvm(&found);
            println!("cargo:warning=Found jvm.lib in {}", found.display());
            return;
        }
        // Check first-level subdirectories
        if let Ok(entries) = fs::read_dir(root) {
            for e in entries.flatten() {
                let path = e.path();
                if let Some(found) = search_jvm_in(&path) {
                    link_jvm(&found);
                    println!("cargo:warning=Found jvm.lib in {}", found.display());
                    return;
                }
                // Check second-level subdirectories for nested structures (e.g. scoop/apps/<app>/<version>)
                if let Ok(subentries) = fs::read_dir(&path) {
                    for se in subentries.flatten() {
                        if let Some(found) = search_jvm_in(&se.path()) {
                            link_jvm(&found);
                            println!("cargo:warning=Found jvm.lib in {}", found.display());
                            return;
                        }
                    }
                }
            }
        }
    }

    // --- 5) Windows Registry lookup ---
    if let Some(found) = find_jvm_from_registry() {
        link_jvm(&found);
        println!(
            "cargo:warning=Found jvm.lib via Windows Registry at {}",
            found.display()
        );
        return;
    }

    // --- 6) Nothing found: fail build with instructions ---
    panic!(
        "build.rs: could not find jvm.lib. 
    - Set JAVA_HOME to your JDK root (e.g. C:\\Program Files\\Java\\jdk-21)
    - Or set JVM_LIB_DIR directly to the folder containing jvm.lib"
    );
}

#[cfg(not(windows))]
fn main() {
    // On non-Windows systems this build script does nothing.
}

#[cfg(windows)]
// Adds the directory to the linker search path and tells Cargo to link against jvm.lib
fn link_jvm(dir: &std::path::PathBuf) {
    println!("cargo:rustc-link-search=native={}", dir.display());
    println!("cargo:rustc-link-lib=dylib=jvm");
}

#[cfg(windows)]
// Tries common subpaths of a JDK installation to find jvm.lib
fn search_jvm_in(root: &std::path::PathBuf) -> Option<std::path::PathBuf> {
    let tries = [
        root.join("lib").join("jvm.lib"),
        root.join("lib").join("amd64").join("jvm.lib"),
        root.join("lib").join("x86_64").join("jvm.lib"),
        root.join("lib").join("server").join("jvm.lib"),
        root.join("lib").join("client").join("jvm.lib"),
    ];
    for candidate in &tries {
        if candidate.exists() {
            return candidate.parent().map(|p| p.to_path_buf());
        }
    }
    None
}

// Tries to query Windows Registry for the JDK installation path
#[cfg(windows)]
fn find_jvm_from_registry() -> Option<std::path::PathBuf> {
    use std::path::PathBuf;
    use std::process::Command;

    let keys = [
        r"HKLM\SOFTWARE\JavaSoft",
        r"HKLM\SOFTWARE\Eclipse Adoptium",
        r"HKLM\SOFTWARE\Eclipse Foundation",
        r"HKCU\SOFTWARE\JavaSoft",
    ];

    for key in &keys {
        if let Ok(output) = Command::new("reg")
            .args(["query", key, "/s"])
            .output()
        {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                let has_trigger = line.contains("JavaHome") || line.contains("Path") || line.contains("InstallDir");
                if has_trigger {
                    let mut path_str = if let Some(idx) = line.find("REG_SZ") {
                        line[idx + 6..].trim()
                    } else if let Some(idx) = line.find("REG_EXPAND_SZ") {
                        line[idx + 13..].trim()
                    } else {
                        continue;
                    };

                    // Remove quotes if present
                    if path_str.starts_with('"') && path_str.ends_with('"') && path_str.len() >= 2 {
                        path_str = &path_str[1..path_str.len() - 1];
                    }

                    if !path_str.is_empty() {
                        let p = PathBuf::from(path_str);
                        if let Some(found) = search_jvm_in(&p) {
                            return Some(found);
                        }
                    }
                }
            }
        }
    }
    None
}
