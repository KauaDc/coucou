// Gemini API (Google AI Studio key) — generateContent with Google Search
// grounding, files as inlineData. History and context live in chat.rs.

use std::sync::atomic::{AtomicBool, Ordering};

use reqwest::StatusCode;
use serde_json::{json, Value};

use crate::chat::{self, FilePayload, Outcome, SYSTEM_PROMPT};

const BASE: &str = "https://generativelanguage.googleapis.com/v1beta/models";
/// inlineData rides inside the request, which the API caps at 20 MB; a bigger
/// file is skipped like an oversized text file, instead of failing the turn.
const MAX_INLINE_BASE64: usize = 18 * 1024 * 1024;

/// Stable per ai.google.dev/gemini-api/docs/models (2026-10-02).
pub const DEFAULT_MODEL: &str = "gemini-3.8-flash";

/// finishReason values that mean the answer was blocked, not cut short.
const BLOCKED: &[&str] = &["SAFETY", "PROHIBITED_CONTENT", "BLOCKLIST", "SPII", "RECITATION"];

/// The user turn: the file part first, then one text part per text.
pub fn user_message(file: Option<&FilePayload>, texts: &[String]) -> Value {
    let mut parts: Vec<Value> = Vec::new();
    match file {
        Some(FilePayload::Binary { mime, data, .. }) if data.len() <= MAX_INLINE_BASE64 => {
            parts.push(json!({ "inlineData": { "mimeType": mime, "data": data } }));
        }
        Some(FilePayload::Text(text)) => parts.push(json!({ "text": text })),
        _ => {}
    }
    for text in texts {
        parts.push(json!({ "text": text }));
    }
    json!({ "role": "user", "parts": parts })
}

/// The model id goes into the URL path, so only plain ids are accepted.
fn valid_model(model: &str) -> bool {
    (1..=64).contains(&model.len())
        && model.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'))
}

/// Set once Google Search grounding hit its quota while plain requests still
/// went through. Grounding has its own, much smaller quota (zero on some free
/// projects), so from then on the chat goes without it until the app restarts
/// instead of spending two requests per message.
static SEARCH_OFF: AtomicBool = AtomicBool::new(false);

const NO_SEARCH_NOTE: &str = "Web search is unavailable right now. Answer from what you know, and when the question needs current or live information, say that you cannot check the web at the moment.";

pub async fn call(key: &str, model: &str, history: &[Value]) -> Result<Outcome, String> {
    if !valid_model(model) {
        return Err(crate::i18n::t("Invalid model.", "Modelo inválido.").into());
    }

    let with_search = !SEARCH_OFF.load(Ordering::Relaxed);
    let response = post(key, model, &body(history, with_search)).await?;

    let response = if with_search && response.status() == StatusCode::TOO_MANY_REQUESTS {
        // One retry without the search tool. If that is rate limited too, the
        // general quota is the problem and its message is what gets shown.
        let retry = post(key, model, &body(history, false)).await?;
        if retry.status().is_success() {
            SEARCH_OFF.store(true, Ordering::Relaxed);
            crate::log::line("gemini: search grounding quota reached, chat continues without it");
        }
        retry
    } else {
        response
    };

    let response = chat::read_json("Gemini", response).await?;
    parse(&response)
}

fn body(history: &[Value], with_search: bool) -> Value {
    // No maxOutputTokens: on thinking models the thoughts count against it, and
    // a low cap leaves a reply with no text at all.
    let system = if with_search {
        SYSTEM_PROMPT.to_string()
    } else {
        // The shared prompt promises web search; without the tool the model would
        // otherwise claim to look things up and answer from memory as if it had.
        format!("{SYSTEM_PROMPT} {NO_SEARCH_NOTE}")
    };
    let mut body = json!({
        "systemInstruction": { "parts": [{ "text": system }] },
        "contents": history,
    });
    if with_search {
        body["tools"] = json!([{ "google_search": {} }]);
    }
    body
}

async fn post(key: &str, model: &str, body: &Value) -> Result<reqwest::Response, String> {
    // The key goes in a header, never in the query string, so it cannot end up
    // in a logged URL.
    chat::http_client()?
        .post(format!("{BASE}/{model}:generateContent"))
        .header("x-goog-api-key", key)
        .header("content-type", "application/json")
        .json(body)
        .send()
        .await
        .map_err(chat::network_error)
}

fn parse(response: &Value) -> Result<Outcome, String> {
    let declined = || Outcome::Refusal(crate::i18n::t("Gemini declined this one.", "O Gemini recusou este pedido.").into());

    if response.pointer("/promptFeedback/blockReason").is_some() {
        return Ok(declined());
    }
    let Some(candidate) = response.pointer("/candidates/0") else {
        return Err(crate::i18n::t("Unexpected API response.", "Resposta inesperada da API.").into());
    };
    if let Some(reason) = candidate.get("finishReason").and_then(Value::as_str) {
        if BLOCKED.contains(&reason) {
            return Ok(declined());
        }
    }

    // Keep the content exactly as it came — Gemini 3 checks the thoughtSignature
    // parts on the next turn.
    let mut stored = candidate.get("content").cloned().unwrap_or_else(|| json!({ "parts": [] }));
    if stored.get("role").is_none() {
        stored["role"] = json!("model");
    }

    let text = stored
        .get("parts")
        .and_then(Value::as_array)
        .map(|parts| {
            parts
                .iter()
                .filter(|p| p.get("thought").and_then(Value::as_bool) != Some(true))
                .filter_map(|p| p.get("text").and_then(Value::as_str))
                .collect::<Vec<_>>()
                .join("")
        })
        .unwrap_or_default()
        .trim()
        .to_string();

    Ok(Outcome::Reply { stored, text })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chat::BinaryKind;

    #[test]
    fn user_message_puts_the_file_first() {
        let file = FilePayload::Binary { kind: BinaryKind::Document, mime: "application/pdf", data: "QUJD".into() };
        let msg = user_message(Some(&file), &["File: a.pdf".into(), "what is this?".into()]);
        assert_eq!(msg["role"], "user");
        assert_eq!(msg["parts"][0]["inlineData"]["mimeType"], "application/pdf");
        assert_eq!(msg["parts"][0]["inlineData"]["data"], "QUJD");
        assert_eq!(msg["parts"][1]["text"], "File: a.pdf");
        assert_eq!(msg["parts"][2]["text"], "what is this?");
    }

    #[test]
    fn oversized_binary_is_skipped() {
        let file = FilePayload::Binary { kind: BinaryKind::Image, mime: "image/png", data: "A".repeat(MAX_INLINE_BASE64 + 1) };
        let msg = user_message(Some(&file), &["hi".into()]);
        assert_eq!(msg["parts"].as_array().unwrap().len(), 1);
        assert_eq!(msg["parts"][0]["text"], "hi");
    }

    #[test]
    fn text_skips_thoughts_and_keeps_signatures() {
        let response = json!({ "candidates": [{
            "finishReason": "STOP",
            "content": { "role": "model", "parts": [
                { "text": "thinking…", "thought": true },
                { "text": "Hello", "thoughtSignature": "sig" },
                { "text": " world" },
            ]},
        }]});
        let Ok(Outcome::Reply { stored, text }) = parse(&response) else { panic!("expected a reply") };
        assert_eq!(text, "Hello world");
        assert_eq!(stored["parts"][1]["thoughtSignature"], "sig");
        assert_eq!(stored["parts"].as_array().unwrap().len(), 3);
    }

    #[test]
    fn missing_role_is_filled_in() {
        let response = json!({ "candidates": [{ "content": { "parts": [{ "text": "ok" }] } }] });
        let Ok(Outcome::Reply { stored, .. }) = parse(&response) else { panic!("expected a reply") };
        assert_eq!(stored["role"], "model");
    }

    #[test]
    fn blocks_are_refusals() {
        let prompt = json!({ "promptFeedback": { "blockReason": "SAFETY" } });
        assert!(matches!(parse(&prompt), Ok(Outcome::Refusal(_))));
        let answer = json!({ "candidates": [{ "finishReason": "PROHIBITED_CONTENT" }] });
        assert!(matches!(parse(&answer), Ok(Outcome::Refusal(_))));
    }

    #[test]
    fn cut_short_answer_still_shows_its_text() {
        let response = json!({ "candidates": [{
            "finishReason": "MAX_TOKENS",
            "content": { "role": "model", "parts": [{ "text": "partial" }] },
        }]});
        let Ok(Outcome::Reply { text, .. }) = parse(&response) else { panic!("expected a reply") };
        assert_eq!(text, "partial");
    }

    #[test]
    fn search_tool_only_when_asked() {
        let history = [json!({ "role": "user", "parts": [{ "text": "hi" }] })];
        let on = body(&history, true);
        assert_eq!(on["tools"][0], json!({ "google_search": {} }));
        assert_eq!(on["contents"][0]["parts"][0]["text"], "hi");
        let off = body(&history, false);
        assert!(off.get("tools").is_none());
        let on_prompt = on["systemInstruction"]["parts"][0]["text"].as_str().unwrap();
        let off_prompt = off["systemInstruction"]["parts"][0]["text"].as_str().unwrap();
        assert_eq!(on_prompt, SYSTEM_PROMPT);
        assert!(off_prompt.starts_with(SYSTEM_PROMPT) && off_prompt.ends_with(NO_SEARCH_NOTE));
    }

    #[test]
    fn no_candidates_is_an_error() {
        assert!(parse(&json!({})).is_err());
    }

    #[test]
    fn model_ids_are_checked() {
        assert!(valid_model("gemini-3.8-flash"));
        assert!(valid_model("gemini-3.1-pro-preview"));
        assert!(!valid_model(""));
        assert!(!valid_model("../files"));
        assert!(!valid_model("gemini:generateContent?key=x"));
        assert!(!valid_model(&"a".repeat(65)));
    }
}
