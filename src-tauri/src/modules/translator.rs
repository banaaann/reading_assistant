use std::{collections::HashSet, time::Duration};

use anyhow::Context;
use encoding_rs::GB18030;
use once_cell::sync::Lazy;
use regex::Regex;
use reqwest::blocking::Client;

use crate::modules::{
    dictionary::{lemmatize_word, tokenize_words, Dictionary},
    models::{DictionarySense, KeywordDefinition},
};

const NETWORK_TIMEOUT: Duration = Duration::from_millis(1200);

const STOP_WORDS: &[&str] = &[
    "a", "about", "after", "all", "also", "an", "and", "any", "are", "as", "at", "be", "been",
    "being", "but", "by", "can", "could", "did", "do", "does", "for", "from", "had", "has", "have",
    "he", "her", "his", "i", "if", "in", "into", "is", "it", "its", "may", "more", "most", "not",
    "of", "on", "or", "our", "she", "should", "so", "such", "than", "that", "the", "their", "them",
    "there", "these", "they", "this", "those", "to", "was", "we", "were", "which", "who", "will",
    "with", "would", "you", "your",
];

#[derive(Debug, Clone)]
pub struct OnlineWordResult {
    pub lemma: String,
    pub phonetic: Option<String>,
    pub senses: Vec<DictionarySense>,
}

static HTTP_CLIENT: Lazy<anyhow::Result<Client>> = Lazy::new(|| {
    Client::builder()
        .timeout(NETWORK_TIMEOUT)
        .user_agent("ReadingAssistantPro/0.1")
        .build()
        .context("failed to initialize network client")
});

static TRANSLATION_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"<ul id="translateResult">\s*<li>(.*?)</li>"#)
        .expect("translation regex should compile")
});

static PHONETIC_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"<span class="phonetic">\[(.*?)\]</span>"#).expect("phonetic regex should compile")
});

static SENSE_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"(?s)<div id="ec".*?<ul>(.*?)</ul>"#).expect("sense regex should compile")
});

static ITEM_PATTERN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r#"<li>(.*?)</li>"#).expect("item regex should compile"));

fn http_client() -> anyhow::Result<Client> {
    HTTP_CLIENT
        .as_ref()
        .map(Clone::clone)
        .map_err(|error| anyhow::anyhow!(error.to_string()))
}

pub fn online_translate_text(text: &str) -> anyhow::Result<Option<String>> {
    let client = http_client()?;
    let response = client
        .post("https://m.youdao.com/translate")
        .form(&[("inputtext", text), ("type", "EN2ZH_CN")])
        .send();

    let Ok(response) = response else {
        return Ok(None);
    };
    if !response.status().is_success() {
        return Ok(None);
    }

    let html =
        decode_response_text(response).context("failed to decode youdao translation page")?;
    let html = clean_html_text(&html);
    let translated = TRANSLATION_PATTERN
        .captures(&html)
        .and_then(|captures| captures.get(1).map(|item| item.as_str().trim().to_string()));

    match translated {
        Some(translated_text) if !translated_text.is_empty() => Ok(Some(translated_text)),
        _ => Ok(None),
    }
}

pub fn local_translate_text(dictionary: &Dictionary, text: &str) -> String {
    let tokens = tokenize_words(text);
    if tokens.is_empty() {
        return "没有可翻译的内容。".to_string();
    }

    let translated = tokens
        .iter()
        .map(|token| {
            let lemma = lemmatize_word(dictionary, token);
            dictionary
                .get(&lemma)
                .and_then(local_primary_definition)
                .unwrap_or(token.as_str())
                .to_string()
        })
        .collect::<Vec<_>>();

    format!("本地兜底翻译：{}", translated.join(" "))
}

pub fn online_lookup_word(word: &str) -> anyhow::Result<Option<OnlineWordResult>> {
    let client = http_client()?;
    let response = client
        .get("https://m.youdao.com/dict")
        .query(&[("le", "eng"), ("q", word)])
        .send();

    let Ok(response) = response else {
        return Ok(None);
    };
    if !response.status().is_success() {
        return Ok(None);
    }

    let html = decode_response_text(response).context("failed to decode youdao dictionary page")?;
    let html = clean_html_text(&html);

    let phonetic = PHONETIC_PATTERN.captures(&html).and_then(|captures| {
        captures
            .get(1)
            .map(|item| format!("[{}]", item.as_str().trim()))
    });

    let Some(list_html) = SENSE_PATTERN
        .captures(&html)
        .and_then(|captures| captures.get(1).map(|item| item.as_str().to_string()))
    else {
        return Ok(None);
    };

    let mut senses = Vec::new();
    for capture in ITEM_PATTERN.captures_iter(&list_html) {
        let Some(item) = capture.get(1) else {
            continue;
        };
        let line = html_entity_decode(item.as_str().trim());
        if line.is_empty() {
            continue;
        }

        let (part_of_speech, definitions) = split_youdao_definition(&line);
        if definitions.is_empty() {
            continue;
        }

        senses.push(DictionarySense {
            part_of_speech,
            definitions,
        });
    }

    if senses.is_empty() {
        return Ok(None);
    }

    Ok(Some(OnlineWordResult {
        lemma: word.to_lowercase(),
        phonetic,
        senses,
    }))
}

pub fn extract_keywords(
    dictionary: &Dictionary,
    text: &str,
    limit: usize,
) -> Vec<KeywordDefinition> {
    let stop_words = STOP_WORDS.iter().copied().collect::<HashSet<_>>();
    let mut seen = HashSet::new();
    let mut keywords = Vec::new();
    let mut fallback_keywords = Vec::new();

    for token in tokenize_words(text) {
        if token.len() < 3 || stop_words.contains(token.as_str()) {
            continue;
        }

        let lemma = lemmatize_word(dictionary, &token);
        if !seen.insert(lemma.clone()) {
            continue;
        }

        if let Some(entry) = dictionary.get(&lemma) {
            let sense = entry.senses.first();
            keywords.push(KeywordDefinition {
                word: lemma,
                part_of_speech: sense.map(|item| item.part_of_speech.clone()),
                brief_definition: sense
                    .and_then(|item| item.definitions.first())
                    .cloned()
                    .unwrap_or_else(|| "未找到释义".to_string()),
            });
        } else {
            fallback_keywords.push(KeywordDefinition {
                word: lemma,
                part_of_speech: None,
                brief_definition: "暂无本地释义".to_string(),
            });
        }

        if keywords.len() >= limit {
            break;
        }
    }

    for keyword in fallback_keywords {
        if keywords.len() >= limit {
            break;
        }
        keywords.push(keyword);
    }

    keywords
}

fn decode_response_text(response: reqwest::blocking::Response) -> anyhow::Result<String> {
    let bytes = response.bytes().context("failed to read response bytes")?;

    if let Ok(text) = String::from_utf8(bytes.to_vec()) {
        if !looks_mojibake(&text) {
            return Ok(text);
        }
    }

    let (decoded, _, _) = GB18030.decode(&bytes);
    Ok(decoded.into_owned())
}

fn looks_mojibake(text: &str) -> bool {
    let markers = ["锟", "閿", "鍙", "鐨", "寮", "璇", "鏂", "銆", "鈥"];
    markers.iter().any(|item| text.contains(item))
}

fn split_youdao_definition(line: &str) -> (String, Vec<String>) {
    let normalized = line.replace("&nbsp;", " ").replace('；', ";");
    let (pos, detail) = if let Some((head, tail)) = normalized.split_once(' ') {
        (normalize_part_of_speech(head), tail.trim())
    } else {
        ("释义".to_string(), normalized.trim())
    };

    let definitions = detail
        .split([';', '；'])
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();

    (pos, definitions)
}

fn normalize_part_of_speech(raw: &str) -> String {
    match raw.trim().trim_end_matches('.') {
        "n" => "名词".to_string(),
        "v" | "vt" | "vi" => "动词".to_string(),
        "adj" => "形容词".to_string(),
        "adv" => "副词".to_string(),
        "prep" => "介词".to_string(),
        "pron" => "代词".to_string(),
        "conj" => "连词".to_string(),
        "aux" => "助动词".to_string(),
        "num" => "数词".to_string(),
        "int" => "感叹词".to_string(),
        "art" => "冠词".to_string(),
        _ => raw.trim().to_string(),
    }
}

fn clean_html_text(input: &str) -> String {
    input.replace('\r', "").replace('\n', "")
}

fn html_entity_decode(input: &str) -> String {
    input
        .replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
}

fn local_primary_definition(entry: &crate::modules::models::DictionaryEntry) -> Option<&str> {
    entry
        .senses
        .first()
        .and_then(|sense| sense.definitions.first())
        .map(|item| item.as_str())
}
