// Island views — DOM ports of IslandViewContent.swift. Paddings, font sizes,
// colours and wording are copied from the Swift views so both platforms read
// identically.

import { h, svg, clear, dot } from "./dom";
import { ICONS } from "./icons";
import { Ticker } from "./ticker";
import { State, providerLabel, type AgentTask } from "../core/state";
import { washRGBA, type IslandViewName, type Wash } from "../core/layout";
import { createMiniBot, pruneMiniBots } from "../mochi/minibots";
import { buildPrompt } from "./chat";
import { buildChoose, buildUpload, buildUploading } from "./upload";
import { cardUiKey, renderIntegrationCard, type IntegrationCardHooks } from "./integrations";
import { t } from "../i18n";

export interface ViewActions {
  setView(v: IslandViewName): void;
  /** "Cancel" on a dropped file: forgets it and goes back home. */
  cancelDrop(): void;
  collapse(): void;
  setFocus(id: string): void;
  openTerminal(): void;
  /** The ↗ button: opens whatever the focused pill points at. */
  openTarget(): void;
  openUrl(url: string): void;
  decide(d: "allow" | "deny"): void;
  /** Leave the approval or question card to Claude Code's terminal prompt. */
  answerInTerminal(): void;
  /** Option `i` of the question on screen: answers it, or toggles it (multi). */
  questionPick(i: number): void;
  questionNext(): void;
  questionBack(): void;
  /** Opens or closes the "Other…" field. */
  questionOther(open: boolean): void;
  /** The user's own words for the question on screen. */
  questionCustom(text: string): void;
  toggleSound(): void;
  setVolume(v: number): void;
  setAutoClose(seconds: number): void;
  openSettingsWindow(): void;
  blip(): void;
}

export interface ViewHost {
  el: HTMLElement;
  sync(): void;
  /** Called when the view becomes active, for views with a text field. */
  focus?(): void;
  /** Called every frame while the view is on screen. */
  tick?(nowMs: number): void;
}

// ── Shared pieces ─────────────────────────────────────────────────────────────

function card(wash: Wash, ...children: (Node | string)[]): HTMLElement {
  const el = h("div", { class: wash ? "card wash" : "card" }, ...children);
  if (wash) el.style.setProperty("--wash", washRGBA(wash));
  return el;
}

function btn(
  label: string,
  kind: "primary" | "secondary",
  onClick: () => void,
  kbd?: string,
): HTMLElement {
  return h(
    "button",
    { class: `btn ${kind}`, onclick: onClick },
    h("span", { text: label }),
    kbd ? h("span", { class: "kbd", text: kbd }) : null,
  );
}

/** AgentWho — coloured dot + task name + grey label. */
function agentWho(task: AgentTask | null, label: string): HTMLElement {
  const row = h("div", { class: "who-row" });
  if (task) {
    row.append(dot(task.color, 8), h("span", { class: "n", text: task.name }));
  }
  row.append(h("span", { text: label }));
  return row;
}

function stack(padLeft: number, padRight: number, ...children: Node[]): HTMLElement {
  const el = h("div", { class: "stack" }, ...children);
  el.style.padding = `4px ${padRight}px 4px ${padLeft}px`;
  return el;
}

// ── Header ────────────────────────────────────────────────────────────────────

export function buildHeader(actions: ViewActions): ViewHost {
  const tabHome = h("button", { class: "tab", title: t("tab.overview"), onclick: () => go("overview") }, svg(ICONS.house, 13));
  const tabChat = h("button", { class: "tab", title: t("tab.ask"), onclick: () => go("prompt") }, svg(ICONS.bubble, 13));
  const tabDrop = h("button", { class: "tab", title: t("tab.drop"), onclick: () => go("upload") }, svg(ICONS.plus, 13));

  const gearBtn = h("button", { title: t("tab.settings"), onclick: () => go("settings") }, svg(ICONS.gear, 14));
  const soundBtn = h("button", { title: t("tab.mute"), onclick: () => actions.toggleSound() }, svg(ICONS.speakerOn, 14));

  function go(v: IslandViewName) {
    actions.blip();
    actions.setView(v);
  }

  const el = h(
    "div",
    { id: "header" },
    h("div", { class: "tabs" }, tabHome, tabChat, tabDrop),
    h("div", { class: "header-actions" }, gearBtn, soundBtn),
  );

  return {
    el,
    sync() {
      const v = State.view;
      tabHome.classList.toggle("on", v === "overview" || v === "empty");
      tabChat.classList.toggle("on", v === "prompt");
      tabDrop.classList.toggle("on", v === "upload");
      gearBtn.classList.toggle("on", v === "settings");
      clear(gearBtn);
      gearBtn.append(svg(v === "settings" ? ICONS.gearFill : ICONS.gear, 14));
      clear(soundBtn);
      soundBtn.append(svg(State.settings.soundEnabled ? ICONS.speakerOn : ICONS.speakerOff, 14));
      el.style.opacity = v === "confused" ? "0" : "1";
    },
  };
}

// ── Overview ──────────────────────────────────────────────────────────────────

function buildOverview(actions: ViewActions): ViewHost {
  const ticker = new Ticker();
  const who = h("div", { class: "who" });
  const tickerBody = h("div", { class: "card-body" }, who, ticker.el);
  const leftBody = h("div", { class: "left-body" });
  const jump = h(
    "button",
    { class: "icon-btn jump", title: t("overview.open"), onclick: () => actions.openTarget() },
    svg(ICONS.arrowUpRight, 8),
  );
  const left = card(null, leftBody, jump);
  const pills = h("div", { class: "pills" });
  const right = card(null, pills);

  const el = h("div", { class: "view overview" },
    h("div", { class: "left" }, left),
    h("div", { class: "right" }, right),
  );

  let pillIds = "";
  let detailOpen = false;
  let lastFocus: string | null = null;
  let mode: "ticker" | "card" | null = null;
  let cardKey = "";

  const hooks: IntegrationCardHooks = {
    get detailOpen() {
      return detailOpen;
    },
    openDetail() {
      detailOpen = true;
      cardKey = "";
      State.notify();
    },
    closeDetail() {
      detailOpen = false;
      cardKey = "";
      State.notify();
    },
    openSettings: () => actions.openSettingsWindow(),
  };

  return {
    el,
    tick(nowMs: number) {
      if (mode === "ticker") ticker.tick(nowMs);
    },
    sync() {
      const task = State.focusTask;
      if (task?.id !== lastFocus) {
        lastFocus = task?.id ?? null;
        detailOpen = false;
        cardKey = "";
        mode = null;
      }

      // VS Code with a live Claude Code session keeps the ticker; every other
      // pill shows its own card, exactly like IntegrationCardView.
      const sessionActive =
        task?.id === "integration_claude" && (task.state !== "idle" || task.steps.length > 0);

      if (task && sessionActive) {
        if (mode !== "ticker") {
          clear(leftBody);
          leftBody.append(tickerBody);
          mode = "ticker";
          cardKey = "";
        }
        clear(who);
        who.append(
          dot(task.color, 7),
          h("span", { class: "name", text: task.name }),
          h("span", { class: "tool", text: task.source === "claudeCode" ? "Claude Code" : "n8n" }),
        );
        if (task.steps.length > 1) {
          who.append(h("span", {
            class: "count",
            text: `${Math.min(task.stepIndex + 1, task.steps.length)}/${task.steps.length}`,
          }));
        }
        ticker.sync(task);
      } else if (task) {
        const info = State.integrations[task.id];
        const key = [
          task.id, detailOpen, task.state, task.steps.join("|"),
          info?.loaded, info?.error, info?.configured,
          JSON.stringify(info?.data ?? {}), cardUiKey(task.id),
        ].join("~");
        if (key !== cardKey) {
          cardKey = key;
          mode = "card";
          clear(leftBody);
          leftBody.append(renderIntegrationCard(task, hooks));
        }
      }

      jump.style.display = detailOpen ? "none" : "";

      const others = State.otherTasks.slice(0, 4);
      const pillKey = others.map((t) => `${t.id}:${t.pillBadge ?? ""}`).join("|");
      if (pillKey !== pillIds) {
        pillIds = pillKey;
        clear(pills);
        for (const t of others) pills.append(buildPill(t, actions));
        pruneMiniBots();
      }
    },
  };
}

function buildPill(task: AgentTask, actions: ViewActions): HTMLElement {
  const label = task.id === "integration_claude" ? "VS Code" : task.name;
  const canvas = createMiniBot(task, 24);
  const pill = h(
    "div",
    { class: "pill", onclick: () => actions.setFocus(task.id) },
    canvas,
    h("span", { class: "lbl", text: label }),
  );
  pill.style.borderColor = `${task.color}24`;
  pill.addEventListener("mouseenter", () => {
    pill.style.background = `${task.color}2e`;
    pill.style.borderColor = `${task.color}8c`;
    pill.style.boxShadow = `0 2px 10px ${task.color}59`;
    (pill.querySelector(".lbl") as HTMLElement).style.color = lighten(task.color, 0.3);
  });
  pill.addEventListener("mouseleave", () => {
    pill.style.background = "";
    pill.style.borderColor = `${task.color}24`;
    pill.style.boxShadow = "";
    (pill.querySelector(".lbl") as HTMLElement).style.color = "";
  });

  if (task.pillBadge) {
    const colors = { approval: "#F5A524", finished: "#22C55E", error: "#F4505E" } as const;
    const icons = { approval: ICONS.bang, finished: ICONS.check, error: ICONS.xmark } as const;
    const inner = h("i", { style: `background:${colors[task.pillBadge]}` }, svg(icons[task.pillBadge], 6, { stroke: task.pillBadge === "finished" ? 3 : 0 }));
    const badge = h("div", { class: "pill-badge" }, inner);
    badge.style.boxShadow = `0 0 4px ${colors[task.pillBadge]}99`;
    pill.append(badge);
  }
  return pill;
}

function lighten(hex: string, amount: number): string {
  const v = parseInt(hex.replace("#", ""), 16);
  const c = [(v >> 16) & 255, (v >> 8) & 255, v & 255].map((x) =>
    Math.min(255, Math.round(x + amount * 255)),
  );
  return `rgb(${c[0]},${c[1]},${c[2]})`;
}

// ── Empty ─────────────────────────────────────────────────────────────────────

function buildEmpty(actions: ViewActions): ViewHost {
  const ask = btn("", "primary", () => actions.setView("prompt"));
  const askLabel = ask.firstElementChild as HTMLElement;
  const body = h(
    "div",
    { class: "stack", style: "padding:0 18px 0 118px;flex-direction:row;align-items:center;gap:16px" },
    h(
      "div",
      { style: "display:flex;flex-direction:column;gap:5px" },
      h("div", { class: "title", text: t("empty.title") }),
      h("div", { class: "sub", text: t("empty.sub") }),
    ),
    h("div", { class: "grow" }),
    ask,
  );
  return {
    el: h("div", { class: "view" }, card(null, body)),
    sync() {
      // The provider can change from the settings window while the island runs.
      const label = t("empty.ask", { provider: providerLabel(State.settings.chatProvider) });
      if (askLabel.textContent !== label) askLabel.textContent = label;
    },
  };
}

// ── Approval ──────────────────────────────────────────────────────────────────

function buildApproval(actions: ViewActions): ViewHost {
  const who = h("div");
  const code = h("div", { class: "code" });
  const row = h("div", { class: "actions" });
  const el = h("div", { class: "view" }, card("amber", stack(116, 16, who, code, row)));
  let rowKey = "";
  return {
    el,
    sync() {
      clear(who);
      who.append(agentWho(State.focusTask, t("approval.who")));
      // The whole point of approving here rather than in the terminal: this line
      // is the command, the file path or the URL being authorised, not just the
      // name of the tool asking.
      code.textContent = State.pendingApproval?.command || State.pendingApproval?.tool || "…";
      // Built once. Rebuilding them between a mouse-down and a mouse-up would
      // swallow the click, and there is nothing left to vary: "Always" is gone
      // until the remembered-rules list exists to back it.
      if (rowKey === "built") return;
      rowKey = "built";
      clear(row);
      row.append(
        btn(t("approval.deny"), "secondary", () => actions.decide("deny"), "N"),
        btn(t("approval.allow"), "primary", () => actions.decide("allow"), "Y"),
        h("div", { class: "spacer" }),
        btn(t("approval.terminal"), "secondary", () => actions.answerInTerminal(), "Esc"),
      );
    },
  };
}

// ── Question ──────────────────────────────────────────────────────────────────

/**
 * Two cards in one. With an `AskUserQuestion` pending it is answered right here:
 * one question at a time, a click per choice (several for a multi-select), or
 * the user's own words. Without one — a Notification ending in "?" — it only
 * says that the terminal is waiting.
 */
function buildQuestion(actions: ViewActions): ViewHost {
  const who = h("div");
  const meta = h("div", { class: "q-meta" });
  const title = h("div", { class: "title q-title" });
  const grid = h("div", { class: "q-grid" });
  const input = h("input", {
    class: "q-input",
    type: "text",
    placeholder: t("question.otherPlaceholder"),
  }) as HTMLInputElement;
  input.addEventListener("keydown", (e) => {
    e.stopPropagation();
    if (e.key === "Enter") actions.questionCustom(input.value);
    if (e.key === "Escape") actions.questionOther(false);
  });
  const field = h("div", { class: "q-field" }, input);
  const row = h("div", { class: "actions" });
  const el = h(
    "div",
    { class: "view" },
    card("cyan", stack(116, 16, who, meta, title, grid, field, row)),
  );

  // Buttons are rebuilt only when another question comes up, never on a plain
  // sync: rebuilding between a mouse-down and a mouse-up would swallow the click.
  let builtKey = "";
  let options: HTMLElement[] = [];
  let next: HTMLButtonElement | null = null;

  function rebuild(key: string) {
    builtKey = key;
    clear(grid);
    clear(row);
    options = [];
    next = null;
    const q = State.pendingQuestion;
    const current = q?.questions[q.index];
    if (!q || !current) {
      row.append(h("div", { class: "sub", text: t("question.sub") }));
      return;
    }
    current.options.forEach((o, i) => {
      const b = h(
        "button",
        { class: "q-opt", title: o.description || o.label, onclick: () => actions.questionPick(i) },
        h("span", { class: "q-n", text: String(i + 1) }),
        h("span", { class: "q-text" },
          h("span", { class: "q-l", text: o.label }),
          o.description ? h("span", { class: "q-d", text: o.description }) : null,
        ),
      );
      options.push(b);
      grid.append(b);
    });
    if (q.index > 0) row.append(btn(t("question.back"), "secondary", () => actions.questionBack()));
    row.append(btn(t("question.other"), "secondary", () => actions.questionOther(true)));
    row.append(h("div", { class: "spacer" }));
    row.append(btn(t("question.terminal"), "secondary", () => actions.answerInTerminal(), "Esc"));
    // A single choice is answered by its click; only a multi-select needs a
    // button to say "that's all".
    if (current.multiSelect) {
      const last = q.index === q.questions.length - 1;
      next = btn(last ? t("question.send") : t("question.next"), "primary", () => actions.questionNext(), "↵") as HTMLButtonElement;
      row.append(next);
    }
  }

  return {
    el,
    sync() {
      const q = State.pendingQuestion;
      const current = q?.questions[q.index];
      clear(who);
      who.append(agentWho(State.focusTask, t("question.who")));

      const key = q && current ? `${q.requestId}:${q.index}` : "readonly";
      if (key !== builtKey) rebuild(key);

      if (!q || !current) {
        meta.style.display = "none";
        grid.style.display = "none";
        field.style.display = "none";
        title.textContent = State.focusTask?.steps.at(-1) ?? t("question.fallback");
        return;
      }

      const bits = [current.header, q.questions.length > 1
        ? t("question.progress", { n: String(q.index + 1), total: String(q.questions.length) })
        : "", current.multiSelect ? t("question.multiHint") : ""].filter(Boolean);
      meta.textContent = bits.join(" · ");
      meta.style.display = bits.length ? "" : "none";
      title.textContent = current.question;
      title.title = current.question;

      grid.style.display = q.typing ? "none" : "";
      field.style.display = q.typing ? "" : "none";
      if (!q.typing) input.value = q.custom[q.index] ?? "";

      const picked = q.picked[q.index];
      options.forEach((b, i) => b.classList.toggle("on", picked.includes(i)));
      if (next) next.disabled = picked.length === 0 && q.custom[q.index] == null;
    },
    focus() {
      input.focus();
      input.select();
    },
  };
}

// ── Error ─────────────────────────────────────────────────────────────────────

function buildError(actions: ViewActions): ViewHost {
  const who = h("div");
  const title = h("div", { class: "title", text: t("error.workflowStopped") });
  const detail = h("div", { class: "detail" });
  const row = h("div", { class: "actions" },
    btn(t("error.retry"), "primary", () => actions.setView(State.defaultView())),
    btn(t("error.openN8n"), "secondary", () => actions.openUrl("")),
  );
  const el = h("div", { class: "view" }, card("red", stack(116, 16, who, title, detail, row)));
  return {
    el,
    sync() {
      const task = State.focusTask;
      clear(who);
      who.append(agentWho(task, task?.source === "n8n" ? "n8n" : "Claude Code"));
      title.textContent = task?.source === "n8n" ? t("error.workflowStopped") : t("error.sessionStopped");
      detail.textContent = task?.steps.at(-1) ?? t("error.noDetail");
    },
  };
}

// ── Finished ──────────────────────────────────────────────────────────────────

function buildFinished(actions: ViewActions): ViewHost {
  const who = h("div");
  const title = h("div", { class: "title" });
  const row = h("div", { class: "actions" },
    btn(t("finished.openTerminal"), "primary", () => actions.openTerminal()),
    btn(t("finished.ok"), "secondary", () => actions.collapse()),
  );
  const el = h("div", { class: "view" }, card("green", stack(116, 16, who, title, row)));
  return {
    el,
    sync() {
      clear(who);
      who.append(agentWho(State.focusTask, t("finished.who")));
      title.textContent = State.focusTask?.steps.at(-1) ?? t("finished.fallback");
    },
  };
}

// ── Confused ──────────────────────────────────────────────────────────────────

function buildConfused(): ViewHost {
  const body = h(
    "div",
    { class: "stack", style: "padding:0 18px 0 128px" },
    h("div", { class: "title", text: t("confused.title") }),
    h("div", { class: "sub", text: t("confused.sub") }),
  );
  return { el: h("div", { class: "view" }, card("pink", body)), sync() {} };
}

// ── Note ──────────────────────────────────────────────────────────────────────

function buildNote(): ViewHost {
  const title = h("div", { class: "title" });
  const el = h("div", { class: "view" }, card(null, h("div", { class: "stack", style: "padding:0 18px 0 98px" }, title)));
  return {
    el,
    sync() {
      title.textContent = State.noteMessage ?? "";
    },
  };
}

// ── In-island settings ────────────────────────────────────────────────────────

function buildSettings(actions: ViewActions): ViewHost {
  const soundSwitch = h("button", { class: "switch", onclick: () => actions.toggleSound() });
  const volume = h("input", {
    type: "range", min: "0", max: "0.2", step: "0.005",
    oninput: (e: Event) => actions.setVolume(Number((e.target as HTMLInputElement).value)),
  }) as HTMLInputElement;
  const autoLabel = h("span", {});
  const segButtons = [10, 15, 30].map((s) =>
    h("button", { onclick: () => actions.setAutoClose(s) }, `${s}s`),
  );
  const claudeBadge = h("span", { class: "status-badge" });
  const apiBadge = h("span", { class: "status-badge" });

  const rows = h(
    "div",
    { class: "settings-rows" },
    h("div", { class: "settings-row" }, soundSwitch, h("span", { text: t("islandSettings.sound") }), volume),
    h(
      "div",
      { class: "settings-row" },
      svg(ICONS.timer, 12),
      autoLabel,
      h("div", { class: "seg" }, ...segButtons),
    ),
    h(
      "div",
      { class: "settings-row", style: "gap:14px" },
      claudeBadge,
      apiBadge,
      h("div", { class: "grow" }),
      h("button", {
        class: "link-btn",
        style: "color:#8e939c;font-size:11.5px",
        text: t("islandSettings.more"),
        onclick: () => actions.openSettingsWindow(),
      }),
    ),
  );

  const el = h("div", { class: "view" },
    card(null, h("div", { class: "stack", style: "padding:14px 16px 14px 84px" }, rows)));

  return {
    el,
    sync() {
      const s = State.settings;
      soundSwitch.classList.toggle("on", s.soundEnabled);
      volume.value = String(s.soundVolume);
      volume.style.opacity = s.soundEnabled ? "1" : "0.4";
      autoLabel.textContent = t("islandSettings.autoClose", { s: Math.round(s.autoCloseInterval) });
      segButtons.forEach((b, i) => b.classList.toggle("on", s.autoCloseInterval === [10, 15, 30][i]));
      clear(claudeBadge);
      claudeBadge.append(
        dot(s.hooksInstalled ? "#22C55E" : "#F4505E", 6),
        h("span", { text: "Claude Code" }),
      );
      clear(apiBadge);
      apiBadge.append(dot("#F4505E", 6), h("span", { text: "API" }));
    },
  };
}

// ── Placeholders filled in later stages ───────────────────────────────────────

function buildPlaceholder(title: string | (() => string), sub: string): ViewHost {
  const titleEl = h("div", { class: "title", text: typeof title === "string" ? title : title() });
  const body = h(
    "div",
    { class: "stack", style: "padding:0 18px 0 118px" },
    titleEl,
    h("div", { class: "sub", text: sub }),
  );
  return {
    el: h("div", { class: "view" }, card(null, body)),
    sync() {
      if (typeof title === "string") return;
      const text = title();
      if (titleEl.textContent !== text) titleEl.textContent = text;
    },
  };
}

// ── Registry ──────────────────────────────────────────────────────────────────

export function buildViews(
  actions: ViewActions,
  onChatHeightChange: () => void,
): Map<IslandViewName, ViewHost> {
  const map = new Map<IslandViewName, ViewHost>();
  map.set("overview", buildOverview(actions));
  map.set("empty", buildEmpty(actions));
  map.set("approval", buildApproval(actions));
  map.set("question", buildQuestion(actions));
  map.set("error", buildError(actions));
  map.set("finished", buildFinished(actions));
  map.set("confused", buildConfused());
  map.set("note", buildNote());
  map.set("settings", buildSettings(actions));
  map.set("prompt", buildPrompt(onChatHeightChange));
  map.set("upload", buildUpload());
  map.set("uploading", buildUploading());
  map.set("choose", buildChoose(actions));
  // Not in the Windows v1: sending a file by email, window attach + web result.
  map.set("mail", buildPlaceholder(t("placeholder.mail"), ""));
  map.set("searching", buildPlaceholder(
    () => t("placeholder.searching", { provider: providerLabel(State.settings.chatProvider) }), "",
  ));
  map.set("result", buildPlaceholder(t("placeholder.result"), ""));
  return map;
}
