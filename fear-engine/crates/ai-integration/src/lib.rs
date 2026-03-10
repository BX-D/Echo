//! # Fear Engine — AI Integration
//!
//! LLM and image generation integration for the Fear Engine. Provides an Anthropic Claude
//! API client for narrative generation, a prompt engineering system with dynamic context
//! building, an image generation client (Stability AI / Replicate), and response caching.

pub mod cache;
pub mod claude_client;
pub mod image;
pub mod narrative;
pub mod prompt;
