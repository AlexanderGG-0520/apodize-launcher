//! Core application and domain logic for Apodize Launcher.
//!
//! This crate intentionally contains no terminal rendering or user-facing output.

pub mod error;
pub mod instance;

pub use error::InstanceError;
