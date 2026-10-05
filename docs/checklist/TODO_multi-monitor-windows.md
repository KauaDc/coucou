# TODO — Ilha em vários monitores (app Windows)

Spec: `docs/specs/SPEC_multi-monitor-windows.md` · Plano: `docs/plans/PLANO_multi-monitor-windows.md`

## Parte 2 — Monitor fixo

### Etapa 1 — Escolha do monitor (Rust)
- [x] `windows/src-tauri/src/island.rs` — `monitor_id(&Monitor)` e `parse_monitor_pref(&str)`
- [x] `island.rs` — `target_monitor`: ramo `monitor:` (nome+geometria → nome → geometria → principal)
- [x] `island.rs` — extrair `strip_rect(&Monitor)` de `apply_geometry` (reusado na Parte 1)
- [x] `windows/src-tauri/src/settings.rs` — atualizar o comentário de `screen` com o novo formato
- [x] `cargo build` passa

### Etapa 2 — Comando `list_monitors` (Rust)
- [x] `windows/src-tauri/src/lib.rs` — `MonitorInfo` + `list_monitors`, ordenado por posição
- [x] `lib.rs` — registrar em `generate_handler!`
- [x] `island.rs` — emitir `monitors-changed` junto com `screen-changed` (`island.rs:300`)
- [x] `cargo build` passa

### Etapa 3 — Ponte e tipos (front)
- [x] `windows/src/core/state.ts` — tipo `screen` com `` `monitor:${string}` ``
- [x] `windows/src/core/bridge.ts` — `MonitorInfo`, `listMonitors()`, evento `monitors-changed`
- [x] `rtk npm run build` passa

### Etapa 4 — Seletor nos Ajustes (front)
- [x] `windows/src/i18n/en.ts` — `screenMonitor`, `screenPrimarySuffix`, `screenMissing`
- [x] `windows/src/i18n/pt-BR.ts` — as mesmas chaves
- [x] `windows/src/settings/main.ts` — `fillScreenSelect()` assíncrono + opção "Monitor desconectado"
- [x] `settings/main.ts` — recarregar a lista em `monitors-changed`
- [x] `rtk npm run build` passa
- [ ] Teste manual: escolher Monitor 2 → muda na hora; reiniciar → mantém; desconectar → principal + "Monitor desconectado"; reconectar → volta

## Parte 1 — Faixas em todos os monitores

### Etapa 5 — Feature do crate `windows`
- [x] `windows/src-tauri/Cargo.toml` — `Win32_System_LibraryLoader` (+ `Win32_Graphics_Gdi` se o compilador pedir)

### Etapa 6 — Protótipo da faixa nativa (validar o risco primeiro)
- [x] `windows/src-tauri/src/wake_strips.rs` — classe `CoucouWakeStrip`, `CreateWindowExW` com `WS_EX_TOOLWINDOW | WS_EX_TOPMOST | WS_EX_NOACTIVATE | WS_EX_LAYERED`, alfa 1
- [x] WndProc: `WM_MOUSEMOVE` → evento `wake` (protótipo fundido com a Etapa 7)
- [x] Faixas criadas via `run_on_main_thread`
- [ ] Validar: invisível em fundo branco/preto · recebe o hover · não rouba foco · fora do Alt-Tab/barra de tarefas · WndProc chamada na thread do tao
- [ ] Se falhar: plano B do PLANO (`LWA_COLORKEY` ou thread com `GetMessageW`)

### Etapa 7 — `wake_strips.rs` completo
- [x] `static STRIPS: Mutex<Vec<isize>>` + `OnceLock<AppHandle>`
- [x] `show_except(app, island_monitor)` — uma faixa por monitor, menos o da ilha, usando `strip_rect`
- [x] `hide_all(app)` — `DestroyWindow` + limpar a lista
- [x] WndProc: `WM_MOUSEMOVE` → `emit_to("island", "wake")` uma vez só · `WM_MOUSEACTIVATE` → `MA_NOACTIVATE` · `WM_DISPLAYCHANGE`/`WM_DPICHANGED` → rebuild + `monitors-changed`
- [x] Erros → `log::line`, nunca `panic`
- [x] `lib.rs` — `mod wake_strips;`
- [x] `cargo build` + `cargo clippy` passam

### Etapa 8 — Ciclo de recolhimento (Rust)
- [x] `island.rs` — `current_monitor(app)` da janela da ilha
- [x] `lib.rs` — `set_collapsed`: `show_except` se recolhido e `screen == "cursor"`, senão `hide_all`
- [x] `lib.rs` — `save_settings`: reaplicar quando `screen` mudar com a ilha recolhida
- [x] ~~`lib.rs` — `quit_app`: `hide_all`~~ — dispensado: o Windows destrói as janelas do processo ao sair
- [x] `cargo build` passa

### Etapa 9 — Evento `wake` (front)
- [x] `windows/src/core/bridge.ts` — `{ name: "wake"; payload: null }`
- [x] `windows/src/island/island.ts` — extrair `onWake()` do `mouseenter` da `wakeStrip`
- [x] `island.ts` (ou `main.ts`) — `onEvent("wake", () => this.onWake())`
- [x] `rtk npm run build` passa

### Etapa 10 — Verificação completa
- [ ] 2 monitores, modo "Tela sob o cursor": recolher no A → hover no topo do B → abre no B, e o inverso
- [ ] Escalas diferentes (100 % + 150 %): tamanho da faixa e da ilha corretos nos dois
- [ ] Monitor deslocado na vertical: a faixa fica no topo daquele monitor
- [ ] Plugar/desplugar com a ilha escondida → faixas recriadas
- [ ] Gerenciador de Tarefas com a ilha escondida: 0 % de CPU
- [ ] Nenhuma janela nova no Alt-Tab nem na barra de tarefas
- [ ] App em tela cheia num monitor → nada por cima
- [ ] Barra de tarefas no topo → cliques no centro continuam funcionando
- [ ] Modos "Tela principal" e "Monitor N" → nenhuma faixa nativa criada
- [ ] Um monitor só → comportamento idêntico ao atual

### Etapa 11 — Documentação
- [x] `windows/README.md` — vários monitores (tabela de interações + seção de configurações)
- [x] `CHANGELOG.md` — entrada em Unreleased
