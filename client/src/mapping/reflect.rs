//! Runtime method discovery for unobfuscated Minecraft builds (26.1+).
//!
//! Without an obfuscation map, class and method *names* are already the real
//! ones — but JNI still needs each method's **signature** to make a call.
//! This module reflects a class once through `java.lang.Class` and returns a
//! [`MinecraftClass`] with identity names and reflected signatures, which the
//! rest of the mapping layer then treats exactly like a JSON-parsed entry.

use crate::client::DarkClient;
use crate::mapping::class::{Method, MinecraftClass};
use jni::objects::{JObject, JObjectArray, JString};
use jni::JNIEnv;
use std::collections::HashMap;

/// Reflects every method declared on — or inherited as public by —
/// `class_name`, returning it as a [`MinecraftClass`].
pub fn reflect_class(class_name: &str) -> anyhow::Result<MinecraftClass> {
    let mut env = DarkClient::instance().get_env()?;

    let jclass: JObject = env
        .find_class(class_name)
        .map_err(|_| {
            let _ = env.exception_clear();
            anyhow::anyhow!("Class {} not found at runtime", class_name)
        })?
        .into();

    let mut methods: HashMap<String, Vec<Method>> = HashMap::new();

    // `getMethods` covers inherited public methods; `getDeclaredMethods`
    // covers everything declared on this class, public or not.
    for accessor in ["getMethods", "getDeclaredMethods"] {
        collect_methods(&mut env, &jclass, accessor, &mut methods)?;
    }

    Ok(MinecraftClass::from_reflection(class_name.to_owned(), methods))
}

/// Calls `accessor` (a `Method[]`-returning method of `Class`) and folds every
/// result into `out`, de-duplicating overloads by signature.
fn collect_methods(
    env: &mut JNIEnv,
    jclass: &JObject,
    accessor: &str,
    out: &mut HashMap<String, Vec<Method>>,
) -> anyhow::Result<()> {
    let array = env
        .call_method(jclass, accessor, "()[Ljava/lang/reflect/Method;", &[])?
        .l()?;
    let array = JObjectArray::from(array);
    let count = env.get_array_length(&array)?;

    for index in 0..count {
        // Each method spawns several temporary JNI refs — scope them so the
        // local-reference table cannot overflow on large classes.
        let (name, signature) = env.with_local_frame(64, |env| -> anyhow::Result<_> {
            let method = env.get_object_array_element(&array, index)?;
            describe_method(env, &method)
        })?;

        let overloads = out.entry(name.clone()).or_default();
        if !overloads.iter().any(|m| m.signature == signature) {
            overloads.push(Method { name, signature });
        }
    }
    Ok(())
}

/// Reads a `java.lang.reflect.Method` into its name and JNI signature.
fn describe_method(env: &mut JNIEnv, method: &JObject) -> anyhow::Result<(String, String)> {
    let name = call_string(env, method, "getName")?;

    let params = env
        .call_method(method, "getParameterTypes", "()[Ljava/lang/Class;", &[])?
        .l()?;
    let params = JObjectArray::from(params);
    let param_count = env.get_array_length(&params)?;

    let mut signature = String::from("(");
    for index in 0..param_count {
        let param = env.get_object_array_element(&params, index)?;
        signature.push_str(&type_descriptor(env, &param)?);
    }
    signature.push(')');

    let return_type = env
        .call_method(method, "getReturnType", "()Ljava/lang/Class;", &[])?
        .l()?;
    signature.push_str(&type_descriptor(env, &return_type)?);

    Ok((name, signature))
}

/// Builds the JNI type descriptor of a `java.lang.Class` instance.
fn type_descriptor(env: &mut JNIEnv, class: &JObject) -> anyhow::Result<String> {
    let name = call_string(env, class, "getName")?;
    Ok(match name.as_str() {
        "boolean" => "Z".to_owned(),
        "byte" => "B".to_owned(),
        "char" => "C".to_owned(),
        "short" => "S".to_owned(),
        "int" => "I".to_owned(),
        "long" => "J".to_owned(),
        "float" => "F".to_owned(),
        "double" => "D".to_owned(),
        "void" => "V".to_owned(),
        // Array classes already report a descriptor, only dotted.
        array if array.starts_with('[') => array.replace('.', "/"),
        object => format!("L{};", object.replace('.', "/")),
    })
}

/// Calls a no-argument `String`-returning method and reads the result.
fn call_string(env: &mut JNIEnv, obj: &JObject, method: &str) -> anyhow::Result<String> {
    let value = env
        .call_method(obj, method, "()Ljava/lang/String;", &[])?
        .l()?;
    Ok(env.get_string(&JString::from(value))?.to_str()?.to_owned())
}
