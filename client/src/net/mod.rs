//! Netty pipeline injection — the packet layer.
//!
//! Defines `DarkChannelHandler` (a thin Netty bridge, bytecode embedded from
//! `client/java/DarkChannelHandler.class`) into Minecraft's class loader, binds
//! its native methods to Rust, and inserts an instance into the live server
//! connection's Netty pipeline.
//!
//! Every packet then flows through [`dispatch`]: it is wrapped into a
//! [`packet::Packet`] value-snapshot (only for the types a module handles),
//! offered to every enabled module's `handle_packet`, and — if a module
//! changed it — rebuilt into a fresh JVM object that replaces the original.
//!
//! All JNI here uses explicit descriptors: the navigated classes are Netty
//! (overload-heavy) and the targets are unobfuscated Minecraft 26.1+, so going
//! through the reflecting `Mapping` layer would be both wasteful and ambiguous.

pub mod packet;

use crate::mapping::MappedObject;
use crate::net::packet::{Packet, PacketAction};
use crate::state::{mapping, minecraft};
use jni::objects::{GlobalRef, JClass, JObject, JString, JValue};
use jni::sys::jobject;
use jni::{JNIEnv, NativeMethod};
use std::ffi::c_void;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::Mutex;

/// Bytecode of the `DarkChannelHandler` Netty bridge.
const HANDLER_BYTECODE: &[u8] = include_bytes!("../../java/DarkChannelHandler.class");
/// Binary name of the handler class.
const HANDLER_CLASS: &str = "DarkChannelHandler";
/// Name of our entry in Minecraft's Netty pipeline.
const PIPELINE_NAME: &str = "dark_handler";

// JNI descriptors for the methods/fields this module touches.
const SIG_GET_CLIENT_LISTENER: &str = "()Lnet/minecraft/client/multiplayer/ClientPacketListener;";
const SIG_GET_CONNECTION: &str = "()Lnet/minecraft/network/Connection;";
const SIG_CHANNEL_FIELD: &str = "Lio/netty/channel/Channel;";
const SIG_PIPELINE: &str = "()Lio/netty/channel/ChannelPipeline;";
const SIG_PIPELINE_GET: &str = "(Ljava/lang/String;)Lio/netty/channel/ChannelHandler;";
const SIG_PIPELINE_REMOVE: &str = "(Ljava/lang/String;)Lio/netty/channel/ChannelHandler;";
const SIG_PIPELINE_ADD_BEFORE: &str = "(Ljava/lang/String;Ljava/lang/String;Lio/netty/channel/ChannelHandler;)Lio/netty/channel/ChannelPipeline;";

/// Minecraft's own pipeline entry (`Connection`). Our handler must sit *before*
/// it: inbound packets flow head→tail, so to see — and rewrite — a packet
/// before Minecraft processes it, we have to be upstream of this handler.
const MC_PACKET_HANDLER: &str = "packet_handler";

/// How many times defining the handler class may fail before giving up — a
/// genuine, repeatable failure should not spam the log forever.
const MAX_DEFINE_ATTEMPTS: u32 = 20;

struct NetState {
    /// The `DarkChannelHandler` class, once defined and bound.
    handler_class: Option<GlobalRef>,
    /// The `Connection` our handler is currently installed on.
    installed_on: Option<GlobalRef>,
    /// Consecutive class-definition failures — capped by `MAX_DEFINE_ATTEMPTS`.
    define_attempts: u32,
}

static STATE: Mutex<NetState> = Mutex::new(NetState {
    handler_class: None,
    installed_on: None,
    define_attempts: 0,
});

/// Polls the live connection and makes sure our handler sits on its pipeline.
/// Cheap and idempotent — meant to be called once per game tick.
pub fn ensure_installed() {
    if let Err(error) = ensure_installed_inner() {
        log::debug!("net: install pass failed: {error}");
    }
    clear_pending_exception();
}

/// Drops any stray pending JNI exception. This code runs inside Minecraft's
/// `glfwSwapBuffers` call — a leaked exception would crash the game the moment
/// control returns to its Java frame.
fn clear_pending_exception() {
    if let Ok(env) = mapping().get_env() {
        if env.exception_check().unwrap_or(false) {
            let _ = env.exception_clear();
            log::warn!("net: cleared a stray JNI exception");
        }
    }
}

fn ensure_installed_inner() -> anyhow::Result<()> {
    // Define + bind the handler class once.
    {
        let mut state = STATE.lock().unwrap();
        if state.handler_class.is_none() {
            if state.define_attempts >= MAX_DEFINE_ATTEMPTS {
                return Ok(());
            }
            match define_handler() {
                Ok(class) => {
                    log::info!("net: DarkChannelHandler defined and bound");
                    state.handler_class = Some(class);
                    state.define_attempts = 0;
                }
                Err(error) => {
                    state.define_attempts += 1;
                    log::warn!(
                        "net: handler definition failed (attempt {}/{}): {error}",
                        state.define_attempts,
                        MAX_DEFINE_ATTEMPTS
                    );
                    return Ok(());
                }
            }
        }
    }

    let mut env = mapping().get_env()?;

    // Find the live server connection.
    let Some(connection) = current_connection(&mut env)? else {
        STATE.lock().unwrap().installed_on = None;
        return Ok(());
    };

    // Already installed on this exact connection?
    {
        let state = STATE.lock().unwrap();
        if let Some(previous) = &state.installed_on {
            if env.is_same_object(previous, &connection)? {
                return Ok(());
            }
        }
    }

    install_on(&mut env, &connection)?;
    STATE.lock().unwrap().installed_on = Some(connection);
    log::info!("net: handler installed on the connection pipeline");
    Ok(())
}

/// Makes the `DarkChannelHandler` class available and binds its native methods
/// to this library's functions.
///
/// On the first injection the class is `DefineClass`'d into the game class
/// loader. On a **hot-reload** it is already defined there — a class name can
/// be defined only once per loader, and a second `DefineClass` throws a
/// `LinkageError` — so the existing class is reused instead. Either way the
/// native methods are (re)bound: on reload the previous binding points into the
/// now-unloaded old library and *must* be replaced.
fn define_handler() -> anyhow::Result<GlobalRef> {
    let mut env = mapping().get_env()?;
    let loader = mapping()
        .game_class_loader()
        .ok_or_else(|| anyhow::anyhow!("game class loader not captured yet"))?;

    let class = match load_existing_handler(&mut env, &loader) {
        Some(existing) => existing,
        None => env
            .define_class(HANDLER_CLASS, loader.as_obj(), HANDLER_BYTECODE)
            .map_err(|error| describe_jni_error(&mut env, "DefineClass", error))?,
    };

    let methods = [
        NativeMethod {
            name: "onOutbound".into(),
            sig: "(Ljava/lang/Object;)Ljava/lang/Object;".into(),
            fn_ptr: dark_on_outbound as *mut c_void,
        },
        NativeMethod {
            name: "onInbound".into(),
            sig: "(Ljava/lang/Object;)Ljava/lang/Object;".into(),
            fn_ptr: dark_on_inbound as *mut c_void,
        },
    ];
    env.register_native_methods(&class, &methods)
        .map_err(|error| describe_jni_error(&mut env, "RegisterNatives", error))?;

    Ok(env.new_global_ref(class)?)
}

/// Returns the `DarkChannelHandler` class if a previous injection already
/// defined it in `loader` (the hot-reload case), otherwise `None`.
fn load_existing_handler<'a>(env: &mut JNIEnv<'a>, loader: &GlobalRef) -> Option<JClass<'a>> {
    let Ok(name) = env.new_string(HANDLER_CLASS) else {
        return None;
    };
    let result = env.call_method(
        loader.as_obj(),
        "loadClass",
        "(Ljava/lang/String;)Ljava/lang/Class;",
        &[JValue::Object(&name)],
    );
    match result.and_then(|value| value.l()) {
        Ok(class) if !class.is_null() => Some(JClass::from(class)),
        _ => {
            // Not defined yet — `loadClass` threw `ClassNotFoundException`.
            let _ = env.exception_clear();
            None
        }
    }
}

/// Folds the pending Java exception's text into `error`, so a failed JNI call
/// reports *what* the JVM threw rather than the opaque "Java exception thrown".
fn describe_jni_error(env: &mut JNIEnv, what: &str, error: jni::errors::Error) -> anyhow::Error {
    if env.exception_check().unwrap_or(false) {
        let detail = env
            .exception_occurred()
            .ok()
            .and_then(|throwable| {
                let _ = env.exception_clear();
                env.call_method(&throwable, "toString", "()Ljava/lang/String;", &[])
                    .ok()
            })
            .and_then(|value| value.l().ok())
            .and_then(|obj| {
                let jstr = JString::from(obj);
                let text = env.get_string(&jstr).ok()?;
                Some(text.to_string_lossy().into_owned())
            });
        if let Some(detail) = detail {
            return anyhow::anyhow!("{what} failed: {detail}");
        }
    }
    anyhow::anyhow!("{what} failed: {error}")
}

/// `minecraft.getConnection().getConnection()` — the live `Connection`, if any.
fn current_connection(env: &mut JNIEnv) -> anyhow::Result<Option<GlobalRef>> {
    env.with_local_frame(8, |env| -> anyhow::Result<Option<GlobalRef>> {
        let listener = env
            .call_method(
                minecraft().jni_ref().as_obj(),
                "getConnection",
                SIG_GET_CLIENT_LISTENER,
                &[],
            )?
            .l()?;
        if listener.is_null() {
            return Ok(None);
        }
        let connection = env
            .call_method(&listener, "getConnection", SIG_GET_CONNECTION, &[])?
            .l()?;
        if connection.is_null() {
            return Ok(None);
        }
        Ok(Some(env.new_global_ref(connection)?))
    })
}

/// Inserts our handler into `connection`'s Netty pipeline, just before
/// Minecraft's own `packet_handler` — so it sees both inbound and outbound
/// packets before the game does.
fn install_on(env: &mut JNIEnv, connection: &GlobalRef) -> anyhow::Result<()> {
    let handler_class = STATE
        .lock()
        .unwrap()
        .handler_class
        .clone()
        .ok_or_else(|| anyhow::anyhow!("handler class missing"))?;

    env.with_local_frame(16, |env| -> anyhow::Result<()> {
        let pipeline = pipeline_of(env, connection)?;
        let name: JObject = env.new_string(PIPELINE_NAME)?.into();

        // Idempotent: skip if our handler is already on this pipeline.
        let existing = env
            .call_method(&pipeline, "get", SIG_PIPELINE_GET, &[JValue::Object(&name)])?
            .l()?;
        if !existing.is_null() {
            return Ok(());
        }

        let class = JClass::from(env.new_local_ref(handler_class.as_obj())?);
        let handler = env.new_object(&class, "()V", &[])?;
        let base: JObject = env.new_string(MC_PACKET_HANDLER)?.into();
        env.call_method(
            &pipeline,
            "addBefore",
            SIG_PIPELINE_ADD_BEFORE,
            &[
                JValue::Object(&base),
                JValue::Object(&name),
                JValue::Object(&handler),
            ],
        )?;
        Ok(())
    })
}

/// Releases everything before the library is unloaded — called from
/// `cleanup_client`. Leaving the handler on the pipeline would crash the JVM
/// (its native methods would point at unmapped memory).
pub fn teardown() {
    let (connection, class) = {
        let mut state = STATE.lock().unwrap();
        // A fresh injection gets a fresh definition-retry budget.
        state.define_attempts = 0;
        (state.installed_on.take(), state.handler_class.take())
    };

    if let (Some(connection), Ok(mut env)) = (connection, mapping().get_env()) {
        env.with_local_frame(16, |env| -> anyhow::Result<()> {
            let pipeline = pipeline_of(env, &connection)?;
            let name: JObject = env.new_string(PIPELINE_NAME)?.into();
            let existing = env
                .call_method(&pipeline, "get", SIG_PIPELINE_GET, &[JValue::Object(&name)])?
                .l()?;
            if !existing.is_null() {
                env.call_method(
                    &pipeline,
                    "remove",
                    SIG_PIPELINE_REMOVE,
                    &[JValue::Object(&name)],
                )?;
            }
            Ok(())
        })
        .unwrap_or_else(|error| log::debug!("net: pipeline cleanup failed: {error}"));
    }

    if let Some(class) = class {
        if let Ok(mut env) = mapping().get_env() {
            if let Ok(local) = env.new_local_ref(class.as_obj()) {
                let _ = env.unregister_native_methods(JClass::from(local));
            }
        }
    }
    clear_pending_exception();
}

/// `connection.channel.pipeline()` — the Netty pipeline of a `Connection`.
/// Must be called inside an existing local-reference frame.
fn pipeline_of<'a>(env: &mut JNIEnv<'a>, connection: &GlobalRef) -> anyhow::Result<JObject<'a>> {
    let channel = env
        .get_field(connection.as_obj(), "channel", SIG_CHANNEL_FIELD)?
        .l()?;
    if channel.is_null() {
        return Err(anyhow::anyhow!("connection has no channel"));
    }
    Ok(env
        .call_method(&channel, "pipeline", SIG_PIPELINE, &[])?
        .l()?)
}

// --- packet dispatch -------------------------------------------------------

/// What the dispatch decided the Netty callback should forward.
enum Dispatch {
    /// Forward the original packet object, untouched.
    Forward,
    /// Forward this freshly built object in place of the original.
    Replace(jobject),
    /// Drop the packet — the callback returns `null` so Netty discards it
    /// (the packet is never sent, outbound, nor delivered, inbound).
    Drop,
}

/// `DarkChannelHandler.onOutbound` — dispatches an outbound packet.
unsafe extern "system" fn dark_on_outbound(
    env: *mut jni::sys::JNIEnv,
    _class: jni::sys::jclass,
    packet: jobject,
) -> jobject {
    catch_unwind(AssertUnwindSafe(|| {
        match unsafe { dispatch(env, packet, false) } {
            Ok(Dispatch::Replace(replacement)) => replacement,
            Ok(Dispatch::Drop) => std::ptr::null_mut(),
            Ok(Dispatch::Forward) | Err(_) => packet,
        }
    }))
    .unwrap_or(packet)
}

/// `DarkChannelHandler.onInbound` — dispatches an inbound packet.
unsafe extern "system" fn dark_on_inbound(
    env: *mut jni::sys::JNIEnv,
    _class: jni::sys::jclass,
    packet: jobject,
) -> jobject {
    catch_unwind(AssertUnwindSafe(|| {
        match unsafe { dispatch(env, packet, true) } {
            Ok(Dispatch::Replace(replacement)) => replacement,
            Ok(Dispatch::Drop) => std::ptr::null_mut(),
            Ok(Dispatch::Forward) | Err(_) => packet,
        }
    }))
    .unwrap_or(packet)
}

/// Wraps a packet for the modules, lets each enabled module's `handle_packet`
/// modify or cancel it, and rebuilds the JVM object if it changed. Every packet
/// type no module handles — and every error — yields [`Dispatch::Forward`], so
/// the connection is never disrupted by this layer. `inbound` selects which
/// `Packet` variants to probe.
unsafe fn dispatch(
    env: *mut jni::sys::JNIEnv,
    packet: jobject,
    inbound: bool,
) -> anyhow::Result<Dispatch> {
    if packet.is_null() {
        return Ok(Dispatch::Forward);
    }
    let mut env = unsafe { JNIEnv::from_raw(env)? };
    let packet_obj = unsafe { JObject::from_raw(packet) };

    let built = if inbound {
        Packet::from_inbound(&mut env, &packet_obj)?
    } else {
        Packet::from_outbound(&mut env, &packet_obj)?
    };
    let Some(mut wrapped) = built else {
        return Ok(Dispatch::Forward);
    };

    let original = wrapped.clone();
    match crate::state::client().modules.handle_packet(&mut wrapped) {
        PacketAction::Cancel => Ok(Dispatch::Drop),
        PacketAction::Forward if wrapped != original => {
            Ok(Dispatch::Replace(wrapped.to_java(&mut env)?))
        }
        PacketAction::Forward => Ok(Dispatch::Forward),
    }
}
