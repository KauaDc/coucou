# PLANO — Ilha em vários monitores (app Windows)

Referência: `docs/specs/SPEC_multi-monitor-windows.md`.

## Estratégia
Duas partes independentes, na ordem de menor risco: primeiro a **Parte 2 (monitor fixo)**, que só mexe em `target_monitor` e na UI, e depois a **Parte 1 (faixas em todos os monitores)**, que cria janelas novas.

Sem dependências novas. As faixas são janelas Win32 puras (o crate `windows` já está no projeto), **não** janelas Tauri/WebView2: cada WebView custa dezenas de MB e um processo de renderização, só para detectar um hover.

## Parte 2 — Monitor fixo

### Camada 1 — Rust: escolha do monitor (`island.rs`)
- `fn monitor_id(m: &Monitor) -> String` → `"monitor:{name}@{w}x{h}+{x}+{y}"`.
- `fn parse_monitor_pref(pref) -> Option<(name, w, h, x, y)>`.
- `target_monitor(app, pref)`:
  ```rust
  if let Some(want) = pref.strip_prefix("monitor:") {
      // 1. nome + geometria  2. só nome  3. só geometria  → senão cai no principal
  }
  ```
  O caminho `"cursor"` e o fallback para o principal ficam como estão.
- `current_screen_key` não muda: como já chama `target_monitor`, o monitor fixo voltando dispara `screen-changed` sozinho.

### Camada 2 — Rust: comando (`lib.rs`)
```rust
#[derive(Serialize)] #[serde(rename_all = "camelCase")]
pub struct MonitorInfo { id, label, primary, x, y, width, height, scale }

#[tauri::command]
fn list_monitors(app: AppHandle) -> Vec<MonitorInfo>
```
- Ordenado por `(x, y)`. O `label` fica a cargo do front (i18n); o Rust devolve só os dados.
- Registrar em `generate_handler!`.
- Emitir `monitors-changed` para a janela `settings` quando `current_screen_key` mudar (mesmo ponto de `island.rs:300`) e quando as faixas nativas receberem `WM_DISPLAYCHANGE` (Parte 1).

### Camada 3 — Front: tipo e ponte
- `core/state.ts`: `screen: "primary" | "cursor" | \`monitor:${string}\``.
- `core/bridge.ts`: `listMonitors: () => call<MonitorInfo[]>("list_monitors")`; evento `monitors-changed` no union de eventos.

### Camada 4 — Front: Ajustes (`settings/main.ts`)
- `fillScreenSelect()` assíncrono: "Tela principal", "Tela sob o cursor", depois `t("settings.general.screenMonitor", { n, w, h })` para cada monitor, com sufixo `t("settings.general.screenPrimarySuffix")` no principal.
- Se `settings.screen` começa com `monitor:` e não está na lista → opção extra `t("settings.general.screenMissing")`, selecionada.
- Recarrega em `monitors-changed`.
- Chaves novas em `i18n/en.ts` e `i18n/pt-BR.ts` (o `tsc` acusa se faltar alguma):
  - `screenMonitor`: "Display {n} — {w}×{h}" / "Monitor {n} — {w}×{h}"
  - `screenPrimarySuffix`: " (main)" / " (principal)"
  - `screenMissing`: "Disconnected display" / "Monitor desconectado"

## Parte 1 — Faixas em todos os monitores

### Camada 5 — Rust: `wake_strips.rs` (novo)
Responsável por N janelas Win32 invisíveis, uma por monitor (menos o da ilha).

- **Classe de janela** `"CoucouWakeStrip"`, registrada uma vez (`RegisterClassW`, `GetModuleHandleW` → feature `Win32_System_LibraryLoader`).
- **Janela**: `CreateWindowExW` com
  `WS_EX_TOOLWINDOW | WS_EX_TOPMOST | WS_EX_NOACTIVATE | WS_EX_LAYERED`, estilo `WS_POPUP`.
  `SetLayeredWindowAttributes(hwnd, 0, 1, LWA_ALPHA)` → alfa 1/255: invisível a olho nu, mas ainda recebe o mouse (alfa 0 deixa o mouse passar em alguns casos — **validar na Etapa 6**; se alfa 1 aparecer, testar `LWA_COLORKEY`).
- **Thread**: tudo criado via `app.run_on_main_thread(...)`. O laço de eventos do tao já processa as mensagens de todas as janelas da thread principal, então não precisa de thread nem de laço de mensagens próprios.
- **WndProc**:
  - `WM_MOUSEMOVE` → `app.emit_to(WINDOW_LABEL, "wake", ())` (com uma flag para emitir uma vez só por recolhimento).
  - `WM_MOUSEACTIVATE` → `MA_NOACTIVATE`.
  - `WM_DISPLAYCHANGE`, `WM_DPICHANGED` → `rebuild()` + emitir `monitors-changed`.
  - `WM_NCHITTEST` → `HTCLIENT` (padrão).
- **Estado**: `static STRIPS: Mutex<Vec<isize>>` (HWNDs como `isize`, porque `HWND` não é `Send`) + `OnceLock<AppHandle>` para a WndProc conseguir emitir eventos.
- **API pública**:
  ```rust
  pub fn show_except(app: &AppHandle, island_monitor: Option<&Monitor>)  // cria/reposiciona
  pub fn hide_all(app: &AppHandle)                                        // DestroyWindow em todas
  ```
  Mais simples recriar a cada recolhimento do que manter e reposicionar: recolher é raro (segundos/minutos), e criar uma janela Win32 leva microssegundos.
- **Geometria** por monitor: largura `STRIP_W * scale`, altura `STRIP_H * scale`, `x = mp.x + (ms.width - w)/2`, `y = mp.y` (mesma fórmula de `apply_geometry`; extrair para `fn strip_rect(m: &Monitor) -> (i32,i32,u32,u32)` e usar nos dois lugares).

### Camada 6 — Rust: ligar no ciclo de recolhimento (`lib.rs`)
`set_collapsed`:
```rust
island::apply_geometry(&app, &pref, collapsed);
if collapsed && pref == "cursor" {
    wake_strips::show_except(&app, island::current_monitor(&app).as_ref());
} else {
    wake_strips::hide_all(&app);
}
```
- `save_settings` com troca de `screen` enquanto recolhido: chamar o mesmo trecho.
- Ao sair (`quit_app`): `hide_all`.
- `island::current_monitor(app)` = `win.current_monitor()` da janela da ilha.

### Camada 7 — Front: evento `wake` (`main.ts` / `island.ts`)
- `bridge.ts`: `| { name: "wake"; payload: null }`.
- `island.ts`: extrair o corpo do `mouseenter` da `wakeStrip` para `private onWake()` e usá-lo tanto no listener quanto em `onEvent("wake", …)`.
- Nada muda na FSM: `onWake` → `fsm.mouseEntered()` → `setCollapsed(false)` → `apply_geometry("cursor")` → a ilha abre no monitor do cursor, que é aquele cuja faixa foi tocada.

### Camada 8 — Documentação
- `windows/README.md`: linha na tabela de interações e trecho sobre vários monitores.
- `CHANGELOG` / docs, se o projeto mantiver um para o Windows.

## Tratamento de erro
- `list_monitors` falha → lista vazia; o seletor mostra só "Tela principal" e "Tela sob o cursor".
- Monitor salvo não encontrado → principal, sem sobrescrever a preferência.
- `RegisterClassW` / `CreateWindowExW` falha → registrar em `log::line` e seguir com só a faixa da ilha (o comportamento de hoje). Nunca derrubar o app.
- `emit_to` falha (janela da ilha ainda não existe) → ignorar.
- `HWND` inválido em `hide_all` (janela já destruída pelo sistema) → `DestroyWindow` só falha; ignorar e limpar a lista.

## Verificação
Não há testes automatizados no app Windows.
1. `rtk npm run build` em `windows/` (o `tsc` pega chaves e tipos).
2. `cargo build` + `cargo clippy` em `windows/src-tauri`.
3. Teste manual com 2 monitores (de preferência com escalas diferentes):
   - Parte 2: escolher "Monitor 2" → a ilha muda na hora; reiniciar → continua no 2; desconectar o 2 → vai para o principal e o seletor mostra "Monitor desconectado"; reconectar → volta.
   - Parte 1: modo "Tela sob o cursor", deixar a ilha recolher no monitor A, passar o mouse no topo central do B → abre no B; recolher no B e voltar ao A → abre no A.
   - Gerenciador de Tarefas com a ilha escondida: **0 % de CPU** e nenhuma janela nova no Alt-Tab nem na barra de tarefas.
   - Faixa invisível: conferir com fundo branco e fundo preto em cada monitor.
   - Plugar/desplugar com a ilha escondida → as faixas acompanham.
   - App em tela cheia num monitor → nada aparece por cima.
4. Um monitor só → nenhuma regressão.

## Riscos
- **Alfa 1 com `WS_EX_LAYERED`** pode ficar levemente visível em alguns painéis ou não receber o mouse em versões específicas do Windows → testar cedo (Etapa 6 do TODO, protótipo isolado) antes de ligar no resto.
- **WndProc na thread principal do tao**: deve funcionar, porque o tao despacha mensagens de todas as janelas da thread. Se não funcionar, o plano B é uma thread dedicada com `GetMessageW` (que fica bloqueada, então 0 % de CPU).
- **Nome do monitor instável** (`\\.\DISPLAYn` muda depois de trocar de porta ou driver) → mitigado pela busca com geometria.
- **Faixa sobre a barra de tarefas no topo** em outros monitores pode atrapalhar cliques no centro da barra → mesma limitação da faixa atual; documentar e validar.
