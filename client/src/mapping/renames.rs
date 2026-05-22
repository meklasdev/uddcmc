//! Registry of Mojmap symbols — classes, methods, fields — that Mojang renamed
//! across Minecraft versions.
//!
//! The rest of the codebase always uses a symbol's **latest** name (its
//! `canonical` name). This table translates that name back for older builds, so
//! a single build keeps working on every supported Minecraft version.
//!
//! See `docs_internal/versioned-names.md` for the design rationale.

use crate::mapping::minecraft_version::MinecraftVersion;

/// Shorthand for a [`MinecraftVersion`] literal in the [`RENAMES`] table.
const fn v(major: u32, minor: u32, patch: u32) -> MinecraftVersion {
    MinecraftVersion::new(major, minor, patch)
}

/// A Mojmap symbol — class, method or field — that Mojang renamed over time.
struct Rename {
    /// Owning class (its canonical name) for a method/field; `None` for a class.
    owner: Option<&'static str>,
    /// The current name — what the codebase writes.
    canonical: &'static str,
    /// Older names, **oldest first**. Each `(replaced_in, name)` means `name`
    /// was used up to — but not including — version `replaced_in`.
    legacy: &'static [(MinecraftVersion, &'static str)],
}

/// Every symbol whose name changed across the supported Minecraft versions.
static RENAMES: &[Rename] = &[
    // `Window.getWindow()` -> `Window.handle()`
    Rename {
        owner: Some("com/mojang/blaze3d/platform/Window"),
        canonical: "handle",
        legacy: &[(v(1, 21, 9), "getWindow")],
    },
];

/// The name `canonical` had on Minecraft `version` — `canonical` itself unless
/// an older name applies.
///
/// `owner` is the canonical owning-class name for a method or field lookup, and
/// `None` for a class lookup; it keeps a method/field rename scoped to its own
/// class.
pub fn resolve<'a>(owner: Option<&str>, canonical: &'a str, version: MinecraftVersion) -> &'a str {
    // Reflected builds always run the latest Minecraft: nothing to translate.
    if version == MinecraftVersion::LATEST {
        return canonical;
    }
    let Some(entry) = RENAMES
        .iter()
        .find(|rename| rename.canonical == canonical && rename.owner == owner)
    else {
        return canonical;
    };
    // `legacy` is oldest-first: the first range that contains `version` wins.
    for &(replaced_in, old_name) in entry.legacy {
        if version < replaced_in {
            return old_name;
        }
    }
    canonical
}
