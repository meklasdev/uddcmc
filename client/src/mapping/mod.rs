use crate::client::DarkClient;
use crate::mapping::class::{Method, MethodHandle, MinecraftClass};
pub use crate::mapping::class_type::MinecraftClassType;
use crate::mapping::client::minecraft::Minecraft;
use crate::mapping::minecraft_version::MinecraftVersion;
use jni::objects::{GlobalRef, JClass, JMethodID, JObject, JString, JValue, JValueOwned};
use jni::signature::{Primitive, ReturnType};
use jni::sys::jvalue;
use jni::JNIEnv;
use log::{error, info};
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

pub mod class;
pub mod class_type;
pub mod client;
pub mod entity;
pub mod java;
mod method;
mod minecraft_version;
mod reflect;

pub trait GameContext {
    fn minecraft(&self) -> &'static Minecraft {
        Minecraft::instance()
    }

    fn mapping(&self) -> &'static Mapping {
        self.minecraft().get_mapping()
    }
}

/// On-disk JSON shape. Only obfuscated builds ship one of these.
#[derive(Debug, Deserialize)]
struct MappingFile {
    version: MinecraftVersion,
    classes: HashMap<String, MinecraftClass>,
}

/// How class / method / field names are resolved to their runtime form.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    /// Obfuscated build: translate names through the bundled Mojmap JSON.
    Obfuscated,
    /// Unobfuscated build (Minecraft 26.1+): names are already real, method
    /// signatures are discovered lazily via JNI reflection.
    Reflected,
}

/// Bridges deobfuscated (Mojmap) names to whatever the running JVM actually
/// uses, transparently for both obfuscated and unobfuscated Minecraft.
#[derive(Debug)]
pub struct Mapping {
    mode: Mode,
    version: MinecraftVersion,
    /// In obfuscated mode every class is present up-front; in reflected mode
    /// classes are discovered and cached on first use.
    classes: RwLock<HashMap<String, Arc<MinecraftClass>>>,
    /// The class loader that loaded Minecraft, captured the first time a class
    /// resolves. `JNIEnv::find_class` is classloader-sensitive and only works
    /// from threads with a Minecraft Java frame on the stack; routing every
    /// later lookup through this loader makes resolution thread-independent.
    class_loader: RwLock<Option<GlobalRef>>,
    /// Cache of resolved JVM classes — and known-missing ones (`None`) — keyed
    /// by JNI name, so a class is searched for at most once.
    class_handles: RwLock<HashMap<String, Option<GlobalRef>>>,
}

#[allow(dead_code)]
pub enum FieldType<'local> {
    Boolean,
    Byte,
    Char,
    Short,
    Int,
    Long,
    Float,
    Double,
    String,
    Object(MinecraftClassType, &'local Mapping),
}

impl FieldType<'_> {
    pub fn get_signature(&self) -> anyhow::Result<String> {
        Ok(match self {
            FieldType::Boolean => String::from("Z"),
            FieldType::Byte => String::from("B"),
            FieldType::Char => String::from("C"),
            FieldType::Short => String::from("S"),
            FieldType::Int => String::from("I"),
            FieldType::Long => String::from("J"),
            FieldType::Float => String::from("F"),
            FieldType::Double => String::from("D"),
            FieldType::String => String::from("Ljava/lang/String;"),
            FieldType::Object(class_type, mapping) => {
                format!("L{};", mapping.runtime_class_name(*class_type)?)
            }
        })
    }
}

/// Probes whether the running Minecraft is unobfuscated.
///
/// In an unobfuscated build the real Mojmap class name resolves directly; in
/// an obfuscated build that class only exists under its scrambled name, so the
/// lookup fails (and the resulting pending exception is cleared).
fn is_unobfuscated() -> bool {
    match DarkClient::instance().get_env() {
        Ok(mut env) => {
            let found = env.find_class("net/minecraft/client/Minecraft").is_ok();
            if !found {
                let _ = env.exception_clear();
            }
            found
        }
        Err(_) => false,
    }
}

#[allow(dead_code)]
impl Mapping {
    pub fn new() -> anyhow::Result<Mapping> {
        if is_unobfuscated() {
            info!("Unobfuscated Minecraft detected — using runtime reflection mapping");
            return Ok(Mapping {
                mode: Mode::Reflected,
                version: MinecraftVersion::LATEST,
                classes: RwLock::new(HashMap::new()),
                class_loader: RwLock::new(None),
                class_handles: RwLock::new(HashMap::new()),
            });
        }

        info!("Obfuscated Minecraft detected — using bundled Mojmap mappings");
        let mut file: MappingFile = serde_json::from_str(include_str!("../../../mappings.json"))?;

        // Standard `java.*` classes are never obfuscated; they live in a
        // small hand-written supplement.
        let java_classes: HashMap<String, MinecraftClass> =
            serde_json::from_str(include_str!("../../../java_mappings.json"))?;
        file.classes.extend(java_classes);

        let classes = file
            .classes
            .into_iter()
            .map(|(name, class)| (name, Arc::new(class)))
            .collect();

        Ok(Mapping {
            mode: Mode::Obfuscated,
            version: file.version,
            classes: RwLock::new(classes),
            class_loader: RwLock::new(None),
            class_handles: RwLock::new(HashMap::new()),
        })
    }

    fn get_client(&self) -> &DarkClient {
        DarkClient::instance()
    }

    pub fn get_env(&'_ self) -> anyhow::Result<JNIEnv<'_>> {
        Ok(self.get_client().get_env()?)
    }

    pub fn get_version(&self) -> MinecraftVersion {
        self.version
    }

    /// Resolves a mapped class by its deobfuscated name. In reflected mode the
    /// class is reflected from the JVM and cached on first request.
    pub fn get_class(&self, name: &str) -> anyhow::Result<Arc<MinecraftClass>> {
        if let Some(class) = self.classes.read().unwrap().get(name) {
            return Ok(Arc::clone(class));
        }

        match self.mode {
            Mode::Obfuscated => Err(anyhow::anyhow!("{} java class not found", name)),
            Mode::Reflected => {
                let class = Arc::new(reflect::reflect_class(self, name)?);
                self.classes
                    .write()
                    .unwrap()
                    .insert(name.to_owned(), Arc::clone(&class));
                Ok(class)
            }
        }
    }

    /// Resolves a JVM class by its JNI name, working from any thread.
    ///
    /// `JNIEnv::find_class` resolves against the class loader of the calling
    /// thread; on a thread with no Minecraft Java frame on its stack (the
    /// render thread, for instance) it cannot see `net.minecraft.*` classes.
    /// Once the Minecraft class loader has been captured every lookup goes
    /// through `ClassLoader.loadClass` on it instead.
    ///
    /// Results — hits *and* misses — are cached: a repeated lookup is one map
    /// read, and a missing class is never searched twice (a failed `loadClass`
    /// walks the whole classpath, ruinously slow to repeat per entity).
    pub(crate) fn resolve_class<'a>(
        &self,
        env: &mut JNIEnv<'a>,
        jni_name: &str,
    ) -> anyhow::Result<JClass<'a>> {
        if let Some(cached) = self.class_handles.read().unwrap().get(jni_name).cloned() {
            return match cached {
                Some(handle) => Ok(JClass::from(env.new_local_ref(handle.as_obj())?)),
                None => Err(anyhow::anyhow!("Class {} not present at runtime", jni_name)),
            };
        }

        let resolved = self.lookup_class(env, jni_name);
        let handle = match &resolved {
            Ok(jclass) => env.new_global_ref(jclass).ok(),
            Err(_) => None,
        };
        if handle.is_none() {
            log::warn!(
                "Mapping: class '{}' could not be resolved at runtime",
                jni_name
            );
        }
        self.class_handles
            .write()
            .unwrap()
            .insert(jni_name.to_owned(), handle);
        resolved
    }

    /// Looks a class up from scratch: through the captured Minecraft class
    /// loader if available, otherwise through `find_class`.
    fn lookup_class<'a>(
        &self,
        env: &mut JNIEnv<'a>,
        jni_name: &str,
    ) -> anyhow::Result<JClass<'a>> {
        if let Some(loader) = self.class_loader.read().unwrap().clone() {
            let binary_name = jni_name.replace('/', ".");
            let name: JObject = env.new_string(binary_name)?.into();
            return match env.call_method(
                loader.as_obj(),
                "loadClass",
                "(Ljava/lang/String;)Ljava/lang/Class;",
                &[JValue::Object(&name)],
            ) {
                Ok(value) => Ok(JClass::from(value.l()?)),
                Err(_) => {
                    let _ = env.exception_clear();
                    Err(anyhow::anyhow!("Class {} not found at runtime", jni_name))
                }
            };
        }

        // No loader captured yet: fall back to `find_class` and remember the
        // loader that resolved it for every later lookup.
        match env.find_class(jni_name) {
            Ok(jclass) => {
                self.capture_class_loader(env, &jclass);
                Ok(jclass)
            }
            Err(_) => {
                let _ = env.exception_clear();
                Err(anyhow::anyhow!("Class {} not found at runtime", jni_name))
            }
        }
    }

    /// Records the class loader of `jclass` as the Minecraft class loader.
    fn capture_class_loader(&self, env: &mut JNIEnv, jclass: &JClass) {
        if self.class_loader.read().unwrap().is_some() {
            return;
        }
        let loader = env.call_method(jclass, "getClassLoader", "()Ljava/lang/ClassLoader;", &[]);
        match loader.and_then(|value| value.l()) {
            Ok(obj) if !obj.is_null() => {
                if let Ok(global) = env.new_global_ref(obj) {
                    *self.class_loader.write().unwrap() = Some(global);
                }
            }
            _ => {
                let _ = env.exception_clear();
            }
        }
    }

    /// Runtime (JVM) name of a class.
    fn runtime_class_name(&self, class_type: MinecraftClassType) -> anyhow::Result<String> {
        match self.mode {
            Mode::Reflected => Ok(class_type.get_name().to_owned()),
            Mode::Obfuscated => Ok(self.get_class(class_type.get_name())?.name.clone()),
        }
    }

    /// Runtime (JVM) name of a field.
    fn runtime_field_name(
        &self,
        class_type: MinecraftClassType,
        field: &str,
    ) -> anyhow::Result<String> {
        match self.mode {
            Mode::Reflected => Ok(field.to_owned()),
            Mode::Obfuscated => Ok(self
                .get_class(class_type.get_name())?
                .get_field(field)?
                .name
                .clone()),
        }
    }

    /// Finds the deobfuscated name of a class given its obfuscated name.
    /// Only meaningful in obfuscated mode; used to prettify error messages.
    fn find_class_by_obfuscated_name(&self, obfuscated_name: &str) -> Option<String> {
        self.classes
            .read()
            .unwrap()
            .iter()
            .find(|(_, class)| class.name == obfuscated_name)
            .map(|(deobfuscated_name, _)| deobfuscated_name.clone())
    }

    fn translate_type_descriptor<'a>(&self, descriptor: &mut &'a str) -> String {
        let mut array_brackets = String::new();
        while descriptor.starts_with('[') {
            array_brackets.push_str("[]");
            *descriptor = &descriptor[1..];
        }

        let type_name = if let Some(stripped) = descriptor.strip_prefix('L') {
            if let Some(end_index) = stripped.find(';') {
                let obfuscated_name = &stripped[..end_index];
                let deobfuscated_name = self
                    .find_class_by_obfuscated_name(obfuscated_name)
                    .unwrap_or_else(|| obfuscated_name.to_owned());

                *descriptor = &stripped[end_index + 1..];
                deobfuscated_name
            } else {
                let rest = descriptor.to_string();
                *descriptor = "";
                rest
            }
        } else {
            let (primitive, rest) = descriptor.split_at(1);
            *descriptor = rest;
            match primitive {
                "Z" => "boolean".to_string(),
                "B" => "byte".to_string(),
                "C" => "char".to_string(),
                "S" => "short".to_string(),
                "I" => "int".to_string(),
                "J" => "long".to_string(),
                "F" => "float".to_string(),
                "D" => "double".to_string(),
                "V" => "void".to_string(),
                _ => primitive.to_string(),
            }
        };

        format!("{}{}", type_name, array_brackets)
    }

    fn translate_signature(&self, signature: &str) -> String {
        if let (Some(params_start), Some(params_end)) = (signature.find('('), signature.find(')')) {
            let mut params_str = &signature[params_start + 1..params_end];
            let mut return_type_str = &signature[params_end + 1..];

            let mut translated_params = Vec::new();
            while !params_str.is_empty() {
                translated_params.push(self.translate_type_descriptor(&mut params_str));
            }

            let translated_return = self.translate_type_descriptor(&mut return_type_str);

            format!("({}) -> {}", translated_params.join(", "), translated_return)
        } else {
            signature.to_string()
        }
    }

    pub fn call_static_method(
        &'_ self,
        class_type: MinecraftClassType,
        method_name: &str,
        args: &[JValue],
    ) -> anyhow::Result<JValueOwned<'_>> {
        let mut env = self.get_env()?;

        let class = self.get_class(class_type.get_name())?;
        let jclass = self.resolve_class(&mut env, &class.name)?;
        let method = class.get_method_by_args(method_name, args)?;
        match env.call_static_method(jclass, &method.name, &method.signature, args) {
            Ok(value) => Ok(value),
            Err(_) => {
                let _ = env.exception_clear();
                let translated_signature = self.translate_signature(&method.signature);
                Err(anyhow::anyhow!(
                    "Error calling static method {} ({}) in class {} ({}) with signature {} ({})",
                    method_name,
                    method.name,
                    class_type.get_name(),
                    class.name,
                    translated_signature,
                    method.signature
                ))
            }
        }
    }

    pub fn call_method(
        &'_ self,
        class_type: MinecraftClassType,
        instance: &JObject,
        method_name: &str,
        args: &[JValue],
    ) -> anyhow::Result<JValueOwned<'_>> {
        let mut env = self.get_env()?;

        let class = self.get_class(class_type.get_name())?;
        let method = class.get_method_by_args(method_name, args)?;

        // `call_method_unchecked` does not validate arity — so guard it here,
        // since `get_method_by_args` skips that check for single-overload
        // methods.
        if signature_arg_count(&method.signature) != args.len() {
            return Err(anyhow::anyhow!(
                "Argument count mismatch calling {} ({}) — signature {}",
                method_name,
                method.name,
                method.signature
            ));
        }

        let method_id = self.resolve_method_id(&mut env, class_type, method)?;
        let return_type = parse_return_type(&method.signature);
        let jni_args: Vec<jvalue> = args.iter().map(|arg| arg.as_jni()).collect();

        match unsafe { env.call_method_unchecked(instance, method_id, return_type, &jni_args) } {
            Ok(value) => Ok(value),
            Err(_) => {
                let _ = env.exception_clear();
                let translated_signature = self.translate_signature(&method.signature);
                Err(anyhow::anyhow!(
                    "Error calling method {} ({}) in class {} ({}) with signature {} ({})",
                    method_name,
                    method.name,
                    class_type.get_name(),
                    class.name,
                    translated_signature,
                    method.signature
                ))
            }
        }
    }

    /// Resolves the JNI method id for `method`, caching it on the `Method` so
    /// the costly name+signature lookup happens only once per method.
    fn resolve_method_id(
        &self,
        env: &mut JNIEnv,
        class_type: MinecraftClassType,
        method: &Method,
    ) -> anyhow::Result<JMethodID> {
        if let Some(handle) = method.id.get() {
            return Ok(unsafe { JMethodID::from_raw(handle.0) });
        }

        let class_name = self.runtime_class_name(class_type)?;
        let jclass = self.resolve_class(env, &class_name)?;
        let id = match env.get_method_id(&jclass, &method.name, &method.signature) {
            Ok(id) => id,
            Err(_) => {
                let _ = env.exception_clear();
                return Err(anyhow::anyhow!(
                    "Method {} {} not found on class {}",
                    method.name,
                    method.signature,
                    class_name
                ));
            }
        };
        let _ = method.id.set(MethodHandle(id.into_raw()));
        Ok(id)
    }

    pub fn get_static_field(
        &'_ self,
        class_type: MinecraftClassType,
        field_name: &str,
        field_type: FieldType,
    ) -> anyhow::Result<JValueOwned<'_>> {
        let mut env = self.get_env()?;

        let class_name = self.runtime_class_name(class_type)?;
        let jclass = self.resolve_class(&mut env, &class_name)?;
        let runtime_field = self.runtime_field_name(class_type, field_name)?;
        match env.get_static_field(jclass, &runtime_field, field_type.get_signature()?) {
            Ok(value) => Ok(value),
            Err(_) => {
                let _ = env.exception_clear();
                Err(anyhow::anyhow!(
                    "Error getting static field {} ({}) from class {}",
                    field_name,
                    runtime_field,
                    class_type.get_name()
                ))
            }
        }
    }

    pub fn get_field(
        &'_ self,
        class_type: MinecraftClassType,
        instance: &JObject,
        field_name: &str,
        field_type: FieldType,
    ) -> anyhow::Result<JValueOwned<'_>> {
        let mut env = self.get_env()?;

        let runtime_field = self.runtime_field_name(class_type, field_name)?;
        match env.get_field(instance, &runtime_field, field_type.get_signature()?) {
            Ok(value) => Ok(value),
            Err(_) => {
                let _ = env.exception_clear();
                Err(anyhow::anyhow!(
                    "Error getting field {} ({}) from class {}",
                    field_name,
                    runtime_field,
                    class_type.get_name()
                ))
            }
        }
    }

    pub fn set_field(
        &self,
        class_type: MinecraftClassType,
        instance: &JObject,
        field_name: &str,
        field_type: FieldType,
        value: JValue,
    ) -> anyhow::Result<()> {
        let mut env = self.get_env()?;

        let runtime_field = self.runtime_field_name(class_type, field_name)?;
        match env.set_field(instance, &runtime_field, field_type.get_signature()?, value) {
            Ok(_) => Ok(()),
            Err(_) => {
                let _ = env.exception_clear();
                Err(anyhow::anyhow!(
                    "Error setting field {} ({}) in class {}",
                    field_name,
                    runtime_field,
                    class_type.get_name()
                ))
            }
        }
    }

    pub fn new_global_ref(&self, obj: JObject) -> anyhow::Result<GlobalRef> {
        let env = self.get_env()?;
        Ok(env.new_global_ref(obj)?)
    }

    pub fn get_string(&self, obj: JObject) -> anyhow::Result<String> {
        let env = self.get_env()?;
        let jstring = JString::from(obj);
        unsafe {
            let value = env
                .get_string_unchecked(jstring.as_ref())?
                .to_str()?
                .to_string();
            Ok(value)
        }
    }

    pub fn is_instance_of(
        &self,
        class_type: MinecraftClassType,
        instance: &JObject,
    ) -> anyhow::Result<bool> {
        let mut env = self.get_env()?;
        let class_name = self.runtime_class_name(class_type)?;
        let jclass = self.resolve_class(&mut env, &class_name)?;
        Ok(env.is_instance_of(instance, &jclass)?)
    }
}

impl Default for Mapping {
    fn default() -> Self {
        Self::new().unwrap_or_else(|_| {
            error!("Failed to load mappings");
            panic!("Failed to load mappings");
        })
    }
}

/// Number of parameters in a JNI method signature, e.g. `(ILjava/lang/String;)V`
/// has 2. Used to guard the unchecked call path against arity mismatches.
fn signature_arg_count(signature: &str) -> usize {
    let params = match (signature.find('('), signature.find(')')) {
        (Some(open), Some(close)) if open < close => &signature[open + 1..close],
        // Unparseable: return a count that can never match a real call.
        _ => return usize::MAX,
    };

    let mut count = 0;
    let mut chars = params.chars();
    while let Some(ch) = chars.next() {
        match ch {
            // Array prefix — the descriptor it belongs to is counted next.
            '[' => continue,
            // Object descriptor runs until its terminating ';'.
            'L' => {
                for inner in chars.by_ref() {
                    if inner == ';' {
                        break;
                    }
                }
                count += 1;
            }
            // Any primitive.
            _ => count += 1,
        }
    }
    count
}

/// Maps the return descriptor of a JNI signature to a [`ReturnType`].
fn parse_return_type(signature: &str) -> ReturnType {
    let return_descriptor = signature.rsplit(')').next().unwrap_or("V");
    match return_descriptor.chars().next() {
        Some('Z') => ReturnType::Primitive(Primitive::Boolean),
        Some('B') => ReturnType::Primitive(Primitive::Byte),
        Some('C') => ReturnType::Primitive(Primitive::Char),
        Some('S') => ReturnType::Primitive(Primitive::Short),
        Some('I') => ReturnType::Primitive(Primitive::Int),
        Some('J') => ReturnType::Primitive(Primitive::Long),
        Some('F') => ReturnType::Primitive(Primitive::Float),
        Some('D') => ReturnType::Primitive(Primitive::Double),
        Some('[') => ReturnType::Array,
        Some('L') => ReturnType::Object,
        _ => ReturnType::Primitive(Primitive::Void),
    }
}
