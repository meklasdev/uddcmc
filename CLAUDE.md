# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Overview

DarkClient is a Minecraft (Java Edition, mappings target **1.21.10**) modification framework written in Rust. It injects native libraries into a running Minecraft JVM and drives the game through JNI. It is a Cargo workspace of three crates.

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

**Mapping system** (`mapping/`): handles Minecraft obfuscation. `mappings.json` (project root) and `java_mappings.json` are **`include_str!`'d into the binary at compile time** by `Mapping::new()` — changing mappings requires rebuilding `client`. `MinecraftClassType` enum maps deobfuscated class names to their JSON entries; `Mapping` resolves obfuscated names and wraps all JNI calls (`call_method`, `call_static_method`, `get_field`, `set_field`, etc.). `class.rs` does overload resolution by scoring argument-type compatibility against JNI signatures.

**Lifecycle safety**: the global `RUNNING: AtomicBool` gates `on_frame` and the agent's loops. A panic hook in `initialize_client` calls `cleanup_client` so input/render hooks are always uninstalled and GLFW callbacks restored, even on panic.

## Mappings

`conversion.py` downloads official Mojang mappings for a chosen Minecraft version and writes the custom `mappings.json` format. The committed `mappings.json` is ~18 MB. `java_mappings.json` is a small hand-written supplement for `java.*` classes, merged in at load time.

## Logs

- `injector` → `app.log` (in its working directory)
- `agent_loader` → `agent_loader.log`
- `client` → `dark_client.log` (in `.minecraft`)
