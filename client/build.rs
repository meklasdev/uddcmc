// build.rs
// Generates the OpenGL bindings (`bindings.rs`) on every platform, and locates
// the JVM library so JNI symbols link: `jvm.lib` on Windows (MSVC), `libjvm.so`
// on Linux. The injected `cdylib` could resolve those symbols from the host JVM
// at load time, but the `cargo test` executables must have them linked.

use gl_generator::{Api, Fallbacks, GlobalGenerator, Profile, Registry};
use std::env;
use std::fs::File;
use std::path::Path;

fn main() {
    // Generate the OpenGL bindings on every platform — `lib.rs` `include!`s
    // `bindings.rs` unconditionally.
    let dest = env::var("OUT_DIR").unwrap();
    let mut file = File::create(Path::new(&dest).join("bindings.rs")).unwrap();

    // Ask for OpenGL 3.3 Compatibility so we get VAOs (GenVertexArrays) and modern shader API
    Registry::new(Api::Gl, (3, 3), Profile::Compatibility, Fallbacks::All, [])
        .write_bindings(GlobalGenerator, &mut file)
        .unwrap();

    #[cfg(windows)]
    find_jvm_lib();

    #[cfg(target_os = "linux")]
    link_jvm_linux();
}

/// Links `libjvm.so` on Linux by locating it through `java-locator`.
#[cfg(target_os = "linux")]
fn link_jvm_linux() {
    println!("cargo:rerun-if-env-changed=JAVA_HOME");
    match java_locator::locate_jvm_dyn_library() {
        Ok(dir) => {
            println!("cargo:rustc-link-search=native={dir}");
            println!("cargo:rustc-link-lib=dylib=jvm");
            // RPATH so the `cargo test` executables can find libjvm.so at
            // run time. Harmless for the injected cdylib — it resolves libjvm
            // from the host JVM process, which has it loaded already.
            println!("cargo:rustc-link-arg=-Wl,-rpath,{dir}");
        }
        Err(e) => {
            println!("cargo:warning=libjvm.so could not be located: {e}");
        }
    }
}

#[cfg(windows)]
// Finds the `jvm.lib` import library that is required to link JNI functions on
// the MSVC toolchain. On Linux the linker can use libjvm.so directly.
fn find_jvm_lib() {
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

    // --- 3) Common installation directories ---
    let common = [r"C:\Program Files\Java", r"C:\Program Files (x86)\Java"];

    for root in &common {
        let rootp = PathBuf::from(root);
        if let Ok(entries) = fs::read_dir(&rootp) {
            for e in entries.flatten() {
                if let Some(found) = search_jvm_in(&e.path()) {
                    link_jvm(&found);
                    println!("cargo:warning=Found jvm.lib in {}", found.display());
                    return;
                }
            }
        }
    }

    // --- 4) Windows Registry lookup ---
    if let Some(found) = find_jvm_from_registry() {
        link_jvm(&found);
        println!(
            "cargo:warning=Found jvm.lib via Windows Registry at {}",
            found.display()
        );
        return;
    }

    // --- 5) Nothing found: fail build with instructions ---
    panic!(
        "build.rs: could not find jvm.lib. 
    - Set JAVA_HOME to your JDK root (e.g. C:\\Program Files\\Java\\jdk-21)
    - Or set JVM_LIB_DIR directly to the folder containing jvm.lib"
    );
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

    let output = Command::new("reg")
        .args(["query", r"HKLM\SOFTWARE\JavaSoft\JDK", "/s"])
        .output()
        .ok()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        if line.contains("JavaHome") {
            let parts: Vec<_> = line.split_whitespace().collect();
            if let Some(path) = parts.last() {
                let p = PathBuf::from(path);
                if let Some(found) = search_jvm_in(&p) {
                    return Some(found);
                }
            }
        }
    }
    None
}
