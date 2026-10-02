// Chat core — what every provider shares: the multi-turn history, the file /
// window context of the first message, and the dispatch to claude.rs or
// gemini.rs, which only know how to talk to their own API.
//
// Everything happens here rather than in the island: API keys never leave the
// Credential Manager, and file bytes never cross the IPC boundary.

use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::settings::{ChatProvider, Settings};
use crate::{claude, gemini, secrets};

pub const SYSTEM_PROMPT: &str = "You are Mochi, a personal AI assistant living at the top of the user's screen. \
You have web search access and can help with absolutely anything — research, coding, finding places, recommendations, tasks, questions. \
Respond in the user's language. Be thorough and complete — use as much detail as the task requires. \
No markdown formatting (no **, no ##, no bullet dashes). Use plain text with line breaks.";

/// Text and code files are inlined; anything larger is skipped, as on macOS.
const MAX_INLINE_TEXT: u64 = 200_000;

#[derive(Default)]
struct History {
    /// The API the messages below were written for — the formats differ.
    provider: Option<ChatProvider>,
    /// Bumped on every reset, so a reply that lands after one is not stored.
    generation: u64,
    /// Full multi-turn history in the provider's own format, tool blocks and
    /// thought signatures included.
    messages: Vec<Value>,
}

#[derive(Default)]
pub struct Chat {
    history: Mutex<History>,
}

impl Chat {
    pub fn reset(&self) {
        let mut h = self.history.lock().unwrap();
        h.messages.clear();
        h.provider = None;
        h.generation += 1;
    }

    /// Starts over when the provider changed since the last turn.
    /// Returns (is this the first message, current generation).
    fn begin(&self, provider: ChatProvider) -> (bool, u64) {
        let mut h = self.history.lock().unwrap();
        if h.provider != Some(provider) {
            h.messages.clear();
            h.provider = Some(provider);
            h.generation += 1;
        }
        (h.messages.is_empty(), h.generation)
    }

    fn push(&self, generation: u64, message: Value) -> Option<Vec<Value>> {
        let mut h = self.history.lock().unwrap();
        if h.generation != generation {
            return None;
        }
        h.messages.push(message);
        Some(h.messages.clone())
    }

    /// Drops the user message again, so the history matches what the model saw.
    fn undo(&self, generation: u64) {
        let mut h = self.history.lock().unwrap();
        if h.generation == generation {
            h.messages.pop();
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum ChatContext {
    File { name: String, path: String },
    Window { app_name: String, title: String, url: Option<String> },
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatReply {
    pub text: String,
}

/// A dropped file, read once and handed to the provider to wrap in its format.
pub enum FilePayload {
    /// PDF or image, base64-encoded.
    Binary { kind: BinaryKind, mime: &'static str, data: String },
    /// Text or code, inlined.
    Text(String),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BinaryKind {
    Document,
    Image,
}

/// What one API call came back with.
pub enum Outcome {
    /// `stored` goes into the history as is; `text` is what the island shows.
    Reply { stored: Value, text: String },
    /// A policy decline, with the provider's explanation or a default.
    Refusal(String),
}

/// One chat turn. Returns the assistant's text, or a message the island shows
/// in the note view.
pub async fn send(
    chat: &Chat,
    settings: &Settings,
    query: String,
    context: Option<ChatContext>,
) -> Result<ChatReply, String> {
    let provider = settings.chat_provider;
    let key_name = match provider {
        ChatProvider::Anthropic => "anthropic-api-key",
        ChatProvider::Gemini => "gemini-api-key",
    };
    let key = secrets::get(key_name)
        .ok_or_else(|| crate::i18n::t("API key missing. Open settings.", "Falta a chave da API. Abra os Ajustes.").to_string())?;

    let (first, generation) = chat.begin(provider);

    // File / window context rides along with the first message only, exactly
    // like ClaudeService.chat().
    let mut file = None;
    let mut texts = Vec::new();
    if first {
        match &context {
            Some(ChatContext::File { name, path }) => {
                file = read_file(path);
                texts.push(format!("File: {name}"));
            }
            Some(ChatContext::Window { app_name, title, url }) => {
                let mut text = format!("Context — App: {app_name}, Window: {title}");
                if let Some(url) = url {
                    text.push_str(&format!(", URL: {url}"));
                }
                texts.push(text);
            }
            None => {}
        }
    }
    texts.push(query);

    let message = match provider {
        ChatProvider::Anthropic => claude::user_message(file.as_ref(), &texts),
        ChatProvider::Gemini => gemini::user_message(file.as_ref(), &texts),
    };
    let Some(history) = chat.push(generation, message) else {
        return Err(crate::i18n::t("The chat was reset.", "O chat foi reiniciado.").into());
    };

    let outcome = match provider {
        ChatProvider::Anthropic => claude::call(&key, &settings.model, &history).await,
        ChatProvider::Gemini => gemini::call(&key, &settings.gemini_model, &history).await,
    };

    match outcome {
        Err(err) => {
            chat.undo(generation);
            Err(err)
        }
        Ok(Outcome::Refusal(why)) => {
            chat.undo(generation);
            Err(why)
        }
        Ok(Outcome::Reply { stored, text }) => {
            if text.is_empty() {
                chat.undo(generation);
                return Err(crate::i18n::t("No response text.", "A resposta veio sem texto.").into());
            }
            // Ignored when the chat was reset while waiting (provider switch).
            chat.push(generation, stored);
            Ok(ChatReply { text })
        }
    }
}

pub fn http_client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(90))
        .build()
        .map_err(|e| e.to_string())
}

pub fn network_error(e: reqwest::Error) -> String {
    if crate::i18n::pt() { format!("Erro de rede: {e}") } else { format!("Network error: {e}") }
}

/// Reads a 2xx body as JSON, or turns an error into "<label> API <status>: <message>".
/// Both APIs put their message at error.message, which is what makes a bad key obvious.
pub async fn read_json(label: &str, response: reqwest::Response) -> Result<Value, String> {
    let status = response.status();
    let text = response.text().await.map_err(|e| e.to_string())?;
    if !status.is_success() {
        let detail = serde_json::from_str::<Value>(&text)
            .ok()
            .and_then(|v| {
                v.get("error")
                    .and_then(|e| e.get("message"))
                    .and_then(Value::as_str)
                    .map(str::to_string)
            })
            .unwrap_or_else(|| text.chars().take(200).collect());
        return Err(format!("{label} API {status}: {detail}"));
    }
    serde_json::from_str(&text).map_err(|e| {
        if crate::i18n::pt() { format!("Resposta inválida da API: {e}") } else { format!("Bad API response: {e}") }
    })
}

/// PDF → document, image → image, text/code → inline text.
/// Mirrors readFileAsBlock() in ClaudeService.swift.
fn read_file(path: &str) -> Option<FilePayload> {
    let ext = std::path::Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    let binary = match ext.as_str() {
        "pdf" => Some((BinaryKind::Document, "application/pdf")),
        "jpg" | "jpeg" => Some((BinaryKind::Image, "image/jpeg")),
        "png" => Some((BinaryKind::Image, "image/png")),
        "gif" => Some((BinaryKind::Image, "image/gif")),
        "webp" => Some((BinaryKind::Image, "image/webp")),
        _ => None,
    };

    if let Some((kind, mime)) = binary {
        let bytes = std::fs::read(path).ok()?;
        return Some(FilePayload::Binary { kind, mime, data: base64(&bytes) });
    }

    let len = std::fs::metadata(path).ok()?.len();
    if len > MAX_INLINE_TEXT {
        return None;
    }
    let text = std::fs::read_to_string(path).ok()?;
    Some(FilePayload::Text(format!("File contents:\n{text}")))
}

/// Small standalone base64 encoder — not worth another dependency.
/// Also used for Stripe's basic auth.
pub(crate) fn base64_for(bytes: &[u8]) -> String {
    base64(bytes)
}

fn base64(bytes: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b = [chunk[0], *chunk.get(1).unwrap_or(&0), *chunk.get(2).unwrap_or(&0)];
        let n = ((b[0] as u32) << 16) | ((b[1] as u32) << 8) | b[2] as u32;
        out.push(TABLE[(n >> 18) as usize & 63] as char);
        out.push(TABLE[(n >> 12) as usize & 63] as char);
        out.push(if chunk.len() > 1 { TABLE[(n >> 6) as usize & 63] as char } else { '=' });
        out.push(if chunk.len() > 2 { TABLE[n as usize & 63] as char } else { '=' });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn base64_matches_rfc4648_vectors() {
        assert_eq!(base64(b""), "");
        assert_eq!(base64(b"f"), "Zg==");
        assert_eq!(base64(b"fo"), "Zm8=");
        assert_eq!(base64(b"foo"), "Zm9v");
        assert_eq!(base64(b"foob"), "Zm9vYg==");
        assert_eq!(base64(b"fooba"), "Zm9vYmE=");
        assert_eq!(base64(b"foobar"), "Zm9vYmFy");
    }

    #[test]
    fn switching_provider_starts_over() {
        let chat = Chat::default();
        let (first, gen) = chat.begin(ChatProvider::Anthropic);
        assert!(first);
        chat.push(gen, json!({ "role": "user" }));
        chat.push(gen, json!({ "role": "assistant" }));

        let (first, _) = chat.begin(ChatProvider::Anthropic);
        assert!(!first, "same provider keeps the conversation");

        let (first, new_gen) = chat.begin(ChatProvider::Gemini);
        assert!(first, "new provider starts from an empty history");
        assert_ne!(gen, new_gen);
    }

    #[test]
    fn reply_after_reset_is_not_stored() {
        let chat = Chat::default();
        let (_, gen) = chat.begin(ChatProvider::Gemini);
        chat.push(gen, json!({ "role": "user" }));
        chat.reset();
        assert!(chat.push(gen, json!({ "role": "model" })).is_none());
        chat.undo(gen); // must not touch the new history either
        let (first, _) = chat.begin(ChatProvider::Gemini);
        assert!(first);
    }
}
