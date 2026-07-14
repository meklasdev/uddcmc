# KRASNOSTAV LUA SCRIPTING ENGINE & NATIVE BINDINGS
## Architecture & Technical Specification Document
---

This specification details the low-level, high-performance architecture required to bind a high-performance **LuaJIT** virtual machine inside a native Minecraft injection client wrapper, and connect it to a modern, dynamic user interface (such as the ClickGUI/Web-Dashboard).

Specifically, this document outlines how settings defined in dynamic user-submitted scripts (e.g. `krasnostav_pingspoof.lua`) are compiled, registered, automatically populated as UI controls, and bound directly into JVM/Netty network packet channels at runtime.

---

## 1. Core Structural Pipeline Overview

The system operates across three boundaries:
1. **JVM (Minecraft / Netty)**: The target game environment where network packets (`KeepAlive`, `PlayerPosition`, `CustomPayload`) are intercepted, released, or altered.
2. **Rust Agent (Native Engine)**: The core dynamic library injected into the JVM. It hosts the LuaJIT virtual machine via the `mlua` crate and controls memory/threading safety.
3. **UI Engine (eGUI / Web React)**: The layout layer that queries the active script environment, parses dynamic settings, and displays widgets (sliders, toggles, dropdowns).

```
 +-----------------------------------------------------------------------------------------+
 |                               1. INITIALIZATION & PARSING                               |
 |                                                                                         |
 |  [User .lua Script]                                                                     |
 |         │                                                                               |
 |         ▼                                                                               |
 |  [Rust VM Loader] ──(via mlua)──► [LuaJIT State]                                        |
 |         │                                                                               |
 |         ▼                                                                               |
 |  [Script Module Registry] ──────► [ClickGUI Registry / Web API] ──► [Auto-Generated UI] |
 +-----------------------------------------------------------------------------------------+
                                                │
                                                ▼
 +-----------------------------------------------------------------------------------------+
 |                               2. REAL-TIME PACKET ENGINE                                |
 |                                                                                         |
 |  [JVM Netty Channel] ◄──(JNI Hook)──► [Rust Network Interceptor]                       |
 |                                                   │                                     |
 |                                                   ▼ (Fast Path Thread-Safe)             |
 |                                         [Lua JIT Hook Exec]                             |
 |                                                   │                                     |
 |                                                   ▼                                     |
 |                                         [Delayed Packets Buffer] ──► [Re-inject JVM]    |
 +-----------------------------------------------------------------------------------------+
```

---

## 2. Lua Wrapper API Design

The following classes represent the wrapper bindings built over the C-ABI layer. These classes must be fully mapped into the global state of each executed Lua thread.

### 2.1 The Script Object (`script`)
Every script runs inside its own isolated environment block.
- `script:registerSetting(type, name, description, default, min, max)`
  - **Type**: `String` ("IntSetting", "ToggleSetting", "ChoiceSetting")
  - **Returns**: A thread-safe reference handle to the registered Setting.
  - Registers the control within the Rust `ModuleData` struct. The UI reads this struct to instantiate the correct widget.
- `script:onPacketSend(callback)`
  - Registers a high-speed JNI hook into the JVM Netty outgoing packet pipeline.
- `script:onUpdate(callback)`
  - Registers an update tick listener hook firing on every Minecraft client loop render frame.
- `script:release(packet)`
  - Explicitly releases a buffered packet onto the Netty pipeline (bypassing future hooks to avoid infinite recursive delay loops).

### 2.2 The Packet Handle Object (`packet`)
- `packet:hasAged(ms)`: Returns `true` if the duration since packet creation is greater than the specified milliseconds.
- `packet:getName()`: Returns class name (e.g., `CPacketKeepAlive`).
- `packet:isCanceled()`: Returns boolean cancel status.
- `packet:cancel()`: Flags the packet to be dropped by Netty.

---

## 3. High-Performance Native Bindings & Binding Systems

### 3.1 JNI-to-Netty JVM Interception
In Minecraft, network communications pass through a Netty `ChannelPipeline`.
To intercept packets safely without crashing the JVM or generating thread contention overhead:

1. **Bootstrap Channel Pipeline Interception**:
   - The Rust agent utilizes standard JNI reflection to discover the active connection manager:
     `net.minecraft.network.NetworkManager` (obfuscated as `gw` or `ek` depending on version).
   - Locate the Netty channel member inside `NetworkManager`:
     `io.netty.channel.Channel` (usually typed as `ch`).
   - Dynamically append a custom inbound/outbound handler into the Netty `ChannelPipeline` using:
     ```java
     pipeline.addBefore("packet_handler", "krasnostav_interceptor", new ChannelDuplexHandler() { ... });
     ```

2. **Native Hook Bridge**:
   - The JVM duplex handler delegates the `write(ChannelHandlerContext ctx, Object msg, ChannelPromise promise)` call into a native Rust function through JNI:
     ```rust
     #[no_mangle]
     pub extern "system" fn Java_com_krasnostav_net_Interceptor_onPacketWrite(
         env: JNIEnv,
         class: JClass,
         packet_obj: JObject,
     ) -> jboolean { ... }
     ```
   - Packets flagged as `"DELAY"` by Lua are intercepted: their native reference is placed inside a thread-safe delay queue (`Arc<Mutex<Vec<DelayedPacket>>>`), and the Java Netty promise is resolved with a *no-op* or suspended to prevent forwarding immediately.

### 3.2 Threading & Safety Model
- **The Hot Path Thread Rule**: Lua scripts must **never** perform blocking disk, network, or heavily nested loops directly inside `onPacketSend` or Netty IO worker threads (`epoll` / `nio` pools). Doing so causes micro-stutters in-game.
- **Inter-Thread Message Buffering**:
  - Netty IO thread receives the packet.
  - If a Lua script dictates a delay:
    - Clone or reference-increment the JVM global object handle of the packet.
    - Push the packet structure to a native buffer inside Rust with a timestamp:
      `struct DelayedPacket { packet_ref: GlobalRef, send_time: Instant }`.
  - The tick listener loop (`onUpdate`), running in the client's thread, checks timestamps.
  - When a packet has matured (i.e. aged past the defined slider value):
    - The `onUpdate` loop triggers `script:release(packet)`.
    - This dispatches a JNI call to the JVM back to re-enqueue the packet:
      `ctx.writeAndFlush(packet_ref)`.

---

## 4. UI Settings Binding Synchronization

To achieve automatic, dynamic rendering of settings when a `.lua` file is loaded:

1. **Serialization Manifest**:
   - As `script:registerSetting` runs during script initialization, it appends a `ModuleSetting` descriptor to a synchronized configuration vector in Rust.
2. **UI Reflection**:
   - On the next GUI render tick, the eGUI/React workspace loops through the setting manifest of the active script.
   - Example parsing loop (React Pseudo-logic matching our live frontend):
     ```javascript
     const settingsList = activeScript.settings.map(setting => {
         if (setting.type === "IntSetting") {
             return <Slider min={setting.min} max={setting.max} value={setting.value} ... />;
         } else if (setting.type === "ChoiceSetting") {
             return <Dropdown options={setting.options} selected={setting.value} ... />;
         }
     });
     ```
3. **Atomic State Updates**:
   - Moving a slider in the dashboard immediately writes to the thread-safe backing memory buffer in Rust.
   - When the Lua thread subsequently runs `script:onUpdate`, any lookup of the variable resolves instantly via a fast atomic atomic pointer dereference (`AtomicF32`/`AtomicU32`), ensuring **zero UI-to-script execution lag**.

---

## 5. Security & Isolation Considerations
- **Sandboxed Execution Context**: Standard library functions like `os.execute`, `os.remove`, `os.rename`, `io.open`, `require`, and `package` are **completely stripped** from the Lua environment.
- Only safe utility functions (`math.*`, `string.*`, `table.*`) are preserved. This prevents malicious user configurations from compromising the host operating system.
