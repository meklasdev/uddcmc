# 🔮 DarkClient — Premium Minecraft Injection Framework

```text
  ██████╗   █████╗  ██████╗ ██╗  ██╗ ██████╗██╗     ██╗███████╗███╗   ██╗████████╗
  ██╔══██╗ ██╔══██╗ ██╔══██╗██║ ██╔╝██╔════╝██║     ██║██╔════╝████╗  ██║╚══██╔══╝
  ██║  ██║ ███████║ ██████╔╝█████╔╝ ██║     ██║     ██║█████╗  ██╔██╗ ██║   ██║
  ██║  ██║ ██╔══██║ ██╔══██╗██╔═██╗ ██║     ██║     ██║██╔══╝  ██║╚██╗██║   ██║
  ██████╔╝ ██║  ██║ ██║  ██║██║  ██╗╚██████╗███████╗██║███████╗██║ ╚████║   ██║
  ╚═════╝  ╚═╝  ╚═╝ ╚═╝  ╚═╝╚═╝  ╚═╝ ╚══════╝╚══════╝╚═╝╚══════╝╚═╝  ╚═══╝   ╚═╝
```

<p align="center">
  <img src="https://img.shields.io/badge/Rust-1.95.0%2B-orange.svg" alt="Rust 1.95.0+">
  <img src="https://img.shields.io/badge/Java-JDK%2021-blue.svg" alt="JDK 21+">
  <img src="https://img.shields.io/badge/Platform-Windows%20%7C%20Linux-lightgrey.svg" alt="Windows | Linux">
  <img src="https://img.shields.io/badge/Status-Premium%20Release-brightgreen.svg" alt="Premium Release">
  <img src="https://img.shields.io/badge/License-GNU%20GPL%20v3-blue" alt="GPL License">
</p>

**DarkClient** is a state-of-the-art, high-performance Minecraft injection framework built entirely in Rust. It utilizes the **Java Native Interface (JNI)** to integrate directly with Minecraft's Java Runtime Environment (JVM) at near-zero overhead.

Featuring an ultra-modern premium design language, a hot-swappable module framework, multi-profile layouts, and real-time Netty packet interception, DarkClient provides the ultimate platform for game modification and JVM runtime analysis.

---

## 🖼️ Redesigned Interface Preview

### 1. Premium Dark Launcher & Loader
A completely redesigned, pixel-perfect launcher utilizing a minimalist dark UI theme. It supports sequential, live real-time progress steps for locating, hooking, and injecting client frameworks into the running JVM.

```text
┌────────────────────────────────────────────────────────┐
│  DARKCLIENT INJECTOR                                   │
│  ────────────────────────────────────────────────────  │
│  [🔄 Scan]                                  1 found    │
│                                                        │
│  ┌──────────────────────────────────────────────────┐  │
│  │ ● PID 14242                                      │  │
│  │   Minecraft 1.21.10 (Fabric / KnotClassLoader)   │  │
│  └──────────────────────────────────────────────────┘  │
│                                                        │
│  ┌──────────────────────────────────────────────────┐  │
│  │ ◌ Detecting target Java Virtual Machine...        │  │
│  │   [████████████████████████░░░░] 75%             │  │
│  └──────────────────────────────────────────────────┘  │
│  ────────────────────────────────────────────────────  │
│  [               INJECT PREMIUM CLIENT              ]  │
└────────────────────────────────────────────────────────┘
```

### 2. ClickGUI Dashboard
An in-game draggable dashboard with fluid spring physics. It includes custom-rendered animated toggle switches, smooth category sliders, precise choice combos, and keybind listeners that allow on-the-fly configuration.

---

## ⚡ Premium Features

- **🚀 Live State Injection**: Hot-swap modifications dynamically without restarting Minecraft.
- **🎨 Custom Aesthetic Themes**: Switch between premium presets (*Emerald, Aqua, Amethyst, Ruby, Gold, Sakura*) in real-time with responsive UI recalculation.
- **📁 Multi-Profile System**: Save, switch, duplicate, and manage different module layouts (e.g. *Legit, Blatant, Custom*) stored safely with **atomic writes** to prevent any config corruption.
- **📡 Netty-Pipeline Packet Interceptor**: Intercept and mutate raw client-server network traffic (pure JNI, no JVMTI) enabling flawless velocity control, anti-knockback, and packet spoofing.
- **⚡ Advanced Rendering (ESP & HUD)**: World-space Player/Mob/Chest ESP rendering coupled with an elegant, always-on overlay HUD containing an animated presence factor staircase module list.
- **🔒 Process Hygiene & Stability**: Fully automated startup cleaning routines that discover and remove orphaned temporary loader binaries, leaving your systems pristine.

---

## 🏗️ Technical Workspace Architecture

DarkClient is structured as a Cargo workspace consisting of five high-performance crates and an `xtask` orchestrator:

```text
DarkClient/
├── 📁 protocol/             # High-speed injector ⇆ agent IPC socket contract
├── 📁 injector/             # Premium Loader app (Sleek egui GUI & TUI CLI)
├── 📁 agent_loader/         # Injected JVM bridge: Command server + hot-reload runtime
├── 📁 client/               # Core client modification library
│   └── 📁 src/
│       ├── 📄 lib.rs        # Main DLL entry (initialize_client / cleanup_client)
│       ├── 📄 state.rs      # Global mapping and client registration state
│       ├── 📄 config.rs     # Atomic profile configurations & User Settings
│       ├── 📁 mapping/      # JNI-to-JVM memory reflection layout mapping
│       ├── 📁 graphic/      # ClickGUI, dynamic theme, input filters, and HUD
│       ├── 📁 module/       # Base Module definitions and features registry
│       └── 📁 net/          # Java Netty-pipeline packet hook & dispatcher
├── 📁 mapping_derive/       # Rust proc-macro for automatic JVM object wrappers
├── 📁 xtask/                # Custom workspace automation scripts
├── 📄 mappings.json         # Obfuscated runtime mappings
└── 📄 conversion.py         # Mojang map formatting utility
```

---

## 📋 Prerequisites

To compile or develop DarkClient, ensure you have:
- **Rust 1.95.0+** (using cargo)
- **Java Development Kit (JDK) 21+** (for JNI headers and testing)
- **Minecraft Java Edition** (Vanilla, Fabric, or Forge)

---

## 🛠️ Compilation & Quick Start

### 1. Clone the Repository
```bash
git clone https://github.com/meklasdev/uddcmc.git
cd uddcmc
```

### 2. Format Mappings
Generate Mojmap mappings for obfuscated builds (e.g. Minecraft 1.21.10):
```bash
python conversion.py
```

### 3. Compile the Workspace
Compile the entire workspace with maximum optimization:
```bash
cargo build --release
```
*Note: Make sure that `libagent_loader` and `libclient` artifacts are placed in the same directory as the `injector` binary when starting injection.*

### 4. Inject
1. Launch Minecraft and load into the Main Menu or a World.
2. Open a terminal with elevated privileges (`sudo` on Linux or Administrator on Windows) and start the launcher:
   ```bash
   ./target/release/injector
   ```
3. Click **Scan**, choose your Minecraft process, and hit **Inject Premium Client**.
4. Press `Right Shift` (or your customized keybind) in-game to toggle the ClickGUI!

---

## 🚀 Interactive Module Development

Creating new client capabilities is straightforward. Simply implement the `Module` trait:

```rust
use crate::module::{Module, ModuleData};

pub struct CustomSpeedModule {
    data: ModuleData,
}

impl Module for CustomSpeedModule {
    fn get_module_data(&self) -> &ModuleData {
        &self.data
    }

    fn get_module_data_mut(&mut self) -> &mut ModuleData {
        &mut self.data
    }

    fn on_start(&self) -> anyhow::Result<()> {
        log::info!("Speed module loaded.");
        Ok(())
    }

    fn on_stop(&self) -> anyhow::Result<()> {
        log::info!("Speed module unloaded.");
        Ok(())
    }

    fn on_tick(&self) -> anyhow::Result<()> {
        // Manipulate game coordinates through JVM wrappers
        Ok(())
    }
}
```

Register your module inside `register_modules()` located in `client/src/lib.rs` and the system will automatically handle binding, layout rendering, and configuration storage!

---

## 🗺️ Product Roadmap

- [x] Full UI/UX Visual Redesign (Dashboard, Minimal Dark aesthetics)
- [x] Custom Theme presets and Dynamic Accent Color matching
- [x] Multi-Profile Layout preservation with Atomic File operations
- [x] Multi-stage loader sequence with real-time operations progress
- [x] Temporary bin sanitation and automated system hygiene
- [ ] In-game customizable HUD layout dragging and snapping
- [ ] Auto-updating releases system linked with GitHub API releases
- [ ] Scriptable JavaScript/Lua API for custom-made modules

---

## ⚖️ Legal Notice

This software is designed solely for educational, research, and technical-audit purposes. Users are fully responsible for maintaining compliance with Mojang's EULA, commercial guidelines, and local regulations. The developers assume no liability for misuse.

---

## 📄 License

This project is licensed under the GPL-3.0 License. See the [LICENSE](LICENSE) file for more information.
