# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Overview

DarkClient is a Minecraft (Java Edition) modification framework written in Rust. It injects native libraries into a running Minecraft JVM and drives the game through JNI. One build supports both **obfuscated** Minecraft (≤ 1.21.11, via bundled Mojmap mappings) and **unobfuscated** Minecraft (26.1+, via runtime JNI reflection) — see the mapping system below. It is a Cargo workspace of three crates.

## Build & Common Commands

```bash
cargo build --release          # build all three crates
cargo build -p client --release    # build a single crate
cargo test -p client           # tests live only in client/src/mapping/class.rs
cargo test -p client test_type_compatibility   # run one test
cargo fmt
cargo clippy
python conversion.py           # regenerate mappings.json (needs the `requests` package)
```

- **Always build `--release`.** The release profile (`opt-level = "s"`, `lto = true`) is what CI and runtime expect; debug builds also silence `dead_code` warnings via `lib.rs`.
- **Windows requires the nightly toolchain** (see `.github/workflows/build.yml`) and a discoverable `jvm.lib`. `client/build.rs` and `agent_loader/build.rs` locate it via `JAVA_HOME` or `JVM_LIB_DIR`; Linux links `libjvm.so` directly. JDK 21+ required.
- Running the `injector` needs root (`sudo`) on Linux / Administrator on Windows. `libagent_loader` and `libclient` must sit in the injector's working directory.

## Crate Roles

- **`injector/`** — standalone GUI binary (egui/eframe; `--tui` flag for a crossterm TUI). Finds Java processes whose command line contains `minecraft`, injects `agent_loader`, then drives the client.
- **`agent_loader/`** — `cdylib` injected first. A `#[ctor]` runs on load: starts a JVM health monitor and a TCP command server. Owns the lifecycle of the client library (load/unload/hot-reload).
- **`client/`** — `cdylib`, the actual mod framework. JNI-driven game interaction, OpenGL overlay, module system.

## Injection & Hot-Reload Flow

This is the core control flow and spans all three crates:

1. `injector` injects `libagent_loader.so`/`.dll` into the JVM process — ptrace (`ptrace-inject`) on Linux, `dll-syringe` on Windows.
2. `agent_loader`'s `#[ctor]` `agent_onload()` starts a TCP server on **`127.0.0.1:7878`** (constant duplicated in `injector/src/platform/mod.rs::SOCKET_ADDRESS`).
3. `injector` connects and sends `reload <absolute-path-to-libclient>`.
4. `agent_loader` copies the library to a temp file (avoids file locks), `dlopen`s it, and calls the exported `initialize_client`.
5. Re-injecting repeats step 3 → `reload_client_library` calls `cleanup_client` on the old library before loading the new one. This is the hot-reload path.

`client` exposes exactly two `#[no_mangle] extern "C"` symbols: `initialize_client` and `cleanup_client`. `initialize_client` spawns a thread that builds `Minecraft::instance()`, calls `register_modules()`, and installs hooks.

## client/ Internals

**Rendering & ticking** (`graphic/hook.rs`): `install_hooks` hooks `glfwSwapBuffers` (via `ilhook`) so `on_frame` runs every frame — it renders the egui overlay (`ui_engine.rs`) and calls `check_tick`. `check_tick` compares the player's tick count to detect new game ticks and calls `DarkClient::tick()`, which ticks every enabled module. Tick logic runs on the render thread, not a Minecraft thread.

**Input** (`graphic/input.rs`): swaps GLFW key/mouse/cursor callbacks. **Right Shift** (key `344`) toggles the GUI; while the GUI is open, input events are consumed instead of forwarded to Minecraft. Module keybinds toggle modules on key press.

**Module system** (`module/mod.rs`): implement the `Module` trait (`on_start`/`on_stop`/`on_tick`, all returning `anyhow::Result<()>`) plus `ModuleData` accessors. Register new modules in `register_modules()` in `client/src/lib.rs`. Modules carry typed `ModuleSetting`s (Toggle/Slider/Choice/Color). Note: the trait example in `README.md` is stale — the real trait methods return `anyhow::Result<()>`.

**Mapping system** (`mapping/`): bridges deobfuscated (Mojmap) names — what `MinecraftClassType` and the rest of the code use — to whatever the running JVM actually exposes. `Mapping::new()` auto-detects the build by probing `find_class("net/minecraft/client/Minecraft")` and picks one of two modes:

- **Obfuscated** (`Mode::Obfuscated`): the probe fails. `mappings.json` and `java_mappings.json` (project root, **`include_str!`'d at compile time**) are parsed into a class map; names are translated deobfuscated → obfuscated.
- **Reflected** (`Mode::Reflected`): the probe succeeds (Minecraft 26.1+, unobfuscated). No JSON is used; class/method/field names are identity, and method signatures — still required by JNI — are discovered lazily via `java.lang.Class` reflection in `reflect.rs` and cached. No mapping file is ever needed for new versions.

Both modes share one code path: a `RwLock<HashMap<String, Arc<MinecraftClass>>>` populated up-front (obfuscated) or lazily by reflection (reflected). `Mapping` wraps all JNI calls (`call_method`, `call_static_method`, `get_field`, `set_field`, etc.); `class.rs` does overload resolution by scoring argument-type compatibility against JNI signatures.

**Mod loaders (`loader.rs`)**: Fabric and Forge/NeoForge run the game in an isolated class loader (`KnotClassLoader` / `TransformingClassLoader`), so `find_class` from a native thread resolves a dead duplicate of `Minecraft` whose static `instance` is null. `Mapping::new()` calls `loader::discover_game_loader` first — it scans every live thread's context class loader and keeps the one whose `Minecraft.getInstance()` is non-null. That loader is stored in `class_loader` so every later lookup goes through `ClassLoader.loadClass`. This works loader-agnostically for vanilla, Fabric and Forge on unobfuscated builds; obfuscated Minecraft under a mod loader (intermediary/SRG names) is not supported.

**Lifecycle safety**: the global `RUNNING: AtomicBool` gates `on_frame` and the agent's loops. A panic hook in `initialize_client` calls `cleanup_client` so input/render hooks are always uninstalled and GLFW callbacks restored, even on panic.

## Mappings

`conversion.py` downloads official Mojang mappings for a chosen **obfuscated** Minecraft version (≤ 1.21.11) and writes the custom `mappings.json` format. The committed `mappings.json` is ~18 MB. `java_mappings.json` is a small hand-written supplement for `java.*` classes, merged in at load time. Unobfuscated versions (26.1+) need none of this — they go through the reflected mapping path. The 26.1 runtime requires JDK 25.

## Logs

- `injector` → `app.log` (in its working directory)
- `agent_loader` → `agent_loader.log`
- `client` → `dark_client.log` (in `.minecraft`)
