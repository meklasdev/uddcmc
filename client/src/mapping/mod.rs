use crate::client::DarkClient;
use crate::mapping::class::MinecraftClass;
pub use crate::mapping::class_type::MinecraftClassType;
use crate::mapping::client::minecraft::Minecraft;
use crate::mapping::minecraft_version::MinecraftVersion;
use jni::objects::{GlobalRef, JObject, JString, JValue, JValueOwned};
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
                let class = Arc::new(reflect::reflect_class(name)?);
                self.classes
                    .write()
                    .unwrap()
                    .insert(name.to_owned(), Arc::clone(&class));
                Ok(class)
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
        let jclass = match env.find_class(&class.name) {
            Ok(jclass) => jclass,
            Err(_) => {
                let _ = env.exception_clear();
                return Err(anyhow::anyhow!(
                    "Class {} ({}) not found",
                    class_type.get_name(),
                    class.name
                ));
            }
        };
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
        match env.call_method(instance, &method.name, &method.signature, args) {
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

    pub fn get_static_field(
        &'_ self,
        class_type: MinecraftClassType,
        field_name: &str,
        field_type: FieldType,
    ) -> anyhow::Result<JValueOwned<'_>> {
        let mut env = self.get_env()?;

        let class_name = self.runtime_class_name(class_type)?;
        let jclass = match env.find_class(&class_name) {
            Ok(jclass) => jclass,
            Err(_) => {
                let _ = env.exception_clear();
                return Err(anyhow::anyhow!(
                    "Class {} ({}) not found",
                    class_type.get_name(),
                    class_name
                ));
            }
        };
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
        let jclass = match env.find_class(&class_name) {
            Ok(jclass) => jclass,
            Err(_) => {
                let _ = env.exception_clear();
                return Err(anyhow::anyhow!(
                    "Class {} ({}) not found",
                    class_type.get_name(),
                    class_name
                ));
            }
        };

        Ok(env.is_instance_of(instance, jclass)?)
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
