DarkClient

DarkClient is a modular Minecraft runtime framework written in Rust.

The project focuses on JNI integration, runtime JVM interaction, hot-reloadable systems, and a modern user interface built from the ground up.

Unlike traditional clients built around large Java codebases, DarkClient keeps most of the heavy lifting inside native Rust code while exposing clean abstractions for modules, rendering systems, configuration management, and runtime mappings.

Features

- Native Rust architecture
- Direct JNI integration
- Runtime JVM interaction
- Hot-reloadable modules
- Multi-profile configuration system
- Theme engine
- ClickGUI framework
- Mapping abstraction layer
- Cross-platform support
- Automated workspace tooling

Workspace

DarkClient/
├── protocol/
├── injector/
├── agent_loader/
├── client/
├── mapping_derive/
├── xtask/
└── mappings.json

Design Philosophy

DarkClient is built around three principles:

Performance

Critical systems run natively in Rust with minimal JVM overhead.

Maintainability

The codebase is split into isolated crates with clear responsibilities.

Extensibility

New modules, render components, settings, and mappings can be added without modifying core systems.

Development

git clone https://github.com/meklasdev/uddcmc.git
cd uddcmc

cargo build --release

Generate mappings:

python conversion.py

Run injector:

./target/release/injector

Creating Modules

Modules implement the "Module" trait and are registered through the client registry.

The framework automatically handles:

- state management
- configuration
- keybinds
- GUI integration
- serialization

Goals

- Stable runtime architecture
- Clean internal APIs
- Modern UI framework
- Scriptable extension system
- Fast version migration support

License

GPL-3.0