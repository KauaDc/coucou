// Integration cards shown in the overview's left card — DOM ports of
// IntegrationCardView and friends from IslandViewContent.swift.
//
// Cal.com is the one simplification: macOS shows a three-level calendar
// (month → day → booking); here it is the list of upcoming bookings.

import { h, svg, clear, dot } from "./dom";
import { ICONS } from "./icons";
import { State, type AgentTask } from "../core/state";
import { Bridge } from "../core/bridge";
import { LANG, t, type Key } from "../i18n";

/** Same shape as the Swift `timeAgo` computed properties. */
export function timeAgo(value: unknown): string {
  const date = typeof value === "number" ? new Date(value) : new Date(String(value));
  const diff = (Date.now() - date.getTime()) / 1000;
  if (!Number.isFinite(diff)) return "";
  if (diff < 60) return t("int.justNow");
  if (diff < 3600) return `${Math.floor(diff / 60)}m`;
  if (diff < 86400) return `${Math.floor(diff / 3600)}h`;
  return `${Math.floor(diff / 86400)}d`;
}

function header(color: string, name: string, kind: string, extra?: Node): HTMLElement {
  const row = h("div", { class: "int-head" }, dot(color, 7), h("b", { text: name }), h("span", { text: kind }));
  if (extra) row.append(extra);
  return row;
}

/** Highlighted first row + plain rows, the layout every list card shares. */
function listRow(accent: string, first: boolean, ...children: Node[]): HTMLElement {
  const row = h("div", { class: first ? "int-row first" : "int-row" }, dot(accent, 5), ...children);
  if (first) row.style.background = `${accent}14`;
  return row;
}

function get(id: string): Record<string, unknown> {
  return (State.integrations[id]?.data ?? {}) as Record<string, unknown>;
}

function arr(id: string, key: string): Record<string, unknown>[] {
  const v = get(id)[key];
  return Array.isArray(v) ? (v as Record<string, unknown>[]) : [];
}

// ── Not configured / idle ─────────────────────────────────────────────────────

const OPEN_URLS: Record<string, string> = {
  integration_resend: "https://resend.com/emails",
  integration_vercel: "https://vercel.com/dashboard",
  integration_github: "https://github.com",
  integration_stripe: "https://dashboard.stripe.com/payments",
  integration_notion: "https://notion.so",
  integration_calcom: "https://app.cal.com/bookings",
  integration_discloud: "https://discloud.com/dashboard",
};

function idleCard(task: AgentTask, openSettings: () => void): HTMLElement {
  const info = State.integrations[task.id];
  const configured = info?.configured ?? false;
  const error = info?.error ?? null;
  // The Claude Code pill is about hooks, not a key — the macOS wording would be
  // misleading here.
  const missing = task.id === "integration_claude" ? t("int.hooksMissing") : t("int.keyMissing");
  const label = error ?? (configured ? t("int.loading") : missing);
  const statusColor = error || !configured ? "#F4505E" : "#22C55E";

  const actions = h("div", { class: "int-actions" });
  if (task.id === "integration_claude") {
    actions.append(
      h("button", {
        class: "link-btn",
        style: `color:${task.color}b3`,
        text: t("int.openVSCode"),
        onclick: () => void Bridge.openInVSCode(task.sessionCwd ?? null),
      }),
    );
  } else if (task.id === "integration_n8n") {
    actions.append(
      h("button", {
        class: "link-btn",
        style: `color:${task.color}d9`,
        text: t("int.openN8n"),
        onclick: () => void Bridge.openN8n(),
      }),
    );
  } else if (OPEN_URLS[task.id]) {
    actions.append(
      h("button", {
        class: "link-btn",
        style: `color:${task.color}d9`,
        text: t("int.open", { name: task.name }),
        onclick: () => void Bridge.openUrl(OPEN_URLS[task.id]),
      }),
    );
  }
  if (configured) {
    actions.append(
      h("button", {
        class: "link-btn",
        style: `color:${task.color}d9`,
        text: t("int.refresh"),
        onclick: () => void Bridge.refreshIntegration(task.id),
      }),
    );
  } else {
    actions.append(
      h("button", { class: "link-btn", style: "color:#8e939c", text: t("int.settings"), onclick: openSettings }),
    );
  }

  return h(
    "div",
    { class: "int-card" },
    header(task.color, task.id === "integration_claude" ? "VS Code" : task.name, t("int.kind.integration")),
    h("div", { class: "int-status" }, dot(statusColor, 5), h("span", { text: label })),
    actions,
  );
}

// ── Vercel ────────────────────────────────────────────────────────────────────

function vercelCard(onDetail: () => void): HTMLElement {
  const deployments = arr("integration_vercel", "deployments");
  const rows = h("div", { class: "int-rows" });
  deployments.slice(0, 3).forEach((d, i) => {
    const accent = d.state === "READY" ? "#22C55E" : "#F4505E";
    const name = h("span", { class: "int-name", text: String(d.projectName ?? "") });
    const ago = h("span", { class: "int-ago", text: timeAgo(d.createdAt) });
    if (i === 0) {
      const more = h(
        "button",
        { class: "int-more", title: t("int.details"), onclick: onDetail },
        svg(ICONS.ellipsis, 8),
      );
      rows.append(listRow(accent, true, name, ago, more));
    } else {
      rows.append(listRow(accent, false, name, ago));
    }
  });
  return h("div", { class: "int-card" }, header("#7C5CFF", "Vercel", t("int.kind.deployments")), rows);
}

function vercelDetail(onBack: () => void): HTMLElement {
  const d = arr("integration_vercel", "deployments")[0] ?? {};
  const success = d.state === "READY";
  const accent = success ? "#22C55E" : "#F4505E";
  const status = success ? t("int.ready") : d.state === "CANCELED" ? t("int.canceled") : t("int.error");
  const body = h("div", { class: "int-detail-body" });
  if (d.commitMessage) body.append(h("div", { class: "int-commit", text: String(d.commitMessage) }));
  const meta = h("div", { class: "int-meta" });
  if (d.branch) meta.append(h("span", { text: String(d.branch) }));
  const ago = timeAgo(d.createdAt);
  meta.append(h("span", { text: ago === t("int.justNow") ? ago : t("int.ago", { t: ago }) }));
  body.append(meta);
  if (d.url) {
    body.append(
      h("button", {
        class: "int-link",
        text: String(d.url),
        onclick: () => void Bridge.openUrl(`https://${d.url}`),
      }),
    );
  }
  return h(
    "div",
    { class: "int-card detail" },
    h(
      "div",
      { class: "int-detail-head" },
      h("button", { class: "int-back", onclick: onBack }, svg(ICONS.chevronLeft, 10, { stroke: 2.4 })),
      dot(accent, 6),
      h("b", { text: String(d.projectName ?? t("int.deployment")) }),
      h("span", { class: "int-badge", style: `color:${accent};background:${accent}24`, text: status }),
    ),
    body,
  );
}

// ── Resend ────────────────────────────────────────────────────────────────────

function resendCard(): HTMLElement {
  const emails = arr("integration_resend", "emails");
  const total = get("integration_resend").total;
  const extra =
    total != null
      ? h("span", { class: "int-total" }, h("i", { class: "pulse" }), h("span", { text: String(total) }))
      : undefined;
  const rows = h("div", { class: "int-rows" });
  emails.slice(0, 3).forEach((e, i) => {
    const delivered = e.lastEvent === "delivered";
    const accent = delivered ? "#22C55E" : "#F4505E";
    const to = Array.isArray(e.to) ? String(e.to[0] ?? "?") : "?";
    const short = to.split("@")[0];
    const cells: Node[] = [
      h("span", { class: "int-name", text: short }),
      h("span", { class: "int-ago", text: timeAgo(e.createdAt) }),
    ];
    if (i === 0 && e.subject) cells.push(h("span", { class: "int-sub", text: String(e.subject) }));
    rows.append(listRow(accent, i === 0, ...cells));
  });
  return h("div", { class: "int-card" }, header("#22C55E", "Resend", t("int.kind.emails"), extra), rows);
}

// ── GitHub ────────────────────────────────────────────────────────────────────

function statRow(icon: string, color: string, label: string, value: string): HTMLElement {
  return h(
    "div",
    { class: "int-stat" },
    h("i", { class: "int-stat-icon", style: `color:${color}` }, svg(icon, 10)),
    h("span", { class: "int-stat-label", text: label }),
    h("span", { class: "int-stat-value", text: value }),
  );
}

function githubCard(): HTMLElement {
  const d = get("integration_github");
  const stars = Number(d.totalStars ?? 0);
  const repos = Number(d.totalRepos ?? 0);
  const fmt = (n: number) => (n >= 1000 ? `${(n / 1000).toFixed(1)}k` : String(n));
  return h(
    "div",
    { class: "int-card" },
    header("#F4505E", "GitHub", t("int.kind.overview")),
    h(
      "div",
      { class: "int-stats" },
      statRow(ICONS.star, "#F5A524", t("int.totalStars"), fmt(stars)),
      statRow(ICONS.stack, "#6B7079", t("int.repositories"), String(repos)),
    ),
  );
}

// ── Stripe ────────────────────────────────────────────────────────────────────

function stripeCard(): HTMLElement {
  const d = get("integration_stripe");
  const balance = (Number(d.balance ?? 0) / 100).toFixed(2);
  const currency = String(d.currency ?? "eur").toUpperCase();
  const rows = h("div", { class: "int-rows tight" });
  for (const p of arr("integration_stripe", "payments")) {
    const success = p.status === "succeeded";
    const accent = success ? "#22C55E" : "#F4505E";
    rows.append(
      h(
        "div",
        { class: "int-row" },
        dot(accent, 5),
        h("span", { class: "int-name", text: String(p.description ?? t("int.payment")) }),
        h("span", {
          class: "int-amount",
          style: "color:#22c55e",
          text: `+${(Number(p.amount ?? 0) / 100).toFixed(2)}`,
        }),
        h("span", { class: "int-ago", text: timeAgo(p.createdAt) }),
      ),
    );
  }
  return h(
    "div",
    { class: "int-card" },
    header("#0570DE", "Stripe", t("int.kind.payments")),
    h("div", { class: "int-balance" }, h("span", { text: balance }), h("i", { text: currency })),
    rows,
  );
}

// ── Notion ────────────────────────────────────────────────────────────────────

function notionCard(): HTMLElement {
  const rows = h("div", { class: "int-rows tight" });
  for (const p of arr("integration_notion", "pages").slice(0, 3)) {
    rows.append(
      h(
        "button",
        {
          class: "int-page",
          onclick: () => {
            if (typeof p.url === "string") void Bridge.openUrl(p.url);
          },
        },
        p.emoji
          ? h("span", { class: "int-emoji", text: String(p.emoji) })
          : h("i", { class: "int-emoji" }, svg(ICONS.doc, 9)),
        h("span", { class: "int-name", text: String(p.title ?? t("int.untitled")) }),
        h("span", { class: "int-ago", text: timeAgo(p.lastEditedAt) }),
      ),
    );
  }
  return h("div", { class: "int-card" }, header("#E8E8E8", "Notion", t("int.kind.recent")), rows);
}

// ── Cal.com ───────────────────────────────────────────────────────────────────

function calcomCard(): HTMLElement {
  const bookings = arr("integration_calcom", "bookings")
    .slice()
    .sort((a, b) => new Date(String(a.start)).getTime() - new Date(String(b.start)).getTime());
  const rows = h("div", { class: "int-rows tight" });
  if (bookings.length === 0) {
    rows.append(h("div", { class: "int-empty", text: t("int.noCalls") }));
  }
  for (const b of bookings.slice(0, 3)) {
    const when = new Date(String(b.start));
    const day = when.toLocaleDateString(LANG, { day: "2-digit", month: "2-digit" });
    const time = when.toLocaleTimeString(LANG, { hour: "2-digit", minute: "2-digit" });
    rows.append(
      h(
        "div",
        { class: "int-row" },
        dot("#C9956A", 4),
        h("span", { class: "int-time", text: `${day} ${time}` }),
        h("span", { class: "int-name", text: String(b.title ?? t("int.meeting")) }),
      ),
    );
  }
  return h("div", { class: "int-card" }, header("#C9956A", "Cal.com", t("int.kind.schedule")), rows);
}

// ── Discloud ──────────────────────────────────────────────────────────────────
// Windows only, and the one card that writes: start / stop / restart. Stop and
// restart need a second click within 4 s.

const DISCLOUD = "integration_discloud";
const DISCLOUD_COLOR = "#14B8A6";
const CONFIRM_MS = 4000;
const NOTICE_MS = 6000;

type DiscloudAction = "start" | "stop" | "restart";

const ACTION_LABEL: Record<DiscloudAction, Key> = {
  start: "int.start",
  stop: "int.stop",
  restart: "int.restart",
};
const CONFIRM_LABEL: Record<DiscloudAction, Key> = {
  start: "int.start",
  stop: "int.confirmStop",
  restart: "int.confirmRestart",
};
const PENDING_LABEL: Record<DiscloudAction, Key> = {
  start: "int.starting",
  stop: "int.stopping",
  restart: "int.restarting",
};

interface DiscloudLogsEntry {
  text: string;
  url: string | null;
  error: string | null;
}

/** UI state that isn't in State. Bumping `version` makes the overview redraw the card. */
const discloud = {
  version: 0,
  selected: null as string | null,
  /** The detail shows the selected app's logs instead of its actions. */
  showLogs: false,
  armed: null as { id: string; action: DiscloudAction; timer: number } | null,
  pending: new Map<string, DiscloudAction>(),
  notice: null as { id: string; text: string; ok: boolean; timer: number } | null,
  logs: new Map<string, DiscloudLogsEntry>(),
  logsLoading: new Set<string>(),
  /** Where the user left the log scrolled; "end" follows new lines. */
  logScroll: new Map<string, number | "end">(),
};

function bumpDiscloud() {
  discloud.version++;
  State.notify();
}

/** Extra redraw key for cards whose state lives here rather than in State. */
export function cardUiKey(id: string): string {
  return id === DISCLOUD ? String(discloud.version) : "";
}

function loadDiscloudLogs(id: string) {
  if (discloud.logsLoading.has(id)) return;
  discloud.logsLoading.add(id);
  bumpDiscloud();
  Bridge.discloudLogs(id)
    .then((r) => discloud.logs.set(id, { text: r.text, url: r.url, error: null }))
    .catch((err) => {
      const previous = discloud.logs.get(id);
      discloud.logs.set(id, { text: previous?.text ?? "", url: previous?.url ?? null, error: String(err) });
    })
    .finally(() => {
      discloud.logsLoading.delete(id);
      bumpDiscloud();
    });
}

function showNotice(id: string, text: string, ok: boolean) {
  if (discloud.notice) window.clearTimeout(discloud.notice.timer);
  const timer = window.setTimeout(() => {
    discloud.notice = null;
    bumpDiscloud();
  }, NOTICE_MS);
  discloud.notice = { id, text, ok, timer };
}

function onDiscloudAction(id: string, action: DiscloudAction) {
  if (discloud.pending.has(id)) return;
  const armed = discloud.armed;
  const confirmed = armed?.id === id && armed.action === action;
  if (armed) window.clearTimeout(armed.timer);
  discloud.armed = null;

  if (action !== "start" && !confirmed) {
    const timer = window.setTimeout(() => {
      discloud.armed = null;
      bumpDiscloud();
    }, CONFIRM_MS);
    discloud.armed = { id, action, timer };
    bumpDiscloud();
    return;
  }

  discloud.pending.set(id, action);
  bumpDiscloud();
  void Bridge.discloudAction(id, action).then((result) => {
    discloud.pending.delete(id);
    showNotice(id, result?.message ?? t("int.actionFailed"), result?.ok ?? false);
    bumpDiscloud();
    // Fresh lines once the container has moved.
    if (result?.ok) window.setTimeout(() => loadDiscloudLogs(id), 8000);
  });
}

function discloudCard(onDetail: () => void): HTMLElement {
  const apps = arr(DISCLOUD, "apps");
  const online = apps.filter((a) => a.online === true).length;
  const extra = apps.length
    ? h("span", { class: "int-total" }, h("span", { text: `${online}/${apps.length}` }))
    : undefined;
  const rows = h("div", { class: "int-rows tight" });
  if (apps.length === 0) rows.append(h("div", { class: "int-empty", text: t("int.noApps") }));
  for (const a of apps.slice(0, 3)) {
    const id = String(a.id);
    const up = a.online === true;
    const stats = up ? [a.cpu, a.memory].filter(Boolean).join(" · ") : t("int.offline");
    rows.append(
      h(
        "button",
        {
          class: "int-row int-app",
          title: t("int.details"),
          onclick: () => {
            discloud.selected = id;
            discloud.showLogs = false;
            onDetail();
          },
        },
        dot(up ? "#22C55E" : "#F4505E", 5),
        h("span", { class: "int-name", text: String(a.name ?? id) }),
        h("span", { class: "int-ago", text: stats }),
      ),
    );
  }
  if (apps.length > 3) {
    rows.append(h("div", { class: "int-empty", text: t("int.moreApps", { n: apps.length - 3 }) }));
  }
  return h("div", { class: "int-card" }, header(DISCLOUD_COLOR, "Discloud", t("int.kind.apps"), extra), rows);
}

/** "78.1MB/512MB" → "78.1/512 MB"; anything else is left as Discloud sent it. */
function compactMemory(memory: string): string {
  const m = /^([\d.]+)\s*([KMGT]i?B)\s*\/\s*([\d.]+)\s*\2$/i.exec(memory.trim());
  return m ? `${m[1]}/${m[3]} ${m[2]}` : memory;
}

function discloudDetail(onBack: () => void): HTMLElement | null {
  const a = arr(DISCLOUD, "apps").find((x) => x.id === discloud.selected);
  if (!a) return null;
  const id = String(a.id);
  const up = a.online === true;
  const accent = up ? "#22C55E" : "#F4505E";
  // The card is ~135 px tall: the logs get the whole card instead of a sliver under the buttons.
  if (discloud.showLogs) return discloudLogsView(id, String(a.name ?? id), accent);

  const meta = h("div", { class: "int-meta" });
  // Each stat is its own block so a narrow card wraps whole stats, never mid-word.
  const stats = [
    a.cpu ? `CPU ${a.cpu}` : "",
    a.memory ? `RAM ${compactMemory(String(a.memory))}` : "",
    up && a.startedAt ? t("int.uptime", { t: timeAgo(a.startedAt) }) : "",
  ].filter(Boolean);
  for (const s of stats) meta.append(h("span", { text: s }));

  // No writes while paused or switched off — Rust refuses too, this just says so.
  const blocked = State.paused || !State.settings.activeIntegrations.includes(DISCLOUD);
  const pending = discloud.pending.get(id);
  const buttons = h("div", { class: "int-actions" });
  const available: DiscloudAction[] = up ? ["restart", "stop"] : ["start"];
  for (const action of available) {
    const armed = discloud.armed?.id === id && discloud.armed.action === action;
    buttons.append(
      h("button", {
        class: `int-action${action === "start" ? "" : " danger"}${armed ? " armed" : ""}`,
        disabled: blocked || pending != null,
        text: t(armed ? CONFIRM_LABEL[action] : ACTION_LABEL[action]),
        onclick: () => onDiscloudAction(id, action),
      }),
    );
  }
  buttons.append(
    h("button", {
      class: "int-action",
      text: t("int.logs"),
      onclick: () => {
        discloud.showLogs = true;
        if (!discloud.logs.has(id)) loadDiscloudLogs(id);
        else bumpDiscloud();
      },
    }),
  );

  const notice = discloud.notice?.id === id ? discloud.notice : null;
  const status = pending
    ? h("div", { class: "int-notice", text: t(PENDING_LABEL[pending]) })
    : notice
      ? h("div", { class: "int-notice", style: `color:${notice.ok ? "#22C55E" : "#F5A524"}`, text: notice.text })
      : null;

  return h(
    "div",
    { class: "int-card detail discloud" },
    h(
      "div",
      { class: "int-detail-head" },
      h("button", { class: "int-back", onclick: onBack }, svg(ICONS.chevronLeft, 10, { stroke: 2.4 })),
      dot(accent, 6),
      h("b", { text: String(a.name ?? id) }),
      h("span", {
        class: "int-badge",
        style: `color:${accent};background:${accent}24`,
        text: up ? t("int.online") : t("int.offline"),
      }),
    ),
    meta,
    buttons,
    status,
  );
}

function discloudLogsView(id: string, name: string, accent: string): HTMLElement {
  const blocked = State.paused || !State.settings.activeIntegrations.includes(DISCLOUD);
  const logs = discloud.logs.get(id);
  const loading = discloud.logsLoading.has(id);
  const logText = logs?.error ?? (logs?.text || (loading ? t("int.loading") : t("int.noLogs")));
  const pre = h("pre", { class: `int-detail-text int-logs${logs?.error ? " failed" : ""}`, text: logText });
  pre.addEventListener("scroll", () => {
    const atEnd = pre.scrollTop + pre.clientHeight >= pre.scrollHeight - 4;
    discloud.logScroll.set(id, atEnd ? "end" : pre.scrollTop);
  });
  requestAnimationFrame(() => {
    const at = discloud.logScroll.get(id) ?? "end";
    pre.scrollTop = at === "end" ? pre.scrollHeight : at;
  });

  return h(
    "div",
    { class: "int-card detail discloud" },
    h(
      "div",
      { class: "int-detail-head" },
      h(
        "button",
        {
          class: "int-back",
          onclick: () => {
            discloud.showLogs = false;
            bumpDiscloud();
          },
        },
        svg(ICONS.chevronLeft, 10, { stroke: 2.4 }),
      ),
      dot(accent, 6),
      h("b", { text: `${name} · ${t("int.logs")}` }),
      h("button", {
        class: "link-btn int-logs-refresh",
        style: `color:${DISCLOUD_COLOR}d9`,
        text: loading ? t("int.loading") : t("int.refresh"),
        disabled: loading || blocked,
        onclick: () => loadDiscloudLogs(id),
      }),
    ),
    pre,
  );
}

// ── n8n ───────────────────────────────────────────────────────────────────────

function n8nCard(task: AgentTask, onDetail: () => void, openSettings: () => void): HTMLElement {
  const hasActivity = task.steps.length > 0 && (task.state === "finished" || task.state === "error");
  if (!hasActivity) return idleCard(task, openSettings);
  const success = task.state === "finished";
  const accent = success ? "#22C55E" : "#F4505E";
  return h(
    "div",
    { class: "int-card" },
    header("#F29B38", "n8n", t("int.kind.workflow")),
    h(
      "div",
      { class: "int-actions" },
      h(
        "button",
        {
          class: "int-pill",
          style: `background:${accent}1a;border-color:${accent}38`,
          onclick: onDetail,
        },
        dot(accent, 5),
        h("span", { class: "int-name", text: task.steps[0] ?? t("int.workflow") }),
        svg(ICONS.ellipsis, 8),
      ),
    ),
  );
}

function n8nDetail(task: AgentTask, onBack: () => void): HTMLElement {
  const success = task.state === "finished";
  const accent = success ? "#22C55E" : "#F4505E";
  const detail = task.steps[1];
  return h(
    "div",
    { class: "int-card detail" },
    h(
      "div",
      { class: "int-detail-head" },
      h("button", { class: "int-back", onclick: onBack }, svg(ICONS.chevronLeft, 10, { stroke: 2.4 })),
      dot(accent, 6),
      h("b", { text: task.steps[0] ?? t("int.workflow") }),
      h("span", {
        class: "int-badge",
        style: `color:${accent};background:${accent}24`,
        text: success ? t("int.success") : t("int.failed"),
      }),
    ),
    detail
      ? h("pre", { class: "int-detail-text", text: detail })
      : h("div", {
          class: "int-status",
          text: success ? t("int.completed") : t("int.noErrorDetail"),
        }),
  );
}

// ── Dispatch ──────────────────────────────────────────────────────────────────

export interface IntegrationCardHooks {
  detailOpen: boolean;
  openDetail(): void;
  closeDetail(): void;
  openSettings(): void;
}

/** True when this integration has data worth showing instead of the idle card. */
export function hasIntegrationData(id: string): boolean {
  const info = State.integrations[id];
  if (!info || info.error) return false;
  switch (id) {
    case "integration_vercel":
      return arr(id, "deployments").length > 0;
    case "integration_resend":
      return arr(id, "emails").length > 0;
    case "integration_github":
      return get(id).totalRepos != null;
    case "integration_stripe":
      return info.loaded;
    case "integration_notion":
      return arr(id, "pages").length > 0;
    case "integration_calcom":
    case "integration_discloud":
      return info.loaded;
    default:
      return false;
  }
}

export function renderIntegrationCard(task: AgentTask, hooks: IntegrationCardHooks): HTMLElement {
  if (task.id === "integration_n8n") {
    const hasActivity = task.steps.length > 0 && (task.state === "finished" || task.state === "error");
    return hooks.detailOpen && hasActivity
      ? n8nDetail(task, hooks.closeDetail)
      : n8nCard(task, hooks.openDetail, hooks.openSettings);
  }
  if (task.id === "integration_vercel" && hasIntegrationData(task.id)) {
    return hooks.detailOpen ? vercelDetail(hooks.closeDetail) : vercelCard(hooks.openDetail);
  }
  if (task.id === DISCLOUD && hasIntegrationData(task.id)) {
    const detail = hooks.detailOpen ? discloudDetail(hooks.closeDetail) : null;
    return detail ?? discloudCard(hooks.openDetail);
  }
  if (!hasIntegrationData(task.id)) return idleCard(task, hooks.openSettings);

  switch (task.id) {
    case "integration_resend":
      return resendCard();
    case "integration_github":
      return githubCard();
    case "integration_stripe":
      return stripeCard();
    case "integration_notion":
      return notionCard();
    case "integration_calcom":
      return calcomCard();
    default:
      return idleCard(task, hooks.openSettings);
  }
}

export { clear };
