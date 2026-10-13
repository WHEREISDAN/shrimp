//! Transforms settings (TRF-01..05): Polish and its rules, Prompt Engineer,
//! up to 8 custom transforms, their shortcuts and auto-apply.

use serde::{Deserialize, Serialize};

pub const MAX_CUSTOM_TRANSFORMS: usize = 8;
pub const MAX_CUSTOM_INSTRUCTIONS: usize = 5;
pub const MAX_INSTRUCTION_WORDS: usize = 50;
pub const MAX_SAMPLES: usize = 5;
pub const SAMPLE_WORDS: std::ops::RangeInclusive<usize> = 50..=500;
const MAX_NAME_CHARS: usize = 40;
const MAX_PROMPT_CHARS: usize = 4_000;

pub const DEFAULT_PROMPT_ENGINEER: &str = "Turn the text into a clear, well-structured prompt \
for an AI assistant. State the goal first, then the context, the requirements and constraints, \
and the output wanted, using short headings or bullet points where they help. Keep every detail \
the writer gave and invent no new requirements.";

/// Which transform auto-apply runs.
pub const POLISH: &str = "polish";
pub const PROMPT_ENGINEER: &str = "promptEngineer";

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct TransformSettings {
    /// Turned on from the Transforms page; until then no shortcut is bound.
    pub enabled: bool,
    pub polish: PolishSettings,
    pub prompt_engineer: PromptEngineerSettings,
    pub custom: Vec<CustomTransform>,
    /// Phase 3's View changes shortcut, moved into `shortcuts.bindings` on load.
    #[serde(default, skip_serializing)]
    pub view_changes_shortcut: Option<String>,
    /// "polish", "promptEngineer" or a custom transform's id; runs after
    /// every dictation (TRF-05).
    pub auto_apply: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PolishSettings {
    pub rules: PolishRules,
    pub instructions: Vec<String>,
    /// Phase 3's shortcut, moved into `shortcuts.bindings` on load.
    #[serde(default, skip_serializing)]
    pub shortcut: Option<String>,
}

/// Wispr Flow's five base Polish rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PolishRules {
    pub concise: bool,
    pub clarity: bool,
    pub reorder: bool,
    pub structure: bool,
    pub tone: bool,
}

impl Default for PolishRules {
    fn default() -> Self {
        Self {
            concise: true,
            clarity: true,
            reorder: true,
            structure: true,
            tone: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PromptEngineerSettings {
    pub prompt: String,
    /// Phase 3's shortcut, moved into `shortcuts.bindings` on load.
    #[serde(default, skip_serializing)]
    pub shortcut: Option<String>,
}

impl Default for PromptEngineerSettings {
    fn default() -> Self {
        Self {
            prompt: DEFAULT_PROMPT_ENGINEER.into(),
            shortcut: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct CustomTransform {
    pub id: String,
    pub name: String,
    pub prompt: String,
    /// A shortcut sent with a new transform, moved into `shortcuts.bindings`.
    #[serde(skip_serializing)]
    pub shortcut: Option<String>,
    /// Writing samples whose style the result follows.
    pub samples: Vec<String>,
}

impl CustomTransform {
    /// A transform runs once it has a name and a prompt.
    pub fn is_complete(&self) -> bool {
        !self.name.trim().is_empty() && !self.prompt.trim().is_empty()
    }
}

fn words(text: &str) -> usize {
    text.split_whitespace().count()
}

impl TransformSettings {
    /// Shortcuts still in phase 3's fields, by their `shortcuts.bindings` key.
    /// Taking them clears the fields.
    pub fn take_legacy_shortcuts(&mut self) -> Vec<(String, String)> {
        let mut taken = Vec::new();
        let mut take = |key: String, field: &mut Option<String>| {
            if let Some(shortcut) = field.take() {
                taken.push((key, shortcut));
            }
        };
        take(POLISH.into(), &mut self.polish.shortcut);
        take(PROMPT_ENGINEER.into(), &mut self.prompt_engineer.shortcut);
        take("viewChanges".into(), &mut self.view_changes_shortcut);
        for transform in &mut self.custom {
            take(custom_key(&transform.id), &mut transform.shortcut);
        }
        taken
    }

    pub fn validate(&self) -> Result<(), String> {
        let instructions = &self.polish.instructions;
        if instructions.len() > MAX_CUSTOM_INSTRUCTIONS {
            return Err(format!(
                "Use at most {MAX_CUSTOM_INSTRUCTIONS} custom instructions."
            ));
        }
        if instructions.iter().any(|text| text.trim().is_empty()) {
            return Err("An instruction cannot be empty.".into());
        }
        if instructions
            .iter()
            .any(|text| words(text) > MAX_INSTRUCTION_WORDS)
        {
            return Err(format!(
                "Keep each instruction under {MAX_INSTRUCTION_WORDS} words."
            ));
        }
        for (index, text) in instructions.iter().enumerate() {
            let duplicate = instructions[..index]
                .iter()
                .any(|other| other.trim().eq_ignore_ascii_case(text.trim()));
            if duplicate {
                return Err("That instruction is already in the list.".into());
            }
        }
        if self.prompt_engineer.prompt.len() > MAX_PROMPT_CHARS {
            return Err(format!(
                "Keep the prompt under {MAX_PROMPT_CHARS} characters."
            ));
        }
        if self.custom.len() > MAX_CUSTOM_TRANSFORMS {
            return Err(format!(
                "Use at most {MAX_CUSTOM_TRANSFORMS} custom transforms."
            ));
        }
        for transform in &self.custom {
            if transform.id.trim().is_empty() {
                return Err("A transform needs an id.".into());
            }
            if transform.name.chars().count() > MAX_NAME_CHARS {
                return Err(format!("Keep the name under {MAX_NAME_CHARS} characters."));
            }
            if transform.prompt.len() > MAX_PROMPT_CHARS {
                return Err(format!(
                    "Keep the prompt under {MAX_PROMPT_CHARS} characters."
                ));
            }
            if transform.samples.len() > MAX_SAMPLES {
                return Err(format!("Use at most {MAX_SAMPLES} writing samples."));
            }
        }
        if let Some(choice) = &self.auto_apply {
            let known = choice == POLISH
                || choice == PROMPT_ENGINEER
                || self.custom.iter().any(|transform| &transform.id == choice);
            if !known {
                return Err("Choose a transform to apply after dictation.".into());
            }
        }
        Ok(())
    }
}

/// The `shortcuts.bindings` key of a custom transform.
pub fn custom_key(id: &str) -> String {
    format!("transform:{id}")
}

/// Whether a writing sample has a usable length.
pub fn sample_fits(sample: &str) -> bool {
    SAMPLE_WORDS.contains(&words(sample))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_off() {
        let settings = TransformSettings::default();
        assert!(!settings.enabled);
        assert!(settings.polish.rules.concise && settings.polish.rules.tone);
        settings.validate().unwrap();
    }

    #[test]
    fn limits_are_enforced() {
        let mut settings = TransformSettings::default();
        settings.polish.instructions = vec!["Use British spelling".into(); 2];
        assert!(settings.validate().is_err(), "duplicates");
        settings.polish.instructions = (0..6).map(|n| format!("Rule {n}")).collect();
        assert!(settings.validate().is_err(), "at most 5");
        settings.polish.instructions = vec!["word ".repeat(51)];
        assert!(settings.validate().is_err(), "at most 50 words");
        settings.polish.instructions = Vec::new();
        settings.custom = (0..9)
            .map(|n| CustomTransform {
                id: format!("t{n}"),
                ..CustomTransform::default()
            })
            .collect();
        assert!(settings.validate().is_err(), "at most 8");
        settings.custom.truncate(1);
        settings.auto_apply = Some("t0".into());
        settings.validate().unwrap();
        settings.auto_apply = Some("gone".into());
        assert!(settings.validate().is_err());
    }

    #[test]
    fn phase_3_shortcuts_are_taken_once_and_never_written() {
        let json = serde_json::json!({
            "polish": {"shortcut": "Opt+1"},
            "viewChangesShortcut": "Opt+O",
            "custom": [{"id": "pirate", "shortcut": "Opt+3"}],
        });
        let mut settings: TransformSettings = serde_json::from_value(json).unwrap();
        let taken = settings.take_legacy_shortcuts();
        assert_eq!(
            taken,
            vec![
                ("polish".to_owned(), "Opt+1".to_owned()),
                ("viewChanges".to_owned(), "Opt+O".to_owned()),
                ("transform:pirate".to_owned(), "Opt+3".to_owned()),
            ]
        );
        assert!(settings.take_legacy_shortcuts().is_empty());
        let written = serde_json::to_value(&settings).unwrap();
        assert!(written["polish"].get("shortcut").is_none());
        assert!(written["custom"][0].get("shortcut").is_none());
    }

    #[test]
    fn samples_need_50_to_500_words() {
        assert!(!sample_fits("too short"));
        assert!(sample_fits(&"word ".repeat(60)));
        assert!(!sample_fits(&"word ".repeat(501)));
    }
}
