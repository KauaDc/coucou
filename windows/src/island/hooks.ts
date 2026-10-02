// Claude Code hook events → island state.
// Port of HookServer.processEvent / processPermissionRequest from the macOS app.
// Difference from macOS: no terminal filter. On Windows the hook fires from any
// terminal (Windows Terminal, VS Code, PowerShell…) and all of them are handled.

import { Bridge, onEvent } from "../core/bridge";
import { Sound } from "../core/sound";
import { State, type AskQuestion } from "../core/state";
import type { Island } from "./island";
import { t } from "../i18n";

const CLAUDE_ID = "integration_claude";

interface HookPayload {
  hook_event_name?: string;
  request_id?: string;
  session_id?: string;
  cwd?: string;
  message?: string;
  /** UserPromptSubmit carries `prompt`; `message` belongs to Notification/Stop. */
  prompt?: string;
  tool_name?: string;
  tool_input?: Record<string, unknown>;
}

const PROJECT_ALIASES: Record<string, string> = {
  "notch-buddy": "Notch Buddy",
  notchbuddy: "Notch Buddy",
  notch_buddy: "Notch Buddy",
};

function aliasProjectName(name: string): string {
  return PROJECT_ALIASES[name.toLowerCase()] ?? name;
}

function lastPathComponent(p: string): string {
  const cleaned = p.replace(/[\\/]+$/, "");
  const idx = Math.max(cleaned.lastIndexOf("\\"), cleaned.lastIndexOf("/"));
  return idx >= 0 ? cleaned.slice(idx + 1) : cleaned;
}

/** Step labels for the ticker, in the interface language. */
const TOOL_LABELS: Record<string, string> = {
  Bash: t("tool.Bash"),
  Read: t("tool.Read"),
  Write: t("tool.Write"),
  Edit: t("tool.Edit"),
  Glob: t("tool.Glob"),
  Grep: t("tool.Grep"),
  WebSearch: t("tool.WebSearch"),
  WebFetch: t("tool.WebFetch"),
  TodoWrite: t("tool.TodoWrite"),
  Task: t("tool.Task"),
  LS: t("tool.LS"),
  MultiEdit: t("tool.MultiEdit"),
  NotebookEdit: t("tool.NotebookEdit"),
  PowerShell: t("tool.PowerShell"),
};

function stepLabel(tool: string, input: Record<string, unknown>): string {
  const label = TOOL_LABELS[tool] ?? tool;
  const str = (k: string) => (typeof input[k] === "string" ? (input[k] as string) : null);
  const cmd = str("command");
  if (cmd) return `${label} · ${cmd.slice(0, 40)}`;
  const path = str("path");
  if (path) return `${label} · ${lastPathComponent(path)}`;
  const file = str("file_path");
  if (file) return `${label} · ${lastPathComponent(file)}`;
  const query = str("query");
  if (query) return `${label} · ${query.slice(0, 40)}`;
  return label;
}

/**
 * What the Allow button actually authorises. Approving "Write" tells you nothing
 * — approving `Write · C:\…\.env` tells you everything, and the difference is
 * the whole point of approving from the island rather than blind.
 *
 * Ordered by how specific the field is, so an unfamiliar tool still shows
 * whatever identifying string it carries instead of falling back to its name.
 */
const APPROVAL_FIELDS = [
  "command", // Bash, PowerShell
  "file_path", // Write, Edit, MultiEdit, NotebookEdit
  "path", // Read, LS
  "url", // WebFetch
  "query", // WebSearch
  "pattern", // Glob, Grep
  "prompt", // Task
] as const;

function approvalTarget(tool: string, input: Record<string, unknown>): string {
  for (const field of APPROVAL_FIELDS) {
    const value = input[field];
    if (typeof value === "string" && value.trim()) {
      return `${tool} · ${value.trim()}`;
    }
  }
  return tool;
}

/**
 * `AskUserQuestion`'s input, checked against what Claude Code documents: 1–4
 * questions, 2–4 options each, every question and label a non-empty string.
 * Anything else returns null and the terminal asks instead — the answers are
 * keyed by question text, so two identical questions could not both be answered
 * either.
 */
export function parseQuestions(input: Record<string, unknown>): AskQuestion[] | null {
  const raw = input.questions;
  if (!Array.isArray(raw) || raw.length < 1 || raw.length > 4) return null;
  const out: AskQuestion[] = [];
  for (const q of raw) {
    if (!q || typeof q !== "object") return null;
    const { question, header, options, multiSelect } = q as Record<string, unknown>;
    if (typeof question !== "string" || !question.trim()) return null;
    if (!Array.isArray(options) || options.length < 2 || options.length > 4) return null;
    const opts = [];
    for (const o of options) {
      const label = (o as Record<string, unknown> | null)?.label;
      const description = (o as Record<string, unknown> | null)?.description;
      if (typeof label !== "string" || !label.trim()) return null;
      opts.push({ label, description: typeof description === "string" ? description : "" });
    }
    out.push({
      question,
      header: typeof header === "string" ? header : "",
      options: opts,
      multiSelect: multiSelect === true,
    });
  }
  if (new Set(out.map((q) => q.question)).size !== out.length) return null;
  return out;
}

/**
 * Claude Code moved on while a card was up, so the prompt was settled in the
 * terminal. It does not kill the relay when the terminal wins — the relay would
 * wait until the session ends — so this is how the card learns it is stale.
 *
 * Only events that cannot happen while the prompt is still waiting count: the
 * same tool finishing, the turn ending, a new prompt. `Notification` does not —
 * Claude Code sends one *because* the prompt is waiting.
 */
function settledElsewhere(island: Island, name: string, payload: HookPayload) {
  const card = State.pendingQuestion
    ? { id: State.pendingQuestion.requestId, session: State.pendingQuestion.sessionId, tool: "AskUserQuestion" }
    : State.pendingApproval
      ? { id: State.pendingApproval.requestId, session: State.pendingApproval.sessionId, tool: State.pendingApproval.tool }
      : null;
  if (!card || (card.session && payload.session_id && card.session !== payload.session_id)) return;
  const toolDone = (name === "PostToolUse" || name === "PostToolUseFailure") && payload.tool_name === card.tool;
  const turnMoved = name === "Stop" || name === "StopFailure" || name === "SessionEnd" || name === "UserPromptSubmit";
  if (!toolDone && !turnMoved) return;
  void Bridge.log(`${name} after req=${card.id} — answered elsewhere`);
  // Frees the relay too: it exits with nothing on stdout, which is ignored now.
  void Bridge.approvalDecline(card.id);
  island.releaseCard(card.id);
}

function upsert(projectName: string, cwd: string) {
  const t = State.tasks.find((x) => x.id === CLAUDE_ID);
  if (!t) return;
  t.name = projectName;
  if (cwd) t.sessionCwd = cwd;
}

function clearSession() {
  const t = State.tasks.find((x) => x.id === CLAUDE_ID);
  if (!t) return;
  t.steps = [];
  t.stepIndex = 0;
  t.name = "VS Code";
  t.pillBadge = null;
}

export function registerHookHandlers(island: Island) {
  void onEvent<HookPayload>("hook", (payload) => handleHook(island, payload));
  // The relay hung up while a card was up: the terminal took the answer (or the
  // session ended), so the card is asking about something that is gone.
  void onEvent<{ request_id: string }>("hook-cancelled", ({ request_id }) => {
    void Bridge.log(`hook-cancelled req=${request_id}`);
    island.releaseCard(request_id);
  });
}

function handleHook(island: Island, payload: HookPayload) {
  if (State.paused) {
    // Silence here used to cost Claude Code nearly two minutes: the relay waited
    // for a decision from an island that had already decided not to look. Say so,
    // and the terminal takes the question immediately.
    if (payload.request_id) void Bridge.approvalDecline(payload.request_id);
    return;
  }

  const name = payload.hook_event_name ?? "";
  settledElsewhere(island, name, payload);
  const cwd = payload.cwd ?? "";
  const raw = lastPathComponent(cwd);
  const projectName = aliasProjectName(raw || t("step.session"));
  const focused = State.focusId === CLAUDE_ID;

  /** Alerts force the island open; work events only reveal the compact island. */
  const surface = (view: Parameters<Island["alert"]>[0], isAlert: boolean) => {
    if (State.mode === "expanded") {
      if (isAlert) island.setView(view);
    } else if (isAlert) {
      island.alert(view);
    } else if (State.mode === "hidden") {
      island.reveal();
    }
  };

  switch (name) {
    case "SessionStart":
      upsert(projectName, cwd);
      surface("overview", false);
      Sound.play("work");
      break;

    case "UserPromptSubmit": {
      upsert(projectName, cwd);
      State.updateTask(CLAUDE_ID, "thinking");
      // The field is `prompt`; reading `message` meant this step was always blank.
      const asked = payload.prompt ?? payload.message;
      if (asked) State.appendStep(CLAUDE_ID, asked.slice(0, 60));
      surface("overview", false);
      break;
    }

    case "PreToolUse": {
      upsert(projectName, cwd);
      State.updateTask(CLAUDE_ID, "working");
      const tool = payload.tool_name ?? t("step.tool");
      State.appendStep(CLAUDE_ID, stepLabel(tool, payload.tool_input ?? {}));
      surface("overview", false);
      break;
    }

    case "PostToolUse":
      State.updateTask(CLAUDE_ID, "working");
      break;

    case "PostToolUseFailure":
      State.updateTask(CLAUDE_ID, "working");
      State.appendStep(CLAUDE_ID, t("step.failed"));
      break;

    case "Notification": {
      const message = payload.message ?? "";
      const lower = message.toLowerCase();
      if (lower.includes("rate limit") || lower.includes("limite d")) {
        State.updateTask(CLAUDE_ID, "ratelimit");
        Sound.play("rate");
      } else if (message.endsWith("?")) {
        State.updateTask(CLAUDE_ID, "question");
        State.appendStep(CLAUDE_ID, message);
      }
      break;
    }

    case "Stop":
      State.updateTask(CLAUDE_ID, "finished");
      if (payload.message) State.appendStep(CLAUDE_ID, payload.message.slice(0, 60));
      Sound.play("finish");
      if (focused) surface("finished", true);
      else State.setPillBadge(CLAUDE_ID, "finished");
      window.setTimeout(() => {
        State.updateTask(CLAUDE_ID, "idle");
        State.setPillBadge(CLAUDE_ID, null);
      }, 5200);
      break;

    case "StopFailure":
      State.updateTask(CLAUDE_ID, "error");
      Sound.play("error");
      if (focused) surface("error", true);
      else State.setPillBadge(CLAUDE_ID, "error");
      break;

    case "SessionEnd":
      State.updateTask(CLAUDE_ID, "idle");
      clearSession();
      break;

    case "SubagentStart":
      State.appendStep(CLAUDE_ID, t("step.subagent"));
      break;

    case "SubagentStop":
      State.appendStep(CLAUDE_ID, t("step.subagentDone"));
      break;

    case "PermissionRequest": {
      const requestId = payload.request_id ?? "";
      // One card, one request. A second one must never quietly replace the first
      // — that would leave a human staring at request B while request A waits for
      // a decision nobody can give. Hand it straight back to the terminal.
      const open = State.pendingApproval?.requestId ?? State.pendingQuestion?.requestId;
      if (open && open !== requestId) {
        if (requestId) void Bridge.approvalDecline(requestId);
        break;
      }
      upsert(projectName, cwd);
      const tool = payload.tool_name ?? t("step.tool");
      const input = payload.tool_input ?? {};
      const sessionId = payload.session_id ?? "";
      let view: "approval" | "question" = "approval";
      if (tool === "AskUserQuestion") {
        // Claude asking, not a permission: a yes/no card would be meaningless.
        const questions = parseQuestions(input);
        if (!questions) {
          // A shape we do not understand is answered in the terminal, never
          // guessed at. The read-only card still says a question is waiting.
          if (requestId) void Bridge.approvalDecline(requestId);
          State.updateTask(CLAUDE_ID, "question");
          State.appendStep(CLAUDE_ID, t("question.fallback"));
          if (focused) surface("question", true);
          break;
        }
        State.pendingQuestion = {
          requestId,
          sessionId,
          questions,
          index: 0,
          picked: questions.map(() => []),
          custom: questions.map(() => null),
          typing: false,
        };
        view = "question";
      } else {
        State.pendingApproval = { requestId, sessionId, tool, command: approvalTarget(tool, input) };
      }
      // The relay's short ack window closes in 800 ms; everything below this
      // line is synchronous, so the card really is up by the time it lands.
      if (requestId) void Bridge.approvalAck(requestId);
      State.updateTask(CLAUDE_ID, view === "question" ? "question" : "approval");
      State.isPinned = true;
      Sound.play(view === "question" ? "question" : "approval");
      if (focused) {
        island.alert(view);
      } else {
        // Another agent holds the view, so the card would yank it away. The badge
        // is the signal instead — but it has to be on screen for that to mean
        // anything, hence the reveal. We just told the relay a human can act.
        State.setPillBadge(CLAUDE_ID, "approval");
        island.reveal();
      }
      // No timer: the card stays until it is answered here, handed to the
      // terminal, or the relay hangs up (`hook-cancelled`).
      break;
    }

    default:
      break;
  }
  State.notify();
}
