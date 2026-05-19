use std::collections::HashMap;

use anyhow::Context;

use crate::modules::models::DictionaryEntry;

pub struct Dictionary {
    entries: HashMap<String, DictionaryEntry>,
}

impl Dictionary {
    pub fn load() -> anyhow::Result<Self> {
        let raw = include_str!("../../resources/dictionary.json");
        let list: Vec<DictionaryEntry> =
            serde_json::from_str(raw).context("failed to parse dictionary json")?;
        let entries = list
            .into_iter()
            .map(|entry| (entry.word.clone(), entry))
            .collect::<HashMap<_, _>>();
        Ok(Self { entries })
    }

    pub fn get(&self, word: &str) -> Option<&DictionaryEntry> {
        self.entries.get(word)
    }

    pub fn contains(&self, word: &str) -> bool {
        self.entries.contains_key(word)
    }
}

pub fn normalize_text(text: &str) -> String {
    text.trim()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

pub fn is_single_word(text: &str) -> bool {
    let tokens = tokenize_words(text);
    tokens.len() == 1
}

pub fn tokenize_words(text: &str) -> Vec<String> {
    text.split(|c: char| !(c.is_ascii_alphabetic() || c == '\''))
        .filter(|segment| !segment.trim().is_empty())
        .map(|segment| segment.trim_matches('\'').to_lowercase())
        .filter(|segment| !segment.is_empty())
        .collect()
}

pub fn infer_entry_kind(text: &str) -> &'static str {
    let tokens = tokenize_words(text);
    if tokens.len() <= 1 {
        "word"
    } else if text.contains('.') || text.contains('!') || text.contains('?') || tokens.len() > 5 {
        "sentence"
    } else {
        "phrase"
    }
}

pub fn lemmatize_word(dictionary: &Dictionary, raw: &str) -> String {
    let token = normalize_text(raw)
        .trim_matches(|c: char| !c.is_ascii_alphabetic() && c != '\'')
        .to_string();
    if token.is_empty() {
        return token;
    }

    if dictionary.contains(&token) {
        return token;
    }

    let mut variants = Vec::new();
    if let Some(stem) = token.strip_suffix("ies") {
        variants.push(format!("{stem}y"));
    }
    if let Some(stem) = token.strip_suffix("ing") {
        push_inflection_variants(&mut variants, stem);
    }
    if let Some(stem) = token.strip_suffix("ed") {
        push_inflection_variants(&mut variants, stem);
    }
    if let Some(stem) = token.strip_suffix("es") {
        variants.push(stem.to_string());
    }
    if let Some(stem) = token.strip_suffix('s') {
        variants.push(stem.to_string());
    }

    for variant in variants {
        if dictionary.contains(&variant) {
            return variant;
        }
    }

    token
}

fn push_inflection_variants(variants: &mut Vec<String>, stem: &str) {
    variants.push(stem.to_string());
    variants.push(format!("{stem}e"));

    let mut chars = stem.chars().collect::<Vec<_>>();
    if chars.len() >= 2 && chars[chars.len() - 1] == chars[chars.len() - 2] {
        chars.pop();
        variants.push(chars.into_iter().collect());
    }
}
