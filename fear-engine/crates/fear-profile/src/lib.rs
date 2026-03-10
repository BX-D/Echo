//! # Fear Engine — Fear Profile
//!
//! The fear analysis engine. Processes player behavior events (typing patterns, hesitation,
//! choice timing) through a Bayesian scoring system to build a real-time fear profile.
//! Includes the adaptation strategy engine that decides how to escalate horror content
//! based on the player's confirmed fears.

pub mod adaptation;
pub mod analyzer;
pub mod behavior;
pub mod profile;
pub mod scorer;
