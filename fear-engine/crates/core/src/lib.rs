//! # Fear Engine — Core
//!
//! The game engine for Fear Engine. Contains the scene data model and scene graph,
//! the game state finite state machine, a publish/subscribe event bus, scene transition
//! management, and the base narrative content (hospital scenario).

pub mod event_bus;
pub mod narrative;
pub mod scene;
pub mod scene_manager;
pub mod state_machine;
