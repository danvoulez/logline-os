//! Core shared library for the LogLine ecosystem.
//!
//! This crate exposes reusable primitives that the microservices
//! depend on: identity representation, common errors, configuration
//! loading, database abstractions, websocket helpers and logging setup.

pub mod config;
pub mod db;
pub mod errors;
pub mod identity;
pub mod logging;
pub mod serde_utils;
pub mod websocket;

pub use errors::{LogLineError, Result as CoreResult};
pub use identity::{LogLineID, LogLineIDBuilder, LogLineKeyPair};
