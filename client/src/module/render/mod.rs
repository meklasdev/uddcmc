//! Render-category modules.
//!
//! These modules are pure configuration holders: the actual drawing is done
//! every frame by [`crate::graphic::esp`], which reads their enabled state and
//! settings. Keeping the JNI/render work in one place is what lets the ESP run
//! without costing frame rate.

pub mod chest_esp;
pub mod mob_esp;
pub mod player_esp;
pub mod tracers;
