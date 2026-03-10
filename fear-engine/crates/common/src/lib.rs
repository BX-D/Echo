//! # Fear Engine — Common
//!
//! Shared types, error definitions, and configuration used across all Fear Engine crates.
//! This crate provides the foundational domain types (fear categories, game phases, session IDs),
//! a unified error type ([`FearEngineError`]), and application configuration.

pub mod config;
pub mod error;
pub mod types;

pub use error::{FearEngineError, Result};
