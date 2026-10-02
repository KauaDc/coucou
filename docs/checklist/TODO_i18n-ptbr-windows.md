# TODO — Tradução pt-BR do app Windows

Spec: `docs/specs/SPEC_i18n-ptbr-windows.md` · Plano: `docs/plans/PLANO_i18n-ptbr-windows.md`

## Etapa 1 — Núcleo i18n
- [x] `windows/src/i18n/en.ts` — dicionário base (`as const`)
- [x] `windows/src/i18n/pt-BR.ts` — `Record<keyof typeof en, string>`
- [x] `windows/src/i18n/index.ts` — `detect()`, `LANG`, `t()` com placeholders e fallback
- [x] `windows/src/main.ts` — `document.documentElement.lang = LANG`
- [x] `rtk npm run build` passa

## Etapa 2 — Views simples
- [x] `src/views/chat.ts` — placeholders "Ask me anything…", "Continue…", rótulos
- [x] `src/views/ticker.ts`
- [x] `src/views/upload.ts` — "Drop your files here", "Uploading {name}", "What do you want to do with it?", botões
- [x] build (conferência visual pendente) (`?lang=pt-BR`)

## Etapa 3 — Views principais
- [x] `src/views/views.ts` — idle, permission, question, error, finished, overload, placeholders, auto-close, tooltips (`title: "Open"`)
- [x] build (conferência visual pendente) de cada view

## Etapa 4 — Integrações
- [x] `src/views/integrations.ts` — status ("Connected · loading…", "Key not configured"…), botões "Open …", estatísticas, "No calls scheduled"
- [x] tempo relativo ("just now", "{n} min ago"…) com chaves próprias
- [x] `toLocaleDateString/TimeString(LANG, …)`
- [x] build (conferência visual pendente)

## Etapa 5 — Rótulos de ferramentas
- [x] `src/island/hooks.ts` — `TOOL_LABELS` (hoje em francês) → chaves `tool.*` em en e pt-BR
- [x] não tocar na detecção de rate limit (linha ~188)

## Etapa 6 — Canvas de upload
- [x] `src/upload/canvas.ts` — textos desenhados via `t()`
- [x] conferir que pt-BR cabe nas posições fixas (botão centralizado, linha de upload); encurtar tradução se preciso

## Etapa 7 — Janela de Ajustes
- [x] `src/settings/main.ts` — seções Claude Code, Claude, integrações, botões, avisos, placeholders, rótulos de campos
- [x] `settings.html` — "Loading…" e `lang` em runtime
- [x] build (conferência visual pendente)

## Etapa 8 — Rust
- [x] `src-tauri/Cargo.toml` — feature `Win32_Globalization` no crate `windows`
- [x] `src-tauri/src/i18n.rs` — `Lang`, `lang()` com `OnceLock`, `tr()`
- [x] `src-tauri/src/lib.rs` — `mod i18n;` + título "Settings — Coucou"
- [x] `src-tauri/src/tray.rs` — Open Coucou, Settings…, Pause, Quit
- [x] `src-tauri/src/hooks.rs` — "No change.", "Can't read …"
- [x] `cargo build` passa

## Etapa 9 — Fechamento
- [x] grep por literais em inglês remanescentes em `windows/src` e `src-tauri/src`
- [ ] teste no Windows real em pt-BR e em en (pendente — feito só build + tsc + cargo test)
- [x] `CHANGELOG.md` — entrada "Windows: interface em português quando o sistema está em português"
