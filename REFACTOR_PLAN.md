# DarkClient — Refactor Plan

> Branch: `refactor/project-cleanup` (from `master`)
> Goal: cleaner, leaner, faster project. Deeper restructure allowed (module
> boundaries, traits and data flow may change). Behavior is preserved — no
> feature is removed. Linux + Windows must keep working; macOS is not
> implemented but every platform seam is designed so it can be added later.

## Decisions (confirmed with the user)

| Topic | Decision |
|---|---|
| Mapping access | Global accessor — remove `&Mapping` from **all** constructors and from `FieldType`. |
| Singletons | Refactor: `DarkClient`/`Minecraft`/`Mapping` collapse into **one** explicitly-initialized global `Client` (no `Arc`, no lazy panic). |
| Refactor depth | Deeper restructure — internal APIs may change. |
| Confusing "loader" | The `agent_loader` crate (monolithic `lib.rs`). `mapping/loader.rs` stays. |
| Injector GUI | Full visual redesign **+** code cleanup. Keep the `--tui` mode. |
| macOS | Not implemented now. All `#[cfg]` seams get a `macos` arm (stub returning `Unsupported`). |
| Menu injection | Injecting from the **main menu** (not in-game) must fully work — game state acquired lazily. |
| Testing | Tiered: pure unit tests + an in-process JVM integration framework. Full Minecraft e2e optional / manual. |

## Working rules

- Verify each phase with `cargo check` (workspace + per-crate). Full
  `--release` builds only at phase boundaries when needed.
- Windows / macOS code is **review-only** — not cross-compiled here.
- Each phase ends in a compiling state with its own commit. Behavior unchanged
  (except the deliberate menu-injection fix).
- `cargo fmt` + `cargo clippy` clean at every phase boundary.
- Tests must stay green after the phase that introduces them.

---

## Current pain points (from analysis)

**Workspace** — `SOCKET_ADDRESS` / the `reload` protocol is duplicated as string
literals across `injector` and `agent_loader`; common deps not centralized;
each crate inits its logger with `File::create(...).unwrap()`.

**injector** — TCP-reload block copy-pasted in `unix.rs` and `windows.rs`;
platform layer is bare `#[cfg]` re-exports, no trait; `~7` `.unwrap()`/`.expect()`
panic points; GUI status is a single `String`; injection blocks the UI thread
for up to 5 s; process detection is case-sensitive and hard-codes binary names.

**agent_loader** — everything in one 379-line `lib.rs`; primitive
`splitn(2, ' ')` command parsing; serial blocking server; messy
`format!("{:?}", ...)` + quote-trimming path munging; logger init unguarded;
mutex poisoning unhandled.

**client** — three singletons (`DarkClient`/`Minecraft`/`Mapping`), each a
`OnceLock<Arc<T>>` where the `Arc` is never cloned (dead heap alloc + atomics)
and lazy init `panic!`s at a nondeterministic first-access point; `&Mapping`
threaded through `LocalPlayer/Abilities/World/Window` constructors and
`FieldType::Object(_, &Mapping)` (~15 explicit passes); `FieldType` carries a
lifetime only for that; ~90 `.unwrap()`; no typed errors; module storage
triple-wrapped `Arc<RwLock<HashMap<String, Arc<Mutex<_>>>>>`; `esp.rs` is
1081 lines; frame/GL hook layer is `#[cfg]`-scattered and x86-64 only;
`tick()` `panic!`s if a module fails to stop.

**Menu-injection bug** — `Minecraft::new()` eagerly builds `LocalPlayer`,
`World` and `MultiPlayerGameMode`. In the main menu `Minecraft.player`,
`.level` and `.gameMode` are all null, so `new()` fails, `Minecraft::instance()`
panics, and modules that get an `Err` from `on_tick` are auto-disabled.
Injection "succeeds" but the log is full of failures.

---

## Target architecture

### New crate: `protocol/` (lib)

Single source of truth for injector ⇆ agent_loader IPC.

```
protocol/src/lib.rs
  pub const SOCKET_ADDR: SocketAddr   // 127.0.0.1:7878
  pub enum Command { Reload(PathBuf), Ping, ... }
  Command::encode(&self) -> String / decode(&str) -> Result<Command>
  pub fn init_file_logger(path) -> Result<()>   // non-panicking, shared
```

Depended on by `injector` and `agent_loader`. Unit-tested (encode/decode round-trip).

### `injector/`

```
injector/src/
  main.rs            entry: arg parse, logger, privilege check, GUI/TUI dispatch
  app.rs             UI-agnostic core: process scan + injection orchestration,
                     InjectionStatus enum, runs injection on a worker thread
  inject.rs          high-level flow: platform inject -> protocol reload
  platform/
    mod.rs           trait Injector + ProcessInfo + cfg-selected impl
    discovery.rs     cross-platform Minecraft process discovery (sysinfo)
    linux.rs         ptrace-inject
    windows.rs       dll-syringe
    macos.rs         stub -> Err(Unsupported)
  gui/
    mod.rs           eframe App (thin: renders app.rs state)
    theme.rs         colors / fonts / spacing
    widgets.rs       process card, status banner, action button
  tui.rs             crossterm TUI, rebuilt on app.rs core
```

- `trait Injector { fn inject(&self, pid, agent: &Path) -> Result<(), InjectError>; }`
- Async injection: worker thread + `mpsc` channel; GUI polls `InjectionStatus`
  (`Idle / Scanning / Injecting / Done / Failed(msg)`). No async runtime added.
- `InjectError` (thiserror): `Privilege / ProcessGone / Attach / Inject / Connect / Protocol`.

### `agent_loader/`

```
agent_loader/src/
  lib.rs        #[ctor]/#[dtor], globals, wires the modules together
  logging.rs    non-panicking logger init (via protocol helper)
  jvm.rs        get_jvm() + JVM health monitor
  server.rs     TCP command server loop
  command.rs    dispatch over protocol::Command
  library.rs    client lib lifecycle: load / unload / reload (hot-reload)
  platform.rs   signal handlers — cfg(unix); macos/windows arms
```

- `library.rs` keeps the temp-copy hot-reload but uses clean `Path` APIs.
- Mutex poisoning handled (recover, not panic).

### `client/` — one global `Client`

The three singletons collapse into a single owned runtime root, explicitly
initialized once, `Arc`-free:

```rust
// client/src/state.rs
static CLIENT: OnceLock<Client> = OnceLock::new();

pub struct Client {
    pub jvm: JavaVM,
    pub mapping: Mapping,
    pub minecraft: Minecraft,   // window-level handle, always valid once injected
    pub modules: ModuleRegistry,
}

/// Called exactly once, from `initialize_client`. The single, known init point.
pub fn init() -> Result<(), ClientError> {
    CLIENT.set(Client::new()?).map_err(|_| ClientError::AlreadyInitialized)
}

/// Infallible accessor — valid after `init()` succeeded.
#[inline]
pub fn client() -> &'static Client {
    CLIENT.get().expect("client() used before init()")
}

/// Convenience — `&client().mapping`.
#[inline]
pub fn mapping() -> &'static Mapping { &client().mapping }
```

- No `Arc`: access = one atomic-acquire load + branch, `#[inline]`d.
- `Mapping` / `Minecraft` become **fields**, not singletons. `&Mapping` removed
  from every constructor; `FieldType` loses its lifetime →
  `Object(MinecraftClassType)`.
- `GameContext` trait dropped (or reduced to nothing) — replaced by the free
  `client()` / `mapping()` functions.
- `new()` no longer `unsafe` — the unsafe JNI calls are wrapped internally.
- Init order is straight-line in `initialize_client`: `state::init()?` →
  `register_modules()` → install hooks **last** (so `on_frame` never observes an
  uninitialized `Client`; `RUNNING` is set true only after init succeeds).

### `client/` — menu-safe lazy game state

`Minecraft.getInstance()` and the game `Window` exist from the main menu
onward. `player`, `level`/`world` and `gameMode` are **world-scoped**: null in
the menu, populated on world join, null again on leave. So they must never be
built in a constructor — only fetched on demand.

```rust
pub struct Minecraft {
    jni_ref: GlobalRef,   // Minecraft.getInstance() — valid from menu onward
    window: Window,       // valid from menu onward
}

impl Minecraft {
    /// `Ok(None)` in the menu / not in a world. `Err` only on a real JNI fault.
    pub fn player(&self)    -> Result<Option<LocalPlayer>>;
    pub fn world(&self)     -> Result<Option<World>>;
    pub fn game_mode(&self) -> Result<Option<MultiPlayerGameMode>>;
    pub fn in_world(&self)  -> bool;
}
```

- `Result<Option<T>>` is honest: `Err` = JNI failure, `Ok(None)` = not in world.
- `player()` keeps the existing cache (`RwLock<Option<LocalPlayer>>`) with the
  `is_same_object` staleness check.
- Every world-dependent module's `on_tick` early-returns `Ok(())` when not in
  world — "nothing to do", **not** an error, so the module is not disabled:

  ```rust
  fn on_tick(&self) -> anyhow::Result<()> {
      let Some(player) = client().minecraft.player()? else { return Ok(()) };
      // ... real logic
  }
  ```

Result: injecting from the menu initializes cleanly; modules sit idle until a
world loads, then start working — no log spam, no auto-disable.

### `client/` — other restructuring

```
graphic/
  platform/
    mod.rs      trait FrameHook + trait GlLoader, cfg-selected
    linux.rs    glX/glfw via dlsym + ilhook
    windows.rs  wgl + ilhook
    macos.rs    stub -> Err(Unsupported)
  esp/          esp.rs (1081 LOC) split: math.rs / gather.rs / render.rs / mod.rs
module/
  registry.rs   ModuleRegistry — single Mutex<Vec<_>>, not the triple wrapper
```

- `ClientError` (thiserror) at mapping/JNI boundaries; `anyhow` stays at the
  module-trait boundary. Lock access via a `lock_or_err` helper.
- `DarkClient::tick()` no longer `panic!`s on a failing module — log + disable.

---

## Testing strategy

A faithful test "framework" **is** feasible without launching Minecraft: the
`jni` crate (`invocation` feature) can create a real in-process JVM, and the
reflected mapping path is plain JNI reflection — it only needs classes named
like Minecraft's, not Minecraft itself.

**Tier 1 — pure unit tests (fast, CI, no JVM).** Done as a dedicated phase
(Phase T1) after the refactor, so they are reviewed together:
- `protocol`: `Command` encode/decode round-trip.
- `client`: `class.rs` overload scoring (exists), ESP projection math,
  `mappings.json` parsing, `FieldType` signature strings.
- `injector`: process-discovery filtering (pure fn over fake process lists).

**Tier 2 — in-process JVM integration framework (CI-capable, needs a JDK).**
A dedicated test harness, e.g. `client/tests/jvm/`:
- A tiny Java fixture — stub classes (`net/minecraft/client/Minecraft` with a
  static `getInstance`, a fake player/world, a custom classloader to emulate
  Fabric's `KnotClassLoader`) compiled to a jar.
- Rust tests boot a `JavaVM`, load the fixture, and exercise the **real**
  code paths: reflected `Mapping` resolution, `loader::discover_game_loader`
  classloader scanning, method-signature reflection, `call_method` overload
  resolution, and the **menu vs in-world** transitions (fixture toggles
  `player`/`level` between null and set).
- Fixture build wired via a `build.rs` or a `cargo xtask` step (`javac`).

**Tier 3 — full Minecraft e2e (optional, manual).** A documented `cargo xtask`
that launches a real Minecraft, injects, and asserts over the TCP channel /
logs. Marked `#[ignore]` / not run in CI — too slow and flaky for automation.
Provided as an opt-in harness; not a phase blocker.

If Tier 2's JDK-at-test-time cost is unwanted in CI, it can be gated behind a
feature flag and Tier 1 alone runs in CI — but Tier 2 is the recommended core.

---

## Phases

Each phase = one commit, compiles, behavior unchanged (except Phase 5).

### Phase 0 — Workspace foundation
- New `protocol` crate: `SOCKET_ADDR`, `Command`, encode/decode, shared
  non-panicking logger helper.
- Centralize common deps in `[workspace.dependencies]` (`log`, `simplelog`,
  `anyhow`, `thiserror`, `libc`, `libloading`, `jni`, `serde`, `sysinfo`,
  `crossterm`, `ctor`).
- ✅ `cargo check` workspace.

### Phase 1 — injector: platform layer + core
- `platform/`: `Injector` trait, `linux.rs` / `windows.rs` / `macos.rs` (stub),
  `discovery.rs` (case-insensitive, robust binary-name match).
- Extract duplicated TCP-reload into `inject.rs` using `protocol`.
- `app.rs`: UI-agnostic core + `InjectionStatus`; injection on a worker thread.
- Remove all `.unwrap()`/`.expect()` panic points; `InjectError` type.
- ✅ `cargo check -p injector`.

### Phase 2 — injector: GUI redesign + TUI
- New `gui/` (theme, widgets, layout): process cards, clear status/progress
  states, non-blocking injection wired to `app.rs`.
- Rebuild `tui.rs` on the shared `app.rs` core.
- ✅ `cargo check -p injector`; manual GUI smoke test on Linux.

### Phase 3 — agent_loader: modularize
- Split `lib.rs` into `logging / jvm / server / command / library / platform`.
- Command dispatch via `protocol::Command`; clean `Path` handling; handle
  mutex poisoning; guarded logger init.
- ✅ `cargo check -p agent_loader`.

### Phase 4 — client: global `Client` state
- Implement `state.rs`: one `OnceLock<Client>`, `init()` + `client()` / `mapping()`.
- Collapse `DarkClient` + `Minecraft` + `Mapping` singletons into `Client`
  fields; drop the dead `Arc`s; drop `unsafe fn new()`.
- `mapping()` global accessor — remove `&Mapping` from all constructors; drop
  `FieldType`'s lifetime. Fix the straight-line init order in `lib.rs`.
- ✅ `cargo check -p client` + `cargo test -p client`.

### Phase 5 — client: menu-safe lazy game state
- `Minecraft` keeps only `jni_ref` + `window`; `player()` / `world()` /
  `game_mode()` become lazy `Result<Option<_>>` accessors.
- `init()` succeeds in the main menu.
- (Module no-op behavior lands in Phase 7.)
- ✅ `cargo check -p client`; manual: inject from menu, no error log.

### Phase 6 — client: error handling
- `ClientError` (thiserror) at mapping/JNI boundaries; `lock_or_err` helper.
- Remove critical-path `.unwrap()`; `tick()` and `init()` stop panicking.
- Convert the mapping caches (`classes`, `class_handles`) to `DashMap`.
- ✅ `cargo check -p client` + `cargo test -p client`.

### Phase 7 — client: module system
- `ModuleRegistry` backed by `DashMap`; tidy `Module` trait;
  keep explicit `register_modules()` (zero-dep, lean).
- Every world-dependent module `on_tick` early-returns `Ok(())` when not in
  world — completes the menu-injection fix.
- ✅ `cargo check -p client`.

### Phase 8 — client: graphic platform seam
- `graphic/platform/` (`mod` + `linux` / `windows` / `macos`) exposing
  `gl_proc_address`, `open_glfw_library`, `frame_hook_targets`.
- `esp.rs` split **dropped** — it is already cleanly sectioned; splitting it
  is cosmetic churn on working render code (user decision).
- ✅ `cargo check -p client` + `cargo test -p client`.

### Phase 9 — T1: pure unit tests (no JVM)
- `protocol`: `Command` encode/decode round-trip.
- `client`: `class.rs` overload scoring (exists), ESP projection math,
  `mappings.json` parsing, `FieldType` signature strings.
- `injector`: process-discovery filtering (pure fn over fake process lists).
- Fast, CI-friendly. Reviewed together before moving on.
- ✅ `cargo test` workspace.

### Phase 10 — T2: JVM integration framework
- Java fixture (stub Minecraft classes + fake Fabric classloader), built via
  `xtask`/`build.rs`.
- `client/tests/jvm/`: in-process `JavaVM` tests for reflected `Mapping`,
  classloader discovery, overload resolution, menu↔in-world transitions.
- ✅ `cargo test -p client` (with JDK).

### Phase 11 — T3: e2e harness (optional / manual)
- `xtask` crate + `cargo xtask e2e`: builds the workspace, discovers a
  running Minecraft and injects into it; the overlay check is manual.
- Headless injector modes `--list` / `--inject <pid>` make injection
  scriptable. Not run in CI (needs a running game + root).
- ✅ manual run.

### Phase 12 — polish
- `cargo fmt` + `cargo clippy` clean across the workspace.
- Update `CLAUDE.md` and `README.md` (the README `Module` trait example is
  already stale) to match the new structure.
- Final `cargo build --release` (Linux); review Windows/macOS paths.
- Remove this file or move it to `docs/`.

---

## Risks

- **Phase 4** changes global init — mitigated: one explicit, documented init
  point instead of three lazy ones; `cargo test` after. Lower risk than the
  original lazy-`OnceLock` design.
- **Phase 5/7** menu-safe state touches every module — mitigated by the
  uniform `let Some(..) = ..? else { return Ok(()) }` pattern.
- Frame/GL hooking (`ilhook`) is x86-64 only — the macOS stub compiles but
  returns `Unsupported`; real macOS hooking is out of scope.
- Windows code can't be verified here — kept review-only.
- Tier 2 tests need a JDK + `javac` at test time — can be feature-gated if CI
  cost is unwanted.

## Out of scope

- macOS implementation (only the seams).
- New client features / modules.
- Mapping-format or `conversion.py` changes.
- Replacing `egui`/`ilhook`/`jni`.
