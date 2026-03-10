//! Base narrative content for the abandoned hospital scenario.
//!
//! Provides calibration scenes, fear-probe scenes, and AI-customisable
//! templates.  Call [`build_hospital_graph`] to get a fully wired [`SceneGraph`].

pub mod hospital;
pub mod templates;

pub use hospital::build_hospital_graph;
