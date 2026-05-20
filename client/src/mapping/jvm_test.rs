//! In-process JVM integration tests for the mapping layer.
//!
//! These boot a real JVM — via the `jni` crate's `invocation` feature — with
//! a small Java fixture on its class path (see `client/tests/java/`) and
//! exercise the reflected mapping path and the menu/in-world transitions
//! against it. A JDK (`javac` on `PATH`) is required to run them.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, OnceLock};

use jni::objects::JValue;
use jni::{InitArgsBuilder, JNIVersion, JavaVM};

use crate::mapping::Mapping;

/// JNI name of the fixture class used for the reflection tests.
const SAMPLE: &str = "com/darkclient/fixture/Sample";

/// Compiles the Java fixture once and returns the output classes directory.
fn fixture_classes() -> &'static Path {
    static DIR: OnceLock<PathBuf> = OnceLock::new();
    DIR.get_or_init(|| {
        let sources_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/java");
        let out = std::env::temp_dir().join("darkclient_jvm_fixture");
        std::fs::create_dir_all(&out).expect("create the fixture output directory");

        let sources = java_sources(&sources_root);
        assert!(!sources.is_empty(), "no fixture .java sources found");

        let status = Command::new("javac")
            .arg("-d")
            .arg(&out)
            .args(&sources)
            .status()
            .expect("`javac` (a JDK) must be available to run the JVM tests");
        assert!(status.success(), "fixture compilation failed");
        out
    })
}

/// Recursively collects every `.java` file under `root`.
fn java_sources(root: &Path) -> Vec<PathBuf> {
    let mut sources = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|ext| ext == "java") {
                sources.push(path);
            }
        }
    }
    sources
}

/// The shared in-process JVM, created once with the fixture on its class path.
fn jvm() -> &'static JavaVM {
    static JVM: OnceLock<JavaVM> = OnceLock::new();
    JVM.get_or_init(|| {
        let classpath = format!("-Djava.class.path={}", fixture_classes().display());
        let args = InitArgsBuilder::new()
            .version(JNIVersion::V8)
            .option(&classpath)
            .build()
            .expect("build the JVM init arguments");
        JavaVM::new(args).expect("create the in-process JVM")
    })
}

/// Builds a `Mapping` against the shared JVM. The fixture ships a class named
/// `net/minecraft/client/Minecraft`, so the mapping selects reflected mode.
fn reflected_mapping() -> Mapping {
    jvm(); // ensure the JVM exists before `Mapping` probes for it
    Mapping::new().expect("Mapping::new against the fixture JVM")
}

/// Invokes a no-argument `void` static method on the fixture `Minecraft`.
fn call_fixture_static(method: &str) {
    let mut env = jvm()
        .attach_current_thread_as_daemon()
        .expect("attach to the JVM");
    let class = env
        .find_class("net/minecraft/client/Minecraft")
        .expect("fixture Minecraft class");
    env.call_static_method(class, method, "()V", &[])
        .unwrap_or_else(|e| panic!("calling {method}: {e}"));
}

#[test]
fn mapping_reflects_a_fixture_class() {
    let mapping = reflected_mapping();
    let class = mapping.get_class(SAMPLE).expect("Sample must reflect");
    let methods = class.method_names();
    for expected in ["value", "greet", "sum", "create"] {
        assert!(
            methods.iter().any(|m| m == expected),
            "reflected class is missing method `{expected}`",
        );
    }
}

#[test]
fn reflected_method_signatures_are_built_from_jni_types() {
    let mapping = reflected_mapping();
    let class = mapping.get_class(SAMPLE).unwrap();
    assert!(
        class
            .get_method_by_signature("greet", "(Ljava/lang/String;)Ljava/lang/String;")
            .is_ok(),
        "greet(String) signature should be reflected",
    );
    assert!(
        class.get_method_by_signature("sum", "(IJ)J").is_ok(),
        "sum(int, long) signature should be reflected",
    );
}

#[test]
fn an_overloaded_method_reflects_every_overload() {
    let mapping = reflected_mapping();
    let class = mapping.get_class(SAMPLE).unwrap();
    assert_eq!(
        class.get_methods("value").unwrap().len(),
        2,
        "value(int) and value(double) are distinct overloads",
    );
}

#[test]
fn overload_resolution_uses_reflected_signatures() {
    let mapping = reflected_mapping();
    let class = mapping.get_class(SAMPLE).unwrap();
    let chosen = class
        .get_method_by_args("value", &[JValue::Int(3)])
        .expect("an int argument must resolve an overload");
    assert_eq!(chosen.signature, "(I)I");
}

#[test]
fn get_class_caches_reflected_results() {
    let mapping = reflected_mapping();
    let first = mapping.get_class(SAMPLE).unwrap();
    let second = mapping.get_class(SAMPLE).unwrap();
    assert!(
        Arc::ptr_eq(&first, &second),
        "the second lookup must hit the cache",
    );
}

#[test]
fn resolving_a_missing_class_fails() {
    let mapping = reflected_mapping();
    let mut env = mapping.get_env().unwrap();
    assert!(
        mapping
            .resolve_class(&mut env, "totally/made/up/Class")
            .is_err(),
        "a class that does not exist must not resolve",
    );
}

#[test]
fn client_initializes_and_tracks_the_menu_and_world_states() {
    jvm();
    crate::state::init().expect("state::init must succeed against the fixture");
    let minecraft = crate::state::minecraft();

    // A fresh init — the fixture is in its "main menu" state.
    assert!(
        minecraft.player().unwrap().is_none(),
        "no player in the menu"
    );
    assert!(minecraft.world().unwrap().is_none(), "no world in the menu");
    assert!(minecraft.game_mode().unwrap().is_none());
    assert!(!minecraft.in_world());

    // Join a world through the fixture.
    call_fixture_static("enterWorld");
    assert!(
        minecraft.player().unwrap().is_some(),
        "the player must be present once in a world",
    );
    assert!(minecraft.world().unwrap().is_some());
    assert!(minecraft.in_world());

    // Leave it again.
    call_fixture_static("leaveWorld");
    assert!(
        minecraft.player().unwrap().is_none(),
        "the player must be gone after leaving the world",
    );
    assert!(!minecraft.in_world());
}
