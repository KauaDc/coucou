// Claude API — the same integration as ClaudeService.swift: web search, files
// as document/image/text blocks. History and context live in chat.rs.

use serde_json::{json, Value};

use crate::chat::{self, BinaryKind, FilePayload, Outcome, SYSTEM_PROMPT};

const ENDPOINT: &str = "https://api.anthropic.com/v1/messages";
const ANTHROPIC_VERSION: &str = "2023-06-01";
/// Server-side fallback: on a policy decline the API retries the same request on
/// a fallback model inside the same call, so the island never shows a dead end.
const FALLBACK_BETA: &str = "server-side-fallback-2026-07-01";
const MAX_TOKENS: u32 = 4096;

pub const DEFAULT_MODEL: &str = "claude-opus-5";

/// The user turn: the file block first, then one text block per text.
pub fn user_message(file: Option<&FilePayload>, texts: &[String]) -> Value {
    let mut content: Vec<Value> = Vec::new();
    match file {
        Some(FilePayload::Binary { kind, mime, data }) => {
            let block_type = if *kind == BinaryKind::Document { "document" } else { "image" };
            content.push(json!({
                "type": block_type,
                "source": { "type": "base64", "media_type": mime, "data": data },
            }));
        }
        Some(FilePayload::Text(text)) => content.push(json!({ "type": "text", "text": text })),
        None => {}
    }
    for text in texts {
        content.push(json!({ "type": "text", "text": text }));
    }
    json!({ "role": "user", "content": content })
}

pub async fn call(key: &str, model: &str, history: &[Value]) -> Result<Outcome, String> {
    let body = json!({
        "model": model,
        "max_tokens": MAX_TOKENS,
        "system": SYSTEM_PROMPT,
        "tools": [{ "type": "web_search_20260209", "name": "web_search", "max_uses": 5 }],
        "fallbacks": "default",
        "messages": history,
    });

    let response = chat::http_client()?
        .post(ENDPOINT)
        .header("x-api-key", key)
        .header("anthropic-version", ANTHROPIC_VERSION)
        .header("anthropic-beta", FALLBACK_BETA)
        .header("content-type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(chat::network_error)?;
    let response = chat::read_json("Claude", response).await?;

    // A policy decline comes back as HTTP 200 with stop_reason "refusal".
    if response.get("stop_reason").and_then(Value::as_str) == Some("refusal") {
        let why = response
            .get("stop_details")
            .and_then(|d| d.get("explanation"))
            .and_then(Value::as_str)
            .unwrap_or(crate::i18n::t("Claude declined this one.", "O Claude recusou este pedido."));
        return Ok(Outcome::Refusal(why.to_string()));
    }

    let Some(blocks) = response.get("content").and_then(Value::as_array).cloned() else {
        return Err(crate::i18n::t("Unexpected API response.", "Resposta inesperada da API.").into());
    };

    let text = blocks
        .iter()
        .filter(|b| b.get("type").and_then(Value::as_str) == Some("text"))
        .filter_map(|b| b.get("text").and_then(Value::as_str))
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string();

    // Store the whole content — tool_use / tool_result blocks included — so the
    // next turn has the right context.
    Ok(Outcome::Reply { stored: json!({ "role": "assistant", "content": blocks }), text })
}
