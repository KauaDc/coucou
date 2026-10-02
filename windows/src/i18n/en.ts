// English strings — the reference dictionary. Every other language must define
// exactly these keys (see pt-BR.ts). `{name}` placeholders are filled by t().

export const en = {
  // Island chrome
  "tab.overview": "Overview",
  "tab.ask": "Ask",
  "tab.drop": "Drop",
  "tab.settings": "Settings",
  "tab.mute": "Mute",

  // Overview
  "overview.open": "Open",

  // Empty
  "empty.title": "Nothing running right now.",
  "empty.sub": "Drop a file or window, or ask me anything.",
  "empty.ask": "Ask Claude",

  // Approval
  "approval.who": "needs permission",
  "approval.deny": "Deny",
  "approval.allow": "Allow",

  // Question
  "question.who": "Claude Code is asking a question",
  "question.fallback": "Claude needs an answer.",
  "question.sub": "Answer in your terminal — Coucou can't reply for you yet.",

  // Error
  "error.workflowStopped": "Workflow stopped.",
  "error.sessionStopped": "Session stopped on an error.",
  "error.noDetail": "No detail available.",
  "error.retry": "Retry",
  "error.openN8n": "Open in n8n",

  // Finished
  "finished.who": "Claude Code finished",
  "finished.fallback": "Session finished",
  "finished.openTerminal": "Open terminal",
  "finished.ok": "OK",

  // Confused
  "confused.title": "Too many hits at once.",
  "confused.sub": "Give me a sec — back to work in three seconds.",

  // In-island settings
  "islandSettings.sound": "Sound",
  "islandSettings.autoClose": "Auto-close · {s}s",
  "islandSettings.more": "Settings…",

  // Placeholders
  "placeholder.mail": "Sending by email isn't in this version.",
  "placeholder.searching": "Claude is searching…",
  "placeholder.result": "Result",

  // Chat
  "chat.placeholder": "Ask me anything…",
  "chat.continue": "Continue…",
  "chat.send": "Send",

  // Upload
  "upload.dropHere": "Drop your files here",
  "upload.tag.pdf": "PDF",
  "upload.tag.images": "Images",
  "upload.tag.code": "Code",
  "upload.tag.docs": "Docs",
  "upload.uploading": "Uploading {name}",
  "upload.file": "file",
  "upload.fileCap": "File",
  "upload.isReady": " is ready.",
  "upload.isReadyFull": "{name} is ready.",
  "upload.whatToDo": "What do you want to do with it?",
  "upload.ask": "Ask a question",
  "upload.askAbout": "Ask a question about it",
  "upload.cancel": "Cancel",

  // Integrations
  "int.justNow": "just now",
  "int.ago": "{t} ago",
  "int.hooksMissing": "Hooks not installed",
  "int.keyMissing": "Key not configured",
  "int.loading": "Connected · loading…",
  "int.openVSCode": "Open Visual Studio Code",
  "int.openN8n": "Open n8n",
  "int.open": "Open {name}",
  "int.refresh": "Refresh",
  "int.settings": "Settings…",
  "int.kind.integration": "Integration",
  "int.kind.deployments": "Deployments",
  "int.kind.emails": "Emails",
  "int.kind.overview": "Overview",
  "int.kind.payments": "Payments",
  "int.kind.recent": "Recent",
  "int.kind.schedule": "Schedule",
  "int.kind.workflow": "Workflow",
  "int.details": "Details",
  "int.ready": "Ready",
  "int.canceled": "Canceled",
  "int.error": "Error",
  "int.deployment": "Deployment",
  "int.totalStars": "Total stars",
  "int.repositories": "Repositories",
  "int.payment": "Payment",
  "int.untitled": "Untitled",
  "int.noCalls": "No calls scheduled",
  "int.meeting": "Meeting",
  "int.workflow": "Workflow",
  "int.success": "Success",
  "int.failed": "Failed",
  "int.completed": "Completed successfully.",
  "int.noErrorDetail": "No error details available.",

  // Claude Code steps (ticker)
  "tool.Bash": "Runs",
  "tool.Read": "Reads",
  "tool.Write": "Writes",
  "tool.Edit": "Edits",
  "tool.Glob": "Finds",
  "tool.Grep": "Searches",
  "tool.WebSearch": "Web search",
  "tool.WebFetch": "Fetches",
  "tool.TodoWrite": "Tasks",
  "tool.Task": "Agent",
  "tool.LS": "Lists",
  "tool.MultiEdit": "Edits",
  "tool.NotebookEdit": "Notebook",
  "tool.PowerShell": "Runs",
  "step.session": "Session",
  "step.tool": "Tool",
  "step.failed": "⚠ failed",
  "step.subagent": "+ subagent",
  "step.subagentDone": "• subagent done",

  // Settings window
  "settings.windowTitle": "Settings — Coucou",
  "settings.loading": "Loading…",
  "settings.hooks.installed":
    "Coucou is hooked into your Claude Code sessions. Tool calls, questions and permission requests show up in the island, and you can answer them there.",
  "settings.hooks.notInstalled":
    "Install the hooks to see your Claude Code sessions in the island and approve permissions without leaving what you are doing.",
  "settings.hooks.relay": "Relay",
  "settings.hooks.relayMissing":
    "coucou-hook.exe is not in place yet. Restart Coucou; if it still fails, build it with `cargo build -p coucou-hook`.",
  "settings.hooks.reinstall": "Reinstall hooks…",
  "settings.hooks.install": "Install hooks…",
  "settings.hooks.relayNotInstalled": "The relay isn't installed yet.",
  "settings.hooks.uninstall": "Uninstall hooks…",
  "settings.back": "Back",
  "settings.hooks.previewInstall":
    "This is exactly what will change in your settings.json. Your own hooks are left untouched.",
  "settings.hooks.previewRemove": "This removes Coucou's entries only. Your own hooks are left untouched.",
  "settings.hooks.backup": "Backup → {path}",
  "settings.hooks.write": "Back up and write",
  "settings.hooks.remove": "Back up and remove",
  "settings.hooks.done":
    "Done. Previous settings saved as {backup}. Open a new Claude Code session to pick the hooks up.",
  "settings.couldNotWrite": "Could not write: {err}",
  "settings.cancel": "Cancel",
  "settings.api.saved": "Key saved in the Windows Credential Manager.",
  "settings.api.none": "No key yet — the chat needs one.",
  "settings.api.stored": "(stored)",
  "settings.api.save": "Save key",
  "settings.api.remove": "Remove",
  "settings.api.savedOk": "Saved. It never touches disk.",
  "settings.couldNotSave": "Could not save: {err}",
  "settings.api.removed": "Key removed.",
  "settings.couldNotRemove": "Could not remove: {err}",
  "settings.api.key": "API key",
  "settings.api.model": "Model",
  "settings.int.title": "Integrations",
  "settings.int.note":
    "Pick up to {max} pills to show next to Mochi — {used}/{max} in use. Keys are stored in the Windows Credential Manager, never on disk.",
  "settings.int.secretKey": "Secret key",
  "settings.int.token": "Token",
  "settings.int.instanceUrl": "Instance URL",
  "settings.int.integrationToken": "Integration token",
  "settings.save": "Save",
  "settings.general.title": "General",
  "settings.general.sound": "Sound",
  "settings.general.autoClose": "Auto-close",
  "settings.general.autoCloseHint": "seconds after you leave the island",
  "settings.general.screen": "Island lives on",
  "settings.general.screenMain": "Main display",
  "settings.general.screenCursor": "Display under the cursor",
  "settings.general.autostart": "Launch at startup",
  "settings.privacy": "No telemetry. Network requests only go to the services you configure yourself.",
} as const;
