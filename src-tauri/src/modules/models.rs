use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub hotkey: String,
    pub trigger_source: String,
    pub auto_start: bool,
    pub max_text_length: usize,
    pub close_on_focus_loss: bool,
    pub popup_width: u32,
    pub popup_height: u32,
    pub popup_font_scale: f32,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            hotkey: "Ctrl+Shift+D".to_string(),
            trigger_source: "keyboard".to_string(),
            auto_start: false,
            max_text_length: 280,
            close_on_focus_loss: true,
            popup_width: 380,
            popup_height: 400,
            popup_font_scale: 0.92,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DictionarySense {
    pub part_of_speech: String,
    pub definitions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DictionaryEntry {
    pub word: String,
    pub phonetic: Option<String>,
    pub senses: Vec<DictionarySense>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeywordDefinition {
    pub word: String,
    pub part_of_speech: Option<String>,
    pub brief_definition: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WordLookupPayload {
    pub source_text: String,
    pub normalized_text: String,
    pub lemma: String,
    pub phonetic: Option<String>,
    pub senses: Vec<DictionarySense>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhraseLookupPayload {
    pub source_text: String,
    pub normalized_text: String,
    pub translation: String,
    pub keywords: Vec<KeywordDefinition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum LookupPayload {
    Word(WordLookupPayload),
    Phrase(PhraseLookupPayload),
    Sentence(PhraseLookupPayload),
}

impl LookupPayload {
    pub fn normalized_text(&self) -> &str {
        match self {
            LookupPayload::Word(payload) => &payload.normalized_text,
            LookupPayload::Phrase(payload) | LookupPayload::Sentence(payload) => {
                &payload.normalized_text
            }
        }
    }

    pub fn source_text(&self) -> &str {
        match self {
            LookupPayload::Word(payload) => &payload.source_text,
            LookupPayload::Phrase(payload) | LookupPayload::Sentence(payload) => {
                &payload.source_text
            }
        }
    }

    pub fn display_title(&self) -> String {
        match self {
            LookupPayload::Word(payload) => payload.lemma.clone(),
            LookupPayload::Phrase(payload) | LookupPayload::Sentence(payload) => {
                payload.source_text.clone()
            }
        }
    }

    pub fn entry_type(&self) -> &'static str {
        match self {
            LookupPayload::Word(_) => "word",
            LookupPayload::Phrase(_) => "phrase",
            LookupPayload::Sentence(_) => "sentence",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LookupResult {
    pub payload: LookupPayload,
    pub is_starred: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnchorPoint {
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LookupEvent {
    pub anchor: AnchorPoint,
    pub result: LookupResult,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LookupLoadingEvent {
    pub anchor: AnchorPoint,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LookupErrorEvent {
    pub anchor: Option<AnchorPoint>,
    pub text: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StarredEntry {
    pub id: String,
    pub entry_type: String,
    pub source_text: String,
    pub normalized_text: String,
    pub display_title: String,
    pub payload: LookupPayload,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StarredQuery {
    pub search: Option<String>,
    pub entry_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToggleStarredResponse {
    pub is_starred: bool,
    pub entry: Option<StarredEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManualLookupRequest {
    pub text: String,
}
