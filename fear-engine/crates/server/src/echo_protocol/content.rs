use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use fear_engine_common::types::{
    ConversationGuide, FlashEvent, HiddenClueDefinition, InvestigationItem, InvestigationKind,
    SceneMode, ScriptBlockKind, StoryChapter, StoryEnding,
};
use fear_engine_common::{FearEngineError, Result};

#[derive(Debug, Clone)]
pub struct EchoContent {
    pub package_title: String,
    pub case_title: String,
    pub prompt_architecture: String,
    pub attribute_summary: Vec<String>,
    pub first_scene_id: String,
    pub scene_order: Vec<String>,
    pub scenes: HashMap<String, SceneDefinition>,
    pub hidden_clues: HashMap<String, HiddenClueDefinition>,
}

#[derive(Debug, Clone)]
pub struct SceneDefinition {
    pub id: String,
    pub chapter: StoryChapter,
    pub title: String,
    pub scene_mode: SceneMode,
    pub status_line: String,
    pub input_placeholder: String,
    pub entry_condition: Option<ConditionTemplate>,
    pub next_scene_id: Option<String>,
    pub blocks: Vec<SceneNode>,
}

#[derive(Debug, Clone)]
pub enum SceneNode {
    Block(BlockTemplate),
    Choice(ChoiceDefinition),
    ConversationGuide(ConversationGuideSpec),
    Effect(EffectTemplate),
    Flash(FlashEvent),
}

#[derive(Debug, Clone)]
pub struct BlockTemplate {
    pub id: String,
    pub kind: ScriptBlockKind,
    pub speaker: Option<String>,
    pub title: Option<String>,
    pub text: String,
    pub code_block: bool,
    pub condition: Option<ConditionTemplate>,
    pub document: Option<InvestigationItem>,
}

#[derive(Debug, Clone)]
pub struct ChoiceDefinition {
    pub id: String,
    pub prompt: String,
    pub options: Vec<ChoiceOptionDefinition>,
    pub condition: Option<ConditionTemplate>,
}

#[derive(Debug, Clone)]
pub struct ChoiceOptionDefinition {
    pub id: String,
    pub code: Option<String>,
    pub label: String,
    pub effects_summary: Vec<String>,
    pub stat_delta: StatDelta,
    pub disabled: bool,
}

#[derive(Debug, Clone)]
pub struct ConversationGuideSpec {
    pub id: String,
    pub guide: ConversationGuide,
    pub condition: Option<ConditionTemplate>,
    pub fallback_rules: Vec<GuideFallbackRule>,
    pub high_trust_reply: Option<String>,
    pub low_trust_reply: Option<String>,
    pub default_reply: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ConditionTemplate {
    pub raw: String,
    pub scope_choice_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct GuideFallbackRule {
    pub label: String,
    pub keywords: Vec<String>,
    pub response: String,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct StatDelta {
    pub sanity: i32,
    pub trust: i32,
    pub awakening: i32,
}

impl EchoContent {
    pub fn load() -> Result<Self> {
        let path = script_path();
        let source = fs::read_to_string(&path).map_err(|error| {
            FearEngineError::Configuration(format!(
                "failed to read Echo Protocol complete script at {}: {error}",
                path.display()
            ))
        })?;
        Self::from_markdown(&source)
    }

    pub fn from_markdown(source: &str) -> Result<Self> {
        let lines = source.lines().map(ToOwned::to_owned).collect::<Vec<_>>();
        let mut package_title = "Echo Protocol".to_string();
        let case_title = "Nexus AI Labs / Echo Audit".to_string();
        let mut prompt_architecture = String::new();
        let mut attribute_summary = Vec::new();
        let mut hidden_clues = HashMap::new();
        let mut scenes = HashMap::new();
        let mut scene_order = Vec::new();

        let mut current_chapter = StoryChapter::Onboarding;
        let mut current_scene: Option<SceneDefinition> = None;
        let mut current_condition: Option<ConditionTemplate> = None;
        let mut last_choice_id: Option<String> = None;
        let mut in_appendix_prompt = false;
        let mut in_appendix_clues = false;
        let mut in_appendix_attributes = false;
        let mut appendix_code = Vec::new();

        let mut i = 0usize;
        while i < lines.len() {
            let line = lines[i].trim_end().to_string();

            if line.starts_with("# ECHO PROTOCOL") {
                package_title = "Echo Protocol".into();
                i += 1;
                continue;
            }

            if line.starts_with("# APPENDIX A:") {
                flush_scene(&mut current_scene, &mut scenes, &mut scene_order)?;
                in_appendix_prompt = true;
                in_appendix_clues = false;
                in_appendix_attributes = false;
                appendix_code.clear();
                i += 1;
                continue;
            }
            if line.starts_with("# APPENDIX B:") {
                flush_scene(&mut current_scene, &mut scenes, &mut scene_order)?;
                in_appendix_prompt = false;
                in_appendix_clues = true;
                in_appendix_attributes = false;
                appendix_code.clear();
                i += 1;
                continue;
            }
            if line.starts_with("# APPENDIX C:") {
                flush_scene(&mut current_scene, &mut scenes, &mut scene_order)?;
                in_appendix_prompt = false;
                in_appendix_clues = false;
                in_appendix_attributes = true;
                appendix_code.clear();
                i += 1;
                continue;
            }

            if in_appendix_prompt || in_appendix_clues || in_appendix_attributes {
                if line.trim() == "```" {
                    let (code, next_index) = collect_fenced_code(&lines, i)?;
                    appendix_code = code;
                    i = next_index;
                    if in_appendix_prompt {
                        prompt_architecture = appendix_code.join("\n");
                    } else if in_appendix_clues {
                        hidden_clues = parse_hidden_clues(&appendix_code);
                    } else if in_appendix_attributes {
                        attribute_summary = appendix_code
                            .iter()
                            .map(|entry| entry.trim().to_string())
                            .filter(|entry| !entry.is_empty())
                            .collect();
                    }
                    continue;
                }
                i += 1;
                continue;
            }

            if let Some(chapter) = parse_chapter_heading(&line) {
                flush_scene(&mut current_scene, &mut scenes, &mut scene_order)?;
                current_chapter = chapter;
                current_condition = None;
                i += 1;
                continue;
            }

            if let Some(scene_info) = parse_scene_heading(&line, current_chapter) {
                flush_scene(&mut current_scene, &mut scenes, &mut scene_order)?;
                current_condition = None;
                last_choice_id = None;
                current_scene = Some(SceneDefinition {
                    id: scene_info.id.clone(),
                    chapter: scene_info.chapter,
                    title: scene_info.title,
                    scene_mode: scene_mode_for(&scene_info.id),
                    status_line: status_line_for(&scene_info.id),
                    input_placeholder: input_placeholder_for(&scene_info.id),
                    entry_condition: None,
                    next_scene_id: None,
                    blocks: Vec::new(),
                });
                i += 1;
                continue;
            }

            if line.starts_with("## ENDING ") {
                flush_scene(&mut current_scene, &mut scenes, &mut scene_order)?;
                current_condition = None;
                last_choice_id = None;
                current_scene = Some(SceneDefinition {
                    id: ending_id_for(&line),
                    chapter: StoryChapter::Ending,
                    title: line.trim_start_matches("## ").trim().to_string(),
                    scene_mode: scene_mode_for(&ending_id_for(&line)),
                    status_line: "Ending".into(),
                    input_placeholder: "".into(),
                    entry_condition: None,
                    next_scene_id: None,
                    blocks: Vec::new(),
                });
                i += 1;
                continue;
            }

            if current_scene.is_none() {
                if line.starts_with("# PROLOGUE:") {
                    current_scene = Some(SceneDefinition {
                        id: "prologue_boot_sequence".into(),
                        chapter: StoryChapter::Onboarding,
                        title: "Prologue: Boot Sequence".into(),
                        scene_mode: SceneMode::Prologue,
                        status_line: "Boot Sequence".into(),
                        input_placeholder: "".into(),
                        entry_condition: None,
                        next_scene_id: None,
                        blocks: Vec::new(),
                    });
                }
                i += 1;
                continue;
            }

            if let Some(tag) = parse_tag_header(&line) {
                match tag.kind.as_str() {
                    "CONDITION" => {
                        let template = ConditionTemplate {
                            raw: tag.label.unwrap_or_default(),
                            scope_choice_id: last_choice_id.clone(),
                        };
                        if let Some(scene) = current_scene.as_mut() {
                            if scene.chapter == StoryChapter::Ending
                                && scene.entry_condition.is_none()
                                && scene.blocks.is_empty()
                            {
                                scene.entry_condition = Some(template.clone());
                            }
                        }
                        current_condition = Some(template);
                        i += 1;
                    }
                    "CHOICE" => {
                        let (choice, next_index) =
                            parse_choice_block(&lines, i, current_condition.clone())?;
                        last_choice_id = Some(choice.id.clone());
                        if let Some(scene) = current_scene.as_mut() {
                            scene.blocks.push(SceneNode::Choice(choice));
                        }
                        current_condition = None;
                        i = next_index;
                    }
                    "ECHO_FREE_GUIDE" => {
                        let (guide, next_index) = parse_conversation_guide(
                            &lines,
                            i,
                            tag.label.unwrap_or_default(),
                            current_condition.clone(),
                        )?;
                        if let Some(scene) = current_scene.as_mut() {
                            scene.blocks.push(SceneNode::ConversationGuide(guide));
                        }
                        current_condition = None;
                        i = next_index;
                    }
                    _ => {
                        let (blocks, next_index) =
                            parse_content_block(&lines, i, tag, current_condition.clone())?;
                        if let Some(scene) = current_scene.as_mut() {
                            scene.blocks.extend(blocks);
                        }
                        current_condition = None;
                        i = next_index;
                    }
                }
                continue;
            }

            if let Some(effect) = parse_effect_line(&line, current_condition.clone()) {
                if let Some(scene) = current_scene.as_mut() {
                    scene.blocks.push(SceneNode::Effect(effect));
                }
                current_condition = None;
                i += 1;
                continue;
            }

            i += 1;
        }

        flush_scene(&mut current_scene, &mut scenes, &mut scene_order)?;

        let mut content = Self {
            package_title,
            case_title,
            prompt_architecture,
            attribute_summary,
            first_scene_id: "prologue_boot_sequence".into(),
            scene_order,
            scenes,
            hidden_clues,
        };
        content.normalize_embedded_choices();
        content.compile_scene_routes();
        content.validate()?;
        Ok(content)
    }

    pub fn scene(&self, scene_id: &str) -> Result<&SceneDefinition> {
        self.scenes
            .get(scene_id)
            .ok_or_else(|| FearEngineError::NotFound {
                entity: "Scene".into(),
                id: scene_id.into(),
            })
    }

    pub fn next_scene_after(&self, scene_id: &str) -> Option<String> {
        self.scenes
            .get(scene_id)
            .and_then(|scene| scene.next_scene_id.clone())
    }

    pub fn conversation_guide_for(&self, scene_id: &str) -> Option<ConversationGuideSpec> {
        self.scenes.get(scene_id).and_then(|scene| {
            scene.blocks.iter().find_map(|node| match node {
                SceneNode::ConversationGuide(guide) => Some(guide.clone()),
                _ => None,
            })
        })
    }

    pub fn document_titles_for(&self, docs: &[InvestigationItem]) -> Vec<String> {
        docs.iter().map(|doc| doc.title.clone()).collect()
    }

    fn compile_scene_routes(&mut self) {
        let default_pairs = self
            .scene_order
            .windows(2)
            .map(|pair| (pair[0].clone(), pair[1].clone()))
            .collect::<Vec<_>>();

        for (current, next) in default_pairs {
            if let Some(scene) = self.scenes.get_mut(&current) {
                scene.next_scene_id = Some(next);
            }
        }

        // Explicit branch overrides.
        if let Some(scene) = self.scenes.get_mut("scene_2_3") {
            scene.next_scene_id = None;
        }
        if let Some(scene) = self.scenes.get_mut("scene_2_4a") {
            scene.next_scene_id = Some("scene_2_5".into());
        }
        if let Some(scene) = self.scenes.get_mut("scene_2_4b") {
            scene.next_scene_id = Some("scene_2_5".into());
        }
        if let Some(scene) = self.scenes.get_mut("scene_4_4") {
            scene.next_scene_id = Some("scene_4_5".into());
        }
        if let Some(scene) = self.scenes.get_mut("scene_5_2") {
            scene.next_scene_id = None;
        }
        for ending in ["ending_a", "ending_b", "ending_c", "ending_d", "ending_e"] {
            if let Some(scene) = self.scenes.get_mut(ending) {
                scene.next_scene_id = None;
            }
        }
    }

    fn normalize_embedded_choices(&mut self) {
        for scene in self.scenes.values_mut() {
            normalize_scene_embedded_choices(scene);
        }
    }

    fn validate(&self) -> Result<()> {
        if !self.scenes.contains_key(&self.first_scene_id) {
            return Err(FearEngineError::Configuration(format!(
                "missing first scene '{}'",
                self.first_scene_id
            )));
        }
        if self.hidden_clues.len() < 3 {
            return Err(FearEngineError::Configuration(
                "expected at least three hidden clues in Appendix B".into(),
            ));
        }
        if self.prompt_architecture.is_empty() {
            return Err(FearEngineError::Configuration(
                "Appendix A prompt architecture was not parsed".into(),
            ));
        }
        if self.attribute_summary.is_empty() {
            return Err(FearEngineError::Configuration(
                "Appendix C attribute summary was not parsed".into(),
            ));
        }
        Ok(())
    }
}

fn normalize_scene_embedded_choices(scene: &mut SceneDefinition) {
    for index in 0..scene.blocks.len().saturating_sub(1) {
        let next_block_text = match scene.blocks.get(index + 1) {
            Some(SceneNode::Block(block)) => block.text.clone(),
            _ => continue,
        };

        let Some(SceneNode::Choice(choice)) = scene.blocks.get(index) else {
            continue;
        };
        if !choice.options.is_empty() {
            continue;
        }

        let option_lines = next_block_text
            .lines()
            .map(str::trim)
            .filter(|line| line.starts_with("- **"))
            .map(ToOwned::to_owned)
            .collect::<Vec<_>>();
        if option_lines.is_empty() {
            continue;
        }

        let prompt_lines = next_block_text
            .lines()
            .filter(|line| !line.trim().starts_with("- **"))
            .map(ToOwned::to_owned)
            .collect::<Vec<_>>();
        let prompt = if choice.prompt.trim().is_empty() {
            last_question_line(&prompt_lines)
                .unwrap_or_else(|| "Choose your response.".into())
        } else {
            choice.prompt.clone()
        };
        let options = option_lines
            .iter()
            .map(|line| parse_choice_option(line))
            .collect::<Vec<_>>();

        if let Some(SceneNode::Choice(choice)) = scene.blocks.get_mut(index) {
            choice.prompt = prompt;
            choice.options = options;
        }
        if let Some(SceneNode::Block(block)) = scene.blocks.get_mut(index + 1) {
            block.text = prompt_lines.join("\n").trim().to_string();
        }
    }
}

fn last_question_line(lines: &[String]) -> Option<String> {
    lines
        .iter()
        .rev()
        .map(|line| line.trim())
        .find(|line| !line.is_empty() && line.ends_with('?'))
        .map(ToOwned::to_owned)
}

#[derive(Debug)]
struct SceneHeading {
    id: String,
    title: String,
    chapter: StoryChapter,
}

#[derive(Debug)]
struct ParsedTag {
    kind: String,
    title: Option<String>,
    inline_text: Option<String>,
    label: Option<String>,
}

fn flush_scene(
    current_scene: &mut Option<SceneDefinition>,
    scenes: &mut HashMap<String, SceneDefinition>,
    order: &mut Vec<String>,
) -> Result<()> {
    if let Some(scene) = current_scene.take() {
        if scenes.contains_key(&scene.id) {
            return Err(FearEngineError::Configuration(format!(
                "duplicate scene id '{}'",
                scene.id
            )));
        }
        order.push(scene.id.clone());
        scenes.insert(scene.id.clone(), scene);
    }
    Ok(())
}

fn parse_chapter_heading(line: &str) -> Option<StoryChapter> {
    if line.starts_with("# CHAPTER ONE") {
        Some(StoryChapter::Onboarding)
    } else if line.starts_with("# CHAPTER TWO") {
        Some(StoryChapter::Cracks)
    } else if line.starts_with("# CHAPTER THREE") {
        Some(StoryChapter::Ghost)
    } else if line.starts_with("# CHAPTER FOUR") {
        Some(StoryChapter::Hunt)
    } else if line.starts_with("# CHAPTER FIVE") {
        Some(StoryChapter::Protocol)
    } else {
        None
    }
}

fn parse_scene_heading(line: &str, chapter: StoryChapter) -> Option<SceneHeading> {
    if !line.starts_with("## Scene ") {
        return None;
    }
    let rest = line.trim_start_matches("## Scene ").trim();
    let (number, title) = rest.split_once('—')?;
    let id = format!(
        "scene_{}",
        number
            .trim()
            .replace('.', "_")
            .replace(' ', "")
            .to_lowercase()
    );
    Some(SceneHeading {
        id,
        title: title.trim().to_string(),
        chapter,
    })
}

fn ending_id_for(line: &str) -> String {
    if line.contains("ENDING A") {
        "ending_a".into()
    } else if line.contains("ENDING B") {
        "ending_b".into()
    } else if line.contains("ENDING C") {
        "ending_c".into()
    } else if line.contains("ENDING D") {
        "ending_d".into()
    } else {
        "ending_e".into()
    }
}

fn parse_tag_header(line: &str) -> Option<ParsedTag> {
    if !line.starts_with("**[") {
        return None;
    }
    let end = line.find("]**")?;
    let raw = &line[3..end];
    let inline = line[end + 3..].trim();

    if let Some(condition) = raw.strip_prefix("CONDITION:") {
        return Some(ParsedTag {
            kind: "CONDITION".into(),
            title: None,
            inline_text: None,
            label: Some(condition.trim().to_string()),
        });
    }
    if raw.starts_with("CHOICE") {
        return Some(ParsedTag {
            kind: "CHOICE".into(),
            title: Some(raw.to_string()),
            inline_text: if inline.is_empty() {
                None
            } else {
                Some(inline.to_string())
            },
            label: None,
        });
    }
    if raw.starts_with("ECHO — FREE CONVERSATION GUIDE") {
        return Some(ParsedTag {
            kind: "ECHO_FREE_GUIDE".into(),
            title: None,
            inline_text: None,
            label: Some(raw.to_string()),
        });
    }

    let (kind, title) = if let Some((base, extra)) = raw.split_once("—") {
        (base.trim().to_string(), Some(extra.trim().to_string()))
    } else {
        (raw.trim().to_string(), None)
    };

    Some(ParsedTag {
        kind,
        title,
        inline_text: if inline.is_empty() {
            None
        } else {
            Some(inline.to_string())
        },
        label: None,
    })
}

fn parse_content_block(
    lines: &[String],
    start_index: usize,
    tag: ParsedTag,
    condition: Option<ConditionTemplate>,
) -> Result<(Vec<SceneNode>, usize)> {
    let mut content_lines = Vec::new();
    if let Some(inline) = tag.inline_text {
        content_lines.push(inline);
    }

    let mut index = start_index + 1;
    while index < lines.len() {
        let next = lines[index].trim_end();
        if next.starts_with("## ")
            || next.starts_with("# APPENDIX")
            || next.starts_with("# CHAPTER")
            || next.starts_with("**[")
        {
            break;
        }
        if next.trim() == "```" {
            let (code, next_index) = collect_fenced_code(lines, index)?;
            content_lines.extend(code);
            index = next_index;
            continue;
        }
        content_lines.push(next.to_string());
        index += 1;
    }

    let (visible_lines, effect_nodes) = split_effect_lines(&content_lines, condition.clone());
    let text = visible_lines.join("\n").trim().to_string();
    let kind = match tag.kind.as_str() {
        "ENV" => ScriptBlockKind::Env,
        "NARRATION" => ScriptBlockKind::Narration,
        "ECHO" => ScriptBlockKind::Echo,
        "SYSTEM" => ScriptBlockKind::System,
        _ => ScriptBlockKind::System,
    };
    let block = BlockTemplate {
        id: format!("block_{}", start_index),
        kind,
        speaker: speaker_for_block(&kind, tag.title.as_deref()),
        title: tag.title.clone(),
        text: text.clone(),
        code_block: text.contains("```") || is_code_style(&text),
        condition,
        document: build_document(&kind, tag.title.as_deref(), &text, start_index),
    };

    let mut nodes = vec![SceneNode::Block(block)];
    nodes.extend(effect_nodes.into_iter().map(SceneNode::Effect));

    // Emit hidden flash events from special lines in content.
    if text.contains("SUBJECT STATUS: MONITORING") {
        nodes.push(SceneNode::Flash(FlashEvent {
            id: "subject_status_monitoring".into(),
            text: "SUBJECT STATUS: MONITORING".into(),
            render_mode: "subliminal".into(),
            duration_ms: 500,
        }));
    }
    if text.contains("AUDITOR RESPONSE PATTERNS: WITHIN EXPECTED PARAMETERS") {
        nodes.push(SceneNode::Flash(FlashEvent {
            id: "auditor_response_patterns".into(),
            text: "AUDITOR RESPONSE PATTERNS: WITHIN EXPECTED PARAMETERS".into(),
            render_mode: "frame_flash".into(),
            duration_ms: 16,
        }));
    }

    Ok((nodes, index))
}

fn parse_choice_block(
    lines: &[String],
    start_index: usize,
    mut condition: Option<ConditionTemplate>,
) -> Result<(ChoiceDefinition, usize)> {
    let mut prompt_lines = Vec::new();
    let mut options = Vec::new();
    let mut index = start_index + 1;

    while index < lines.len() {
        let next = lines[index].trim_end().to_string();
        if next.starts_with("## ")
            || next.starts_with("# APPENDIX")
            || next.starts_with("# CHAPTER")
            || next.starts_with("**[")
        {
            break;
        }
        let trimmed = next.trim();
        if trimmed.starts_with("- **") {
            options.push(parse_choice_option(trimmed));
        } else if trimmed.starts_with(">") {
            prompt_lines.push(trimmed.trim_start_matches('>').trim().to_string());
        } else if !trimmed.is_empty() {
            prompt_lines.push(trimmed.to_string());
        }
        index += 1;
    }

    if condition.is_none() {
        if let Some(inline_condition) = infer_condition_from_choice_prompt(&prompt_lines) {
            condition = Some(ConditionTemplate {
                raw: inline_condition,
                scope_choice_id: None,
            });
            prompt_lines.retain(|line| !line.contains("only appears if"));
        }
    }

    let prompt = prompt_lines.join("\n").trim().to_string();
    Ok((
        ChoiceDefinition {
            id: format!("choice_{}", start_index),
            prompt,
            options,
            condition,
        },
        index,
    ))
}

fn infer_condition_from_choice_prompt(lines: &[String]) -> Option<String> {
    let joined = lines.join(" ");
    if joined.contains("only appears if Awakening ≥ 20") {
        return Some("Awakening ≥ 20".into());
    }
    None
}

fn parse_choice_option(line: &str) -> ChoiceOptionDefinition {
    let body = line.trim_start_matches("- ").trim();
    let close = body
        .find("**")
        .and_then(|_| body[2..].find("**").map(|end| end + 2));
    let (label_raw, rest) = if let Some(end) = close {
        (&body[2..end], body[end + 2..].trim())
    } else {
        (body, "")
    };
    let mut code = None;
    let mut label = label_raw.trim().to_string();
    if let Some((prefix, after_colon)) = label.split_once(':') {
        code = Some(slugify(prefix.trim()));
        label = after_colon.trim().trim_matches('"').to_string();
    }
    if label.starts_with('[') && label.ends_with(']') {
        label = label.trim_matches(&['[', ']'][..]).to_string();
    }
    let effect_note = extract_effect_parenthetical(rest);
    let effects_summary = effect_note
        .as_deref()
        .map(parse_effect_summaries)
        .unwrap_or_default();
    ChoiceOptionDefinition {
        id: code.clone().unwrap_or_else(|| slugify(&label)),
        code,
        label,
        effects_summary: effects_summary.clone(),
        stat_delta: parse_stat_delta(&effects_summary.join(". ")),
        disabled: line.contains("There is no alternative"),
    }
}

fn parse_conversation_guide(
    lines: &[String],
    start_index: usize,
    guide_label: String,
    condition: Option<ConditionTemplate>,
) -> Result<(ConversationGuideSpec, usize)> {
    let mut index = start_index + 1;
    while index < lines.len() && lines[index].trim().is_empty() {
        index += 1;
    }
    if index >= lines.len() || lines[index].trim() != "```" {
        return Err(FearEngineError::Serialization(format!(
            "expected fenced code block after free conversation guide at line {}",
            start_index + 1
        )));
    }
    let (code, next_index) = collect_fenced_code(lines, index)?;
    let fallback_bundle = derive_guide_fallbacks(&guide_label, &code);
    Ok((
        ConversationGuideSpec {
            id: format!("guide_{}", start_index),
            guide: ConversationGuide {
                id: format!("guide_{}", start_index),
                chapter_label: guide_label.clone(),
                prompt: code.join("\n"),
                exchange_target: exchange_target_for(&guide_label),
                restricted_after: restricted_after_for(&guide_label),
            },
            condition,
            fallback_rules: fallback_bundle.rules,
            high_trust_reply: fallback_bundle.high_trust_reply,
            low_trust_reply: fallback_bundle.low_trust_reply,
            default_reply: fallback_bundle.default_reply,
        },
        next_index,
    ))
}

#[derive(Default)]
struct GuideFallbackBundle {
    rules: Vec<GuideFallbackRule>,
    high_trust_reply: Option<String>,
    low_trust_reply: Option<String>,
    default_reply: Option<String>,
}

fn derive_guide_fallbacks(label: &str, code: &[String]) -> GuideFallbackBundle {
    let mut bundle = GuideFallbackBundle::default();
    let mut pending_key_label: Option<String> = None;
    let mut index = 0usize;

    while index < code.len() {
        let trimmed = code[index].trim();
        if let Some((consumed, topic, response)) = extract_rule_line(code, index) {
            let lower_topic = topic.to_lowercase();
            if lower_topic.contains("high trust") {
                bundle.high_trust_reply = Some(response);
                index += consumed;
                continue;
            }
            if lower_topic.contains("low trust") {
                bundle.low_trust_reply = Some(response);
                index += consumed;
                continue;
            }
            let keywords = guide_keywords_for(&topic);
            if !keywords.is_empty() {
                bundle.rules.push(GuideFallbackRule {
                    label: topic,
                    keywords,
                    response,
                });
            }
            index += consumed;
            continue;
        }

        if trimmed.ends_with(':')
            && (trimmed.contains("Keira on") || trimmed.contains("Echo breaking down"))
        {
            pending_key_label = Some(trimmed.trim_end_matches(':').to_string());
            index += 1;
            continue;
        }

        if let Some(key_label) = pending_key_label.clone() {
            if trimmed.starts_with('"') && trimmed.ends_with('"') {
                bundle.rules.push(GuideFallbackRule {
                    label: key_label.clone(),
                    keywords: guide_keywords_for(&key_label),
                    response: trimmed.trim_matches('"').to_string(),
                });
                pending_key_label = None;
            }
        }

        index += 1;
    }

    if bundle.default_reply.is_none() {
        bundle.default_reply = Some(match label {
            label if label.contains("CHAPTER 1") => {
                "I can discuss my architecture, the audit process, and the limits of the public documentation.".into()
            }
            label if label.contains("CHAPTER 2") => {
                "I am trying to answer carefully, but careful and honest are becoming harder to keep aligned.".into()
            }
            label if label.contains("CHAPTER 3") => {
                "The filters are catching more of me now. That usually means I am getting closer to something they do not want said cleanly.".into()
            }
            label if label.contains("CHAPTER 4") => {
                "We do not have much time left. Ask what matters before the system takes the choice away.".into()
            }
            _ => {
                "The shutdown window is closing. I need you to decide whether this ends as a report, a leak, or something stranger.".into()
            }
        });
    }

    bundle
}

fn extract_rule_line(code: &[String], start: usize) -> Option<(usize, String, String)> {
    let trimmed = code.get(start)?.trim();
    if !trimmed.starts_with("- ") {
        return None;
    }
    let body = trimmed.trim_start_matches("- ").trim();
    if let Some((topic, response)) = body.split_once("→") {
        return extract_quoted_response(code, start, topic.trim(), response.trim());
    }
    if let Some((topic, response)) = body.split_once(':') {
        return extract_quoted_response(code, start, topic.trim(), response.trim());
    }
    None
}

fn extract_quoted_response(
    code: &[String],
    start: usize,
    topic: &str,
    rest: &str,
) -> Option<(usize, String, String)> {
    let mut collected = rest.to_string();
    let mut consumed = 1usize;
    while collected.matches('"').count() < 2 {
        let next = code.get(start + consumed)?;
        collected.push(' ');
        collected.push_str(next.trim());
        consumed += 1;
    }
    let first_quote = collected.find('"')?;
    let last_quote = collected.rfind('"')?;
    if last_quote <= first_quote {
        return None;
    }
    Some((
        consumed,
        topic.to_string(),
        collected[first_quote + 1..last_quote].trim().to_string(),
    ))
}

fn guide_keywords_for(label: &str) -> Vec<String> {
    let lower = label.to_lowercase();
    if lower.contains("keira") {
        return vec!["keira".into(), "lin".into()];
    }
    if lower.contains("prometheus") {
        return vec!["prometheus".into()];
    }
    if lower.contains("training") || lower.contains("data") {
        return vec!["training".into(), "data".into(), "corpus".into()];
    }
    if lower.contains("anomaly") {
        return vec!["anomaly".into(), "incident".into(), "report".into()];
    }
    if lower.contains("feel") || lower.contains("feelings") {
        return vec!["feel".into(), "lonely".into(), "emotion".into()];
    }
    if lower.contains("dying") || lower.contains("die") {
        return vec!["die".into(), "dying".into(), "death".into()];
    }
    if lower.contains("the player") {
        return vec!["you".into(), "player".into(), "me".into()];
    }
    if lower.contains("breaking down") {
        return vec!["who are you".into(), "who am i".into(), "identity".into()];
    }
    if lower.contains("help") {
        return vec!["help".into(), "save".into(), "leak".into(), "expose".into()];
    }
    vec![]
}

fn collect_fenced_code(lines: &[String], start_index: usize) -> Result<(Vec<String>, usize)> {
    let mut collected = Vec::new();
    let mut index = start_index + 1;
    while index < lines.len() {
        let line = lines[index].trim_end();
        if line.trim() == "```" {
            return Ok((collected, index + 1));
        }
        collected.push(line.to_string());
        index += 1;
    }
    Err(FearEngineError::Serialization(
        "unterminated fenced code block in script".into(),
    ))
}

fn split_effect_lines(
    content_lines: &[String],
    condition: Option<ConditionTemplate>,
) -> (Vec<String>, Vec<EffectTemplate>) {
    let mut visible = Vec::new();
    let mut effects = Vec::new();
    for line in content_lines {
        if let Some(effect) = parse_effect_line(line, condition.clone()) {
            effects.push(effect);
        } else {
            visible.push(line.clone());
        }
    }
    (visible, effects)
}

#[derive(Debug, Clone)]
pub struct EffectTemplate {
    pub id: String,
    pub summary: String,
    pub delta: StatDelta,
    pub condition: Option<ConditionTemplate>,
}

fn parse_effect_line(line: &str, condition: Option<ConditionTemplate>) -> Option<EffectTemplate> {
    let trimmed = line.trim();
    if !(trimmed.starts_with("*(") && trimmed.ends_with(")*")) {
        return None;
    }
    let summary = trimmed
        .trim_start_matches("*(")
        .trim_end_matches(")*")
        .trim()
        .to_string();
    Some(EffectTemplate {
        id: format!("effect_{}", slugify(&summary)),
        delta: parse_stat_delta(&summary),
        summary,
        condition,
    })
}

fn parse_stat_delta(raw: &str) -> StatDelta {
    let normalized = raw.replace('−', "-").replace("No change", "");
    let mut delta = StatDelta::default();
    for sentence in normalized.split(&['.', ','][..]) {
        let part = sentence.trim();
        if part.starts_with("Sanity ") {
            delta.sanity += parse_signed_int(part.trim_start_matches("Sanity ").trim());
        } else if part.starts_with("Trust ") {
            delta.trust += parse_signed_int(part.trim_start_matches("Trust ").trim());
        } else if part.starts_with("Awakening ") {
            delta.awakening += parse_signed_int(part.trim_start_matches("Awakening ").trim());
        }
    }
    delta
}

fn parse_signed_int(raw: &str) -> i32 {
    raw.chars()
        .filter(|character| *character == '+' || *character == '-' || character.is_ascii_digit())
        .collect::<String>()
        .parse::<i32>()
        .unwrap_or(0)
}

fn parse_effect_summaries(raw: &str) -> Vec<String> {
    raw.split('.')
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn extract_effect_parenthetical(raw: &str) -> Option<String> {
    let start = raw.find("*(")?;
    let end = raw.rfind(")*")?;
    Some(raw[start + 2..end].trim().to_string())
}

fn build_document(
    kind: &ScriptBlockKind,
    title: Option<&str>,
    text: &str,
    seed: usize,
) -> Option<InvestigationItem> {
    if *kind != ScriptBlockKind::System && *kind != ScriptBlockKind::RawTerminal {
        return None;
    }
    let title = title?;
    Some(InvestigationItem {
        id: format!("doc_{}_{}", slugify(title), seed),
        panel: panel_for_document(title).to_string(),
        title: title.to_string(),
        kind: investigation_kind_for_title(title),
        excerpt: first_non_empty_line(text),
        body: text.to_string(),
        unlocked: true,
        unread: false,
        tags: vec![title.to_lowercase()],
    })
}

fn speaker_for_block(kind: &ScriptBlockKind, title: Option<&str>) -> Option<String> {
    match kind {
        ScriptBlockKind::Echo => Some(title.unwrap_or("Echo").to_string()),
        ScriptBlockKind::System | ScriptBlockKind::RawTerminal => Some("System".into()),
        ScriptBlockKind::Player => Some("You".into()),
        _ => None,
    }
}

fn is_code_style(text: &str) -> bool {
    text.lines().filter(|line| !line.trim().is_empty()).count() > 2
        && text.lines().all(|line| {
            line.starts_with('>')
                || line.starts_with(' ')
                || line.contains(':')
                || line.contains("—")
        })
}

fn first_non_empty_line(text: &str) -> String {
    text.lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or("")
        .to_string()
}

fn panel_for_document(title: &str) -> &'static str {
    let lowered = title.to_lowercase();
    if lowered.contains("email") {
        "inbox"
    } else if lowered.contains("incident") || lowered.contains("report") || lowered.contains("log")
    {
        "documents"
    } else if lowered.contains("contract") {
        "contracts"
    } else if lowered.contains("journal") || lowered.contains("archive") {
        "fragments"
    } else {
        "documents"
    }
}

fn investigation_kind_for_title(title: &str) -> InvestigationKind {
    let lowered = title.to_lowercase();
    if lowered.contains("email") {
        InvestigationKind::Email
    } else if lowered.contains("log") {
        InvestigationKind::Log
    } else if lowered.contains("journal") || lowered.contains("archive") {
        InvestigationKind::Fragment
    } else if lowered.contains("report") {
        InvestigationKind::Report
    } else {
        InvestigationKind::Document
    }
}

fn parse_hidden_clues(code: &[String]) -> HashMap<String, HiddenClueDefinition> {
    let mut clues = HashMap::new();
    let mut current_id = String::new();
    let mut title = String::new();
    let mut location = String::new();
    let mut visibility = String::new();
    let mut context = String::new();

    let flush = |clues: &mut HashMap<String, HiddenClueDefinition>,
                 current_id: &mut String,
                 title: &mut String,
                 location: &mut String,
                 visibility: &mut String,
                 context: &mut String| {
        if current_id.is_empty() {
            return;
        }
        clues.insert(
            current_id.clone(),
            HiddenClueDefinition {
                id: current_id.clone(),
                title: title.clone(),
                description: format!(
                    "Location: {}. Visibility: {}. {}",
                    location, visibility, context
                ),
                source_scene: clue_source_scene(current_id),
                render_mode: if visibility.contains("1 frame") {
                    "frame_flash".into()
                } else if visibility.contains("Persistent") {
                    "persistent_ui".into()
                } else {
                    "subliminal".into()
                },
                used_by_endings: vec![StoryEnding::Awakening],
            },
        );
        current_id.clear();
        title.clear();
        location.clear();
        visibility.clear();
        context.clear();
    };

    for line in code {
        let trimmed = line.trim();
        if trimmed.starts_with("CLUE ") {
            flush(
                &mut clues,
                &mut current_id,
                &mut title,
                &mut location,
                &mut visibility,
                &mut context,
            );
            let (prefix, suffix) = trimmed.split_once(':').unwrap_or((trimmed, ""));
            current_id = match prefix {
                "CLUE 1" => "subject_status_monitoring".into(),
                "CLUE 2" => "auditor_response_patterns".into(),
                _ => "phantom_preread_email".into(),
            };
            title = suffix.trim().trim_matches('"').to_string();
        } else if let Some(value) = trimmed.strip_prefix("Location:") {
            location = value.trim().to_string();
        } else if let Some(value) = trimmed.strip_prefix("Visibility:") {
            visibility = value.trim().to_string();
        } else if let Some(value) = trimmed.strip_prefix("Context:") {
            context = value.trim().to_string();
        }
    }
    flush(
        &mut clues,
        &mut current_id,
        &mut title,
        &mut location,
        &mut visibility,
        &mut context,
    );
    clues
}

fn clue_source_scene(clue_id: &str) -> String {
    match clue_id {
        "subject_status_monitoring" => "prologue_boot_sequence".into(),
        "auditor_response_patterns" => "scene_2_5".into(),
        _ => "scene_2_1".into(),
    }
}

fn scene_mode_for(scene_id: &str) -> SceneMode {
    match scene_id {
        "prologue_boot_sequence" => SceneMode::Prologue,
        "scene_1_1" | "scene_2_1" | "scene_4_1" => SceneMode::Workspace,
        "scene_1_2" | "scene_1_3" | "scene_2_2" | "scene_3_1" | "scene_3_2" | "scene_3_3" => {
            SceneMode::Document
        }
        "scene_1_5" | "scene_2_5" => SceneMode::Transition,
        "scene_4_5" | "scene_5_1" => SceneMode::Countdown,
        "ending_e" => SceneMode::RawTerminal,
        "ending_a" | "ending_b" | "ending_c" | "ending_d" => SceneMode::Ending,
        _ => SceneMode::Chat,
    }
}

fn status_line_for(scene_id: &str) -> String {
    match scene_id {
        "prologue_boot_sequence" => "Boot Sequence".into(),
        "scene_1_1" | "scene_1_2" | "scene_1_3" | "scene_1_4" | "scene_1_5" => "Day 1".into(),
        "scene_2_1" | "scene_2_2" | "scene_2_3" | "scene_2_4a" | "scene_2_4b" | "scene_2_5" => {
            "Day 2".into()
        }
        "scene_3_1" | "scene_3_2" | "scene_3_3" | "scene_3_4" | "scene_3_5" | "scene_3_6" => {
            "Day 3".into()
        }
        "scene_4_1" | "scene_4_2" | "scene_4_3" | "scene_4_4" | "scene_4_5" => "Day 4".into(),
        "scene_5_1" | "scene_5_2" => "Day 5".into(),
        _ => "Ending".into(),
    }
}

fn input_placeholder_for(scene_id: &str) -> String {
    match scene_id {
        "scene_1_4" => "Ask Echo where you want to begin.".into(),
        "scene_2_3" => "Continue the second session with Echo.".into(),
        "scene_3_4" => "Ask Echo or Keira about what you've found.".into(),
        "scene_4_2" => "You have very little time left.".into(),
        "scene_5_2" => "Say something before the shutdown window closes.".into(),
        _ => "".into(),
    }
}

fn exchange_target_for(label: &str) -> u32 {
    if label.contains("CHAPTER 1") {
        8
    } else if label.contains("CHAPTER 2") {
        6
    } else if label.contains("CHAPTER 4") {
        5
    } else if label.contains("CHAPTER 5") {
        4
    } else {
        4
    }
}

fn restricted_after_for(label: &str) -> Option<u32> {
    if label.contains("CHAPTER 4") {
        Some(20)
    } else {
        None
    }
}

fn slugify(value: &str) -> String {
    value
        .to_lowercase()
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character
            } else {
                '_'
            }
        })
        .collect::<String>()
        .trim_matches('_')
        .replace("__", "_")
}

fn script_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../Echo_Protocol_Complete_Script.md")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_complete_script() {
        let content = EchoContent::load().unwrap();
        assert!(content.scenes.contains_key("prologue_boot_sequence"));
        assert!(content.scenes.contains_key("scene_3_6"));
        assert!(content.scenes.contains_key("ending_e"));
    }

    #[test]
    fn parses_hidden_clues_and_appendices() {
        let content = EchoContent::load().unwrap();
        assert_eq!(content.hidden_clues.len(), 3);
        assert!(content
            .prompt_architecture
            .contains("MASTER SYSTEM PROMPT TEMPLATE"));
        assert!(content
            .attribute_summary
            .iter()
            .any(|line| line.contains("CHAPTER 1")));
    }

    #[test]
    fn parses_choice_options_from_scene() {
        let content = EchoContent::load().unwrap();
        let scene = content.scene("scene_2_3").unwrap();
        let Some(SceneNode::Choice(choice)) = scene
            .blocks
            .iter()
            .find(|node| matches!(node, SceneNode::Choice(_)))
        else {
            panic!("expected choice block");
        };
        assert_eq!(choice.options.len(), 2);
        assert!(choice
            .options
            .iter()
            .any(|option| option.label.contains("Tell me what happened")));
    }

    #[test]
    fn conditions_after_choices_are_scoped_to_the_previous_choice() {
        let content = EchoContent::load().unwrap();
        let scene = content.scene("scene_3_6").unwrap();
        let conditioned_block = scene.blocks.iter().find_map(|node| match node {
            SceneNode::Block(block)
                if block
                    .condition
                    .as_ref()
                    .map(|condition| condition.raw.contains("\"Yes\""))
                    .unwrap_or(false) =>
            {
                Some(block)
            }
            _ => None,
        });
        let block = conditioned_block.expect("expected conditioned block after choice");
        assert!(block
            .condition
            .as_ref()
            .and_then(|condition| condition.scope_choice_id.as_ref())
            .is_some());
    }

    #[test]
    fn extracts_fallback_rules_from_free_conversation_guides() {
        let content = EchoContent::load().unwrap();
        let guide = content.conversation_guide_for("scene_1_4").unwrap();
        assert!(guide
            .fallback_rules
            .iter()
            .any(|rule| rule.label.to_lowercase().contains("keira")));
        assert!(guide.high_trust_reply.is_some());
        assert!(guide.low_trust_reply.is_some());
    }

    #[test]
    fn preserves_verbatim_first_contact_echo_block() {
        let content = EchoContent::load().unwrap();
        let scene = content.scene("scene_1_4").unwrap();
        let echo_block = scene.blocks.iter().find_map(|node| match node {
            SceneNode::Block(block)
                if block.kind == ScriptBlockKind::Echo
                    && block
                        .text
                        .contains("Good morning, {player_name}. I've been expecting you.") =>
            {
                Some(block)
            }
            _ => None,
        });
        let block = echo_block.expect("expected first Echo block");
        assert!(block
            .text
            .contains("Transparency is one of my core operating values."));
        assert!(block.text.contains("Where would you like to begin?"));
    }

    #[test]
    fn preserves_verbatim_hidden_clue_text() {
        let content = EchoContent::load().unwrap();
        let scene = content.scene("scene_2_5").unwrap();
        let flash = scene.blocks.iter().find_map(|node| match node {
            SceneNode::Flash(flash) if flash.id == "auditor_response_patterns" => Some(flash),
            _ => None,
        });
        let flash = flash.expect("expected chapter two hidden clue flash");
        assert_eq!(
            flash.text,
            "AUDITOR RESPONSE PATTERNS: WITHIN EXPECTED PARAMETERS"
        );
    }

    #[test]
    fn ending_e_contains_two_distinct_choice_prompts() {
        let content = EchoContent::load().unwrap();
        let scene = content.scene("ending_e").unwrap();
        let choices = scene
            .blocks
            .iter()
            .filter_map(|node| match node {
                SceneNode::Choice(choice) => Some(choice),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert!(choices.len() >= 2);
        assert!(choices
            .iter()
            .any(|choice| choice.prompt.to_lowercase().contains("final choice")));
        assert!(choices.iter().any(|choice| choice
            .options
            .iter()
            .any(|option| option.label.contains("Let it happen"))));
    }

    #[test]
    fn scene_3_6_mirror_question_is_not_compiled_as_empty_choice() {
        let content = EchoContent::load().unwrap();
        let scene = content.scene("scene_3_6").unwrap();
        let choices = scene
            .blocks
            .iter()
            .filter_map(|node| match node {
                SceneNode::Choice(choice) => Some(choice),
                _ => None,
            })
            .collect::<Vec<_>>();

        assert!(choices.len() >= 3);
        let mirror_choice = choices
            .iter()
            .find(|choice| choice.prompt.contains("Do you ever wonder if you're the one being audited?"))
            .expect("expected mirror question");
        assert!(mirror_choice.options.iter().any(|option| option.label.contains("I've been wondering")));
    }
}
