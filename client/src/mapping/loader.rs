//! Game class-loader discovery for modded and obfuscated Minecraft.
//!
//! Vanilla Minecraft runs every class through a single class loader, so a JNI
//! `FindClass` from any thread resolves `net.minecraft.*` correctly. Mod
//! loaders break that assumption: Fabric's `KnotClassLoader` and ModLauncher's
//! `TransformingClassLoader` load the game in an *isolated* loader while the
//! launch bootstrap still sits on the system class path. `FindClass` from a
//! native thread then resolves a **second, dead copy** of `Minecraft` whose
//! static `instance` field is null.
//!
//! For **obfuscated** builds the same issue applies: even though the obfuscated
//! class short-name (e.g. `bcj`) technically lives on the application classpath,
//! modern launchers (post-Bootstrap) sometimes use an isolated `URLClassLoader`
//! as the render thread's context loader, causing the system-loader copy to have
//! a null `instance` field. We therefore capture the Render thread's context
//! class loader even in obfuscated mode.
//!
//! [`Mapping::lookup_class`]: crate::mapping::Mapping

use jni::objects::{GlobalRef, JClass, JObject, JString, JValue};
use jni::JNIEnv;

/// Binary name of the client entry-point class — identical on every
/// unobfuscated build, vanilla or modded.
const MINECRAFT_CLASS: &str = "net.minecraft.client.Minecraft";

/// Exact JNI descriptor of `Minecraft.getInstance()`. `GetStaticMethodID`
/// matches signatures verbatim, so the real return type is required here.
const GET_INSTANCE_SIG: &str = "()Lnet/minecraft/client/Minecraft;";

/// Name Minecraft gives its main client thread on every modern version.
const RENDER_THREAD: &str = "Render thread";

/// Result of probing one thread's context class loader.
enum LoaderProbe {
    /// The loader owns a `Minecraft` whose `getInstance()` is non-null — this
    /// is the loader that runs the game.
    Live(GlobalRef),
    /// The loader belongs to the `Render thread` and can load `Minecraft`, but
    /// `getInstance()` was still null (game not finished starting). Kept as a
    /// fallback in case no loader reports a live instance.
    RenderThread(GlobalRef),
    /// The loader cannot load an unobfuscated `Minecraft`.
    Unrelated,
}

/// Finds the class loader that owns the live Minecraft instance.
///
/// Returns `None` for vanilla obfuscated builds (no loader exposes an
/// unobfuscated `Minecraft`) and on any JNI error, in which case the caller
/// falls back to plain `FindClass` resolution.
pub fn discover_game_loader(env: &mut JNIEnv) -> Option<GlobalRef> {
    match scan_threads(env) {
        Ok(loader) => loader,
        Err(_) => {
            let _ = env.exception_clear();
            None
        }
    }
}

/// Finds the context class loader of the "Render thread".
///
/// Used in obfuscated mode so that `ClassLoader.loadClass(obfuscated_name)`
/// goes through the same loader that actually loaded the game classes instead
/// of the system class loader (which may hold a dead, un-initialized copy).
///
/// Returns `None` if the render thread has not started yet or on any JNI error.
pub fn discover_render_thread_loader(env: &mut JNIEnv) -> Option<GlobalRef> {
    let thread_class = env.find_class("java/lang/Thread").ok()?;
    let traces = env
        .call_static_method(&thread_class, "getAllStackTraces", "()Ljava/util/Map;", &[])
        .ok()?
        .l()
        .ok()?;
    let threads = env
        .call_method(&traces, "keySet", "()Ljava/util/Set;", &[])
        .ok()?
        .l()
        .ok()?;
    let iter = env
        .call_method(&threads, "iterator", "()Ljava/util/Iterator;", &[])
        .ok()?
        .l()
        .ok()?;

    while env
        .call_method(&iter, "hasNext", "()Z", &[])
        .ok()?
        .z()
        .ok()?
    {
        let result = env.with_local_frame(16, |env| -> anyhow::Result<Option<GlobalRef>> {
            let thread = env
                .call_method(&iter, "next", "()Ljava/lang/Object;", &[])?
                .l()?;
            let name_obj = env
                .call_method(&thread, "getName", "()Ljava/lang/String;", &[])?
                .l()?;
            let binding = JString::from(name_obj);
            let name = env.get_string(&binding)?;
            if name.to_str().ok() != Some(RENDER_THREAD) {
                return Ok(None);
            }
            let loader = env
                .call_method(
                    &thread,
                    "getContextClassLoader",
                    "()Ljava/lang/ClassLoader;",
                    &[],
                )?
                .l()?;
            if loader.is_null() {
                return Ok(None);
            }
            Ok(Some(env.new_global_ref(loader)?))
        });
        if let Ok(Some(loader)) = result {
            return Some(loader);
        }
    }
    None
}

/// Walks `Thread.getAllStackTraces()` and probes each thread's context class
/// loader, returning the first loader proven to run the game.
fn scan_threads(env: &mut JNIEnv) -> anyhow::Result<Option<GlobalRef>> {
    let thread_class = env.find_class("java/lang/Thread")?;
    let traces = env
        .call_static_method(&thread_class, "getAllStackTraces", "()Ljava/util/Map;", &[])?
        .l()?;
    let threads = env
        .call_method(&traces, "keySet", "()Ljava/util/Set;", &[])?
        .l()?;
    let iter = env
        .call_method(&threads, "iterator", "()Ljava/util/Iterator;", &[])?
        .l()?;

    let mut render_thread_loader: Option<GlobalRef> = None;

    while env.call_method(&iter, "hasNext", "()Z", &[])?.z()? {
        // Each thread spawns a handful of temporary JNI refs — scope them so a
        // process with many threads cannot overflow the local-reference table.
        let probe = env.with_local_frame(32, |env| -> anyhow::Result<LoaderProbe> {
            let thread = env
                .call_method(&iter, "next", "()Ljava/lang/Object;", &[])?
                .l()?;
            let loader = env
                .call_method(
                    &thread,
                    "getContextClassLoader",
                    "()Ljava/lang/ClassLoader;",
                    &[],
                )?
                .l()?;
            if loader.is_null() {
                return Ok(LoaderProbe::Unrelated);
            }
            probe_loader(env, &thread, &loader)
        })?;

        match probe {
            LoaderProbe::Live(loader) => return Ok(Some(loader)),
            LoaderProbe::RenderThread(loader) => render_thread_loader = Some(loader),
            LoaderProbe::Unrelated => {}
        }
    }

    Ok(render_thread_loader)
}

/// Classifies `loader` by asking it to load `Minecraft` and, if it can,
/// whether that class already holds a live game instance.
fn probe_loader(
    env: &mut JNIEnv,
    thread: &JObject,
    loader: &JObject,
) -> anyhow::Result<LoaderProbe> {
    let class = match load_class(env, loader, MINECRAFT_CLASS) {
        Some(class) => class,
        None => return Ok(LoaderProbe::Unrelated),
    };

    let instance = match env.call_static_method(&class, "getInstance", GET_INSTANCE_SIG, &[]) {
        Ok(value) => value.l().unwrap_or_else(|_| JObject::null()),
        Err(_) => {
            let _ = env.exception_clear();
            JObject::null()
        }
    };

    if !instance.is_null() {
        return Ok(LoaderProbe::Live(env.new_global_ref(loader)?));
    }
    if thread_name(env, thread)?.as_deref() == Some(RENDER_THREAD) {
        return Ok(LoaderProbe::RenderThread(env.new_global_ref(loader)?));
    }
    Ok(LoaderProbe::Unrelated)
}

/// Calls `loader.loadClass(binary_name)`, returning the class on success and
/// `None` (with the pending exception cleared) when the loader cannot find it.
fn load_class<'a>(env: &mut JNIEnv<'a>, loader: &JObject, binary_name: &str) -> Option<JClass<'a>> {
    let name: JObject = env.new_string(binary_name).ok()?.into();
    let result = env.call_method(
        loader,
        "loadClass",
        "(Ljava/lang/String;)Ljava/lang/Class;",
        &[JValue::Object(&name)],
    );
    match result.and_then(|value| value.l()) {
        Ok(class) if !class.is_null() => Some(JClass::from(class)),
        _ => {
            let _ = env.exception_clear();
            None
        }
    }
}

/// Reads `Thread.getName()`, returning `None` on any JNI error.
fn thread_name(env: &mut JNIEnv, thread: &JObject) -> anyhow::Result<Option<String>> {
    let name = env
        .call_method(thread, "getName", "()Ljava/lang/String;", &[])?
        .l()?;
    Ok(env
        .get_string(&JString::from(name))?
        .to_str()
        .ok()
        .map(str::to_owned))
}
