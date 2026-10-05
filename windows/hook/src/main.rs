//! coucou-hook — the relay Claude Code runs on every hook event.
//!
//! Reads the hook JSON on stdin, adds a little terminal context, and hands it to
//! Coucou over the named pipe `\\.\pipe\coucou-<sid>` (Windows) or the Unix
//! socket `$XDG_RUNTIME_DIR/coucou.sock` (Linux).
//!
//! Hard rule (docs/CLAUDE.md): **never block Claude Code.**
//! * If the pipe does not exist — Coucou is closed — we exit 0 immediately with
//!   nothing on stdout, and the session carries on untouched.
//! * Every fire-and-forget event runs under a deadline enforced by the main
//!   thread, so a pipe that accepts the connection and then stops reading cannot
//!   wedge the session either: we abandon the worker and exit.
//! * Only `PermissionRequest` waits for an answer, because approving — and
//!   answering Claude's questions — from the island is the whole point. That wait
//!   has no deadline of its own: Claude Code shows the same prompt in the terminal
//!   while we wait, takes whichever answer comes first, and kills us when the
//!   terminal wins. No answer means empty stdout, exactly as if Coucou were not
//!   installed.
//!
//! Usage: `coucou-hook <EventName>` (the name is also read from the JSON).

use std::io::{Read, Write};
use std::sync::mpsc;
use std::time::Duration;

/// Budget for getting a pipe connection. Beyond this Claude Code wins, always.
const CONNECT_TIMEOUT: Duration = Duration::from_millis(300);
/// Whole-run budget for an event nobody waits on: connect and write, no more.
const FIRE_AND_FORGET_BUDGET: Duration = Duration::from_secs(2);

/// Fields that are pointless to forward and can be enormous (a whole file read,
/// a full command output). The island never shows them.
const DROPPED_FIELDS: &[&str] = &["tool_response", "transcript_path"];
/// Longest string forwarded for any single field; the island truncates to far
/// less than this anyway.
const MAX_FIELD_LEN: usize = 2_000;

#[cfg(windows)]
mod win;
#[cfg(windows)]
use win::connect;

#[cfg(target_os = "linux")]
mod unix;
#[cfg(target_os = "linux")]
use unix::connect;

fn main() {
    let Some(Event { payload, name, questions }) = read_event() else { std::process::exit(0) };

    let waits_for_answer = name == "PermissionRequest";

    // The worker owns every blocking call. If it overruns the budget we simply
    // stop listening and exit: the process dying takes the pipe handle with it.
    // (No catch_unwind here — the release profile is panic = "abort", so it would
    // be dead code. `talk` is written to have nothing to panic on instead.)
    let (tx, rx) = mpsc::channel::<Option<String>>();
    std::thread::spawn(move || {
        let _ = tx.send(talk(&payload, waits_for_answer));
    });

    // A permission request waits as long as the island needs: the worker ends on
    // its own when Coucou answers, declines or closes the pipe. Everything else
    // gets the short budget.
    let reply = if waits_for_answer {
        rx.recv().ok()
    } else {
        rx.recv_timeout(FIRE_AND_FORGET_BUDGET).ok()
    };
    if let Some(Some(decision)) = reply {
        if let Some(json) = decision_json(&decision, questions.as_ref()) {
            let mut out = std::io::stdout();
            let _ = writeln!(out, "{json}");
            let _ = out.flush();
        }
    }
    // Nothing printed: Claude Code asks in the terminal, as if we were not here.
    std::process::exit(0);
}

/// The documented PermissionRequest output. Anything we do not recognise prints
/// nothing at all rather than guessing — silence is the safe answer.
/// See https://code.claude.com/docs/en/hooks
///
/// `questions` is the untouched `AskUserQuestion` input, present only for that
/// tool; an `answers {…}` reply is only ever accepted against it.
fn decision_json(decision: &str, questions: Option<&serde_json::Value>) -> Option<String> {
    let decision = decision.trim();
    let behavior = match decision {
        // "always" still answers a plain allow; remembering it is the island's
        // business, not Claude Code's.
        "allow" | "always" => r#"{"behavior":"allow"}"#.to_string(),
        "deny" => r#"{"behavior":"deny","message":"Denied from Coucou"}"#.to_string(),
        _ => {
            let raw = decision.strip_prefix("answers ")?;
            return answers_json(raw, questions?);
        }
    };
    Some(format!(
        r#"{{"hookSpecificOutput":{{"hookEventName":"PermissionRequest","decision":{behavior}}}}}"#
    ))
}

/// Answers to an `AskUserQuestion` call: an allow whose `updatedInput` carries
/// the original questions plus `answers` (question text → chosen label, labels
/// joined by ", " for a multi-select, or the user's own words).
/// See https://code.claude.com/docs/en/agent-sdk/user-input
///
/// Every question must be answered, every key must be one of the questions and
/// every answer must be a non-empty string — otherwise nothing is printed and the
/// terminal asks instead.
fn answers_json(raw: &str, questions: &serde_json::Value) -> Option<String> {
    let answers = serde_json::from_str::<serde_json::Value>(raw).ok()?;
    let answers = answers.as_object()?;
    let texts: Vec<&str> = questions
        .as_array()?
        .iter()
        .map(|q| q.get("question").and_then(|v| v.as_str()))
        .collect::<Option<_>>()?;
    let complete = !texts.is_empty()
        && answers.len() == texts.len()
        && texts.iter().all(|t| {
            answers
                .get(*t)
                .and_then(|v| v.as_str())
                .is_some_and(|v| !v.trim().is_empty())
        });
    if !complete {
        return None;
    }
    let out = serde_json::json!({
        "hookSpecificOutput": {
            "hookEventName": "PermissionRequest",
            "decision": {
                "behavior": "allow",
                "updatedInput": { "questions": questions, "answers": answers },
            },
        },
    });
    Some(out.to_string())
}

/// One hook event, ready to forward.
struct Event {
    /// The JSON line sent to Coucou (bulky fields dropped, strings capped).
    payload: String,
    name: String,
    /// `AskUserQuestion`'s questions exactly as Claude Code sent them, before any
    /// truncation: the answer has to hand them back unchanged.
    questions: Option<serde_json::Value>,
}

/// Reads stdin and returns the payload to forward plus the event name.
fn read_event() -> Option<Event> {
    let mut raw = Vec::new();
    if std::io::stdin().read_to_end(&mut raw).is_err() || raw.is_empty() {
        return None;
    }
    // Some shells hand us a UTF-8 BOM; serde_json would choke on it.
    if raw.starts_with(&[0xEF, 0xBB, 0xBF]) {
        raw.drain(..3);
    }

    let mut payload = serde_json::from_slice::<serde_json::Value>(&raw).ok()?;
    let map = payload.as_object_mut()?;

    // Parse argv: "coucou-hook.exe [--agent <name>] [<EventName>]"
    // --agent tags the payload with coucou_agent so the app routes to the right pill.
    // Absent or invalid names are validated and discarded by the app, not here.
    let mut agent = String::new();
    let mut arg_event = String::new();
    {
        let mut it = std::env::args().skip(1);
        while let Some(arg) = it.next() {
            if arg == "--agent" {
                agent = it.next().unwrap_or_default();
            } else if arg_event.is_empty() {
                arg_event = arg;
            }
        }
    }
    // Which agent this hook was installed for. Absent means Claude Code,
    // so existing hook commands keep working unchanged.
    if !agent.is_empty() {
        map.insert("coucou_agent".into(), serde_json::Value::String(agent));
    }
    let event = map
        .get("hook_event_name")
        .and_then(|v| v.as_str())
        .map(str::to_string)
        .filter(|s| !s.is_empty())
        .unwrap_or(arg_event);
    map.insert("hook_event_name".into(), serde_json::Value::String(event.clone()));

    for field in DROPPED_FIELDS {
        map.remove(*field);
    }

    let cwd_missing = map
        .get("cwd")
        .and_then(|v| v.as_str())
        .map(str::is_empty)
        .unwrap_or(true);
    if cwd_missing {
        if let Ok(cwd) = std::env::current_dir() {
            map.insert(
                "cwd".into(),
                serde_json::Value::String(cwd.to_string_lossy().to_string()),
            );
        }
    }

    // Which terminal the session runs in. Unlike macOS, Coucou here accepts
    // events from every terminal, so this is context only — never a filter.
    for (key, var) in [
        ("term_program", "TERM_PROGRAM"),
        ("wt_session", "WT_SESSION"),
        ("term_session_id", "TERM_SESSION_ID"),
        ("vscode_pid", "VSCODE_PID"),
        ("session_pid", "CLAUDE_CODE_SSE_PORT"),
    ] {
        if !map.contains_key(key) {
            let value = std::env::var(var).unwrap_or_default();
            map.insert(key.into(), serde_json::Value::String(value));
        }
    }

    let questions = (map.get("tool_name").and_then(|v| v.as_str()) == Some("AskUserQuestion"))
        .then(|| map.get("tool_input").and_then(|i| i.get("questions")).cloned())
        .flatten();

    truncate_strings(&mut payload);

    let mut line = payload.to_string();
    line.push('\n');
    Some(Event { payload: line, name: event, questions })
}

/// Caps every string in the payload. A single Write can carry a whole file.
fn truncate_strings(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::String(s) => {
            if s.len() > MAX_FIELD_LEN {
                // Cut on a char boundary; a lone byte index can split UTF-8.
                let mut end = MAX_FIELD_LEN;
                while end > 0 && !s.is_char_boundary(end) {
                    end -= 1;
                }
                s.truncate(end);
                s.push('…');
            }
        }
        serde_json::Value::Array(items) => items.iter_mut().for_each(truncate_strings),
        serde_json::Value::Object(map) => map.values_mut().for_each(truncate_strings),
        _ => {}
    }
}

/// Connect, send, and — for a permission request — wait for the island's word.
fn talk(payload: &str, waits_for_answer: bool) -> Option<String> {
    let mut pipe = connect()?;

    if pipe.write_all(payload.as_bytes()).is_err() {
        return None;
    }
    let _ = pipe.flush();

    if !waits_for_answer {
        return None;
    }

    let mut buf = Vec::new();
    let mut chunk = [0u8; 1024];
    loop {
        match pipe.read(&mut chunk) {
            Ok(0) => break,
            Ok(n) => {
                buf.extend_from_slice(&chunk[..n]);
                if buf.contains(&b'\n') {
                    break;
                }
            }
            Err(_) => break,
        }
    }
    let answer = String::from_utf8_lossy(&buf).trim().to_string();
    (!answer.is_empty()).then_some(answer)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decision_json_matches_the_documented_shape() {
        assert_eq!(
            decision_json("allow", None).unwrap(),
            r#"{"hookSpecificOutput":{"hookEventName":"PermissionRequest","decision":{"behavior":"allow"}}}"#
        );
        assert_eq!(
            decision_json("deny", None).unwrap(),
            r#"{"hookSpecificOutput":{"hookEventName":"PermissionRequest","decision":{"behavior":"deny","message":"Denied from Coucou"}}}"#
        );
        // "always" is an island concept; Claude Code just gets an allow.
        assert!(decision_json("always", None).unwrap().contains(r#""behavior":"allow""#));
    }

    #[test]
    fn anything_unrecognised_prints_nothing() {
        assert!(decision_json("", None).is_none());
        assert!(decision_json("maybe", None).is_none());
        // The shape the app used to send must not be mistaken for a decision.
        assert!(decision_json(r#"{"permissionDecision":"allow"}"#, None).is_none());
    }

    fn two_questions() -> serde_json::Value {
        serde_json::json!([
            { "question": "Qual cor?", "header": "Cor", "multiSelect": false,
              "options": [{ "label": "Azul", "description": "" }, { "label": "Verde", "description": "" }] },
            { "question": "Quais \"frutas\"?", "header": "Frutas", "multiSelect": true,
              "options": [{ "label": "Maçã", "description": "" }, { "label": "Uva", "description": "" }] },
        ])
    }

    #[test]
    fn answers_become_the_documented_updated_input() {
        let q = two_questions();
        let reply = r#"answers {"Qual cor?":"Verde","Quais \"frutas\"?":"Maçã, Uva"}"#;
        let out: serde_json::Value =
            serde_json::from_str(&decision_json(reply, Some(&q)).unwrap()).unwrap();
        let d = &out["hookSpecificOutput"]["decision"];
        assert_eq!(out["hookSpecificOutput"]["hookEventName"], "PermissionRequest");
        assert_eq!(d["behavior"], "allow");
        assert_eq!(d["updatedInput"]["questions"], q);
        assert_eq!(d["updatedInput"]["answers"]["Qual cor?"], "Verde");
        assert_eq!(d["updatedInput"]["answers"]["Quais \"frutas\"?"], "Maçã, Uva");
    }

    #[test]
    fn incomplete_or_foreign_answers_print_nothing() {
        let q = two_questions();
        // Without the original questions there is nothing to answer.
        assert!(decision_json(r#"answers {"Qual cor?":"Verde"}"#, None).is_none());
        // One question left unanswered.
        assert!(decision_json(r#"answers {"Qual cor?":"Verde"}"#, Some(&q)).is_none());
        // A key that is not one of the questions.
        let foreign = r#"answers {"Qual cor?":"Verde","Outra?":"x"}"#;
        assert!(decision_json(foreign, Some(&q)).is_none());
        // An empty answer.
        let empty = r#"answers {"Qual cor?":" ","Quais \"frutas\"?":"Uva"}"#;
        assert!(decision_json(empty, Some(&q)).is_none());
        // Not JSON at all.
        assert!(decision_json("answers nope", Some(&q)).is_none());
    }

    #[test]
    fn long_strings_are_cut_on_a_char_boundary() {
        let mut v = serde_json::json!({ "tool_input": { "content": "é".repeat(4000) } });
        truncate_strings(&mut v);
        let s = v["tool_input"]["content"].as_str().unwrap();
        assert!(s.len() <= MAX_FIELD_LEN + 4);
        assert!(s.ends_with('…'));
    }
}
