//! Output schema prompt (Layer 4) — tells the LLM what JSON shape to return.

/// The JSON schema instruction appended to every user message.
///
/// # Example
///
/// ```
/// use fear_engine_ai_integration::prompt::output::OUTPUT_SCHEMA;
/// assert!(OUTPUT_SCHEMA.contains("narrative"));
/// assert!(OUTPUT_SCHEMA.contains("choices"));
/// ```
pub const OUTPUT_SCHEMA: &str = r#"Respond with a JSON object matching this schema EXACTLY:
{
  "narrative": "string — the scene description text (150-300 words, 2-4 paragraphs)",
  "atmosphere": "string — one of: dread, tension, panic, calm, wrongness, isolation, paranoia",
  "sound_cue": "string or null — ambient sound description for the audio engine",
  "image_prompt": "string or null — detailed image generation prompt (only for key visual moments)",
  "choices": [
    {
      "id": "string — unique choice identifier (lowercase_snake_case)",
      "text": "string — what the player sees as the choice text",
      "approach": "string — one of: investigate, avoid, confront, flee, interact, wait",
      "fear_vector": "string — which fear this choice tests: claustrophobia, isolation, body_horror, stalking, loss_of_control, uncanny_valley, darkness, sound_based, doppelganger, abandonment"
    }
  ],
  "hidden_elements": ["string — subtle fear triggers embedded in the narrative (for analytics, not shown to player)"],
  "transcript_lines": ["string — only when medium is transcript; short transcript fragments"],
  "question_prompts": ["string — only when medium is questionnaire; 2-3 prompt blocks"],
  "archive_entries": ["string — only when medium is archive; 2-4 record entries"],
  "mirror_observations": ["string — only when medium is mirror/webcam; observed statements about the player"],
  "intensity": 0.0,
  "meta_break": null
}

Rules:
- "choices" must have 2-4 entries
- "intensity" is a float from 0.0 to 1.0
- Unused surface arrays should be empty arrays, not omitted
- "meta_break" is null unless specifically directed to break the fourth wall
- If meta_break is used: {"text": "string", "target": "title|overlay|whisper|glitch_text"}"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_output_schema_contains_all_fields() {
        assert!(OUTPUT_SCHEMA.contains("narrative"));
        assert!(OUTPUT_SCHEMA.contains("atmosphere"));
        assert!(OUTPUT_SCHEMA.contains("sound_cue"));
        assert!(OUTPUT_SCHEMA.contains("image_prompt"));
        assert!(OUTPUT_SCHEMA.contains("choices"));
        assert!(OUTPUT_SCHEMA.contains("hidden_elements"));
        assert!(OUTPUT_SCHEMA.contains("intensity"));
        assert!(OUTPUT_SCHEMA.contains("meta_break"));
    }

    #[test]
    fn test_output_schema_contains_valid_approaches() {
        assert!(OUTPUT_SCHEMA.contains("investigate"));
        assert!(OUTPUT_SCHEMA.contains("avoid"));
        assert!(OUTPUT_SCHEMA.contains("confront"));
        assert!(OUTPUT_SCHEMA.contains("flee"));
    }

    #[test]
    fn test_output_schema_contains_fear_types() {
        assert!(OUTPUT_SCHEMA.contains("claustrophobia"));
        assert!(OUTPUT_SCHEMA.contains("darkness"));
        assert!(OUTPUT_SCHEMA.contains("doppelganger"));
    }
}
