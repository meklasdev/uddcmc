//! Netty pipeline injection — **Phase 1: plumbing only.**
//!
//! Defines `DarkChannelHandler` (a thin Netty bridge, bytecode embedded from
//! `client/java/DarkChannelHandler.class`) into Minecraft's class loader, binds
//! its native methods to Rust, and inserts an instance into the live server
//! connection's Netty pipeline.
//!
//! For now the native callbacks only **log** each packet type once and pass
//! the packet through unchanged — this proves the injection path works before
//! the Rust packet structs and the dispatch/modules are built on top (Phase 2).
//!
//! All JNI here uses explicit descriptors: the navigated classes are Netty
//! (overload-heavy) and the targets are unobfuscated Minecraft 26.1+, so going
//! through the reflecting `Mapping` layer would be both wasteful and ambiguous.

pub mod packet;

use crate::mapping::MappedObject;
use crate::net::packet::move_player::MovePlayerPacket;
use crate::net::packet::Packet;
use crate::state::{mapping, minecraft};
use jni::objects::{GlobalRef, JClass, JObject, JValue};
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
const SIG_GET_CLIENT_LISTENER: &str =
    "()Lnet/minecraft/client/multiplayer/ClientPacketListener;";
const SIG_GET_CONNECTION: &str = "()Lnet/minecraft/network/Connection;";
const SIG_CHANNEL_FIELD: &str = "Lio/netty/channel/Channel;";
const SIG_PIPELINE: &str = "()Lio/netty/channel/ChannelPipeline;";
const SIG_PIPELINE_GET: &str = "(Ljava/lang/String;)Lio/netty/channel/ChannelHandler;";
const SIG_PIPELINE_REMOVE: &str = "(Ljava/lang/String;)Lio/netty/channel/ChannelHandler;";
const SIG_PIPELINE_ADD_LAST: &str =
    "(Ljava/lang/String;Lio/netty/channel/ChannelHandler;)Lio/netty/channel/ChannelPipeline;";

struct NetState {
    /// The defined `DarkChannelHandler` class, once registered.
    handler_class: Option<GlobalRef>,
    /// The `Connection` our handler is currently installed on.
    installed_on: Option<GlobalRef>,
    /// Set if class definition failed, so it is not retried forever.
    failed: bool,
}

static STATE: Mutex<NetState> = Mutex::new(NetState {
    handler_class: None,
    installed_on: None,
    failed: false,
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
        if state.failed {
            return Ok(());
        }
        if state.handler_class.is_none() {
            match define_handler() {
                Ok(class) => {
                    log::info!("net: DarkChannelHandler defined and bound");
                    state.handler_class = Some(class);
                }
                Err(error) => {
                    log::warn!("net: handler definition failed, disabling: {error}");
                    state.failed = true;
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

/// `DefineClass` the handler into the game class loader and `RegisterNatives`.
fn define_handler() -> anyhow::Result<GlobalRef> {
    let mut env = mapping().get_env()?;
    let loader = mapping()
        .game_class_loader()
        .ok_or_else(|| anyhow::anyhow!("game class loader not captured yet"))?;

    let class = env.define_class(HANDLER_CLASS, loader.as_obj(), HANDLER_BYTECODE)?;

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
    env.register_native_methods(&class, &methods)?;

    Ok(env.new_global_ref(class)?)
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

/// Inserts our handler at the tail of `connection`'s Netty pipeline.
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
        env.call_method(
            &pipeline,
            "addLast",
            SIG_PIPELINE_ADD_LAST,
            &[JValue::Object(&name), JValue::Object(&handler)],
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
fn pipeline_of<'a>(
    env: &mut JNIEnv<'a>,
    connection: &GlobalRef,
) -> anyhow::Result<JObject<'a>> {
    let channel = env
        .get_field(connection.as_obj(), "channel", SIG_CHANNEL_FIELD)?
        .l()?;
    if channel.is_null() {
        return Err(anyhow::anyhow!("connection has no channel"));
    }
    Ok(env.call_method(&channel, "pipeline", SIG_PIPELINE, &[])?.l()?)
}

// --- packet dispatch -------------------------------------------------------

/// `DarkChannelHandler.onOutbound` — runs the outbound packet transformers and
/// returns the object to forward (the original, or a modified replacement).
unsafe extern "system" fn dark_on_outbound(
    env: *mut jni::sys::JNIEnv,
    _class: jni::sys::jclass,
    packet: jobject,
) -> jobject {
    catch_unwind(AssertUnwindSafe(
        || match unsafe { dispatch_outbound(env, packet) } {
            Ok(Some(replacement)) => replacement,
            _ => packet,
        },
    ))
    .unwrap_or(packet)
}

/// `DarkChannelHandler.onInbound` — pass-through for now (inbound transformers
/// are a later phase).
unsafe extern "system" fn dark_on_inbound(
    _env: *mut jni::sys::JNIEnv,
    _class: jni::sys::jclass,
    packet: jobject,
) -> jobject {
    packet
}

/// Inspects an outbound packet: wraps it for the modules, lets each enabled
/// module's `handle_packet` modify it, and rebuilds the JVM object if it
/// changed. `Ok(None)` means "forward the original unchanged".
unsafe fn dispatch_outbound(
    env: *mut jni::sys::JNIEnv,
    packet: jobject,
) -> anyhow::Result<Option<jobject>> {
    if packet.is_null() {
        return Ok(None);
    }
    let mut env = unsafe { JNIEnv::from_raw(env)? };
    let packet_obj = unsafe { JObject::from_raw(packet) };

    if !is_instance(&mut env, &packet_obj, packet::move_player::CLASS)? {
        return Ok(None);
    }

    let original = MovePlayerPacket::read(&mut env, &packet_obj)?;
    let mut wrapped = Packet::MovePlayer(original);
    crate::state::client().modules.handle_packet(&mut wrapped);

    let Packet::MovePlayer(modified) = wrapped;
    if modified != original {
        return Ok(Some(modified.to_java(&mut env)?));
    }
    Ok(None)
}

/// Whether `object` is an instance of the class named `class_name`.
fn is_instance(env: &mut JNIEnv, object: &JObject, class_name: &str) -> anyhow::Result<bool> {
    let class = mapping().resolve_class(env, class_name)?;
    Ok(env.is_instance_of(object, &class)?)
}
