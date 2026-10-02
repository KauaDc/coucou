# SPEC — Ilha em vários monitores (app Windows)

## Objetivo
Hoje, com dois ou mais monitores, a ilha do Coucou para Windows (`windows/`) fica presa a uma tela só:
- **"Tela principal"** (padrão) sempre usa o monitor principal.
- **"Tela sob o cursor"** escolhe o monitor certo só no momento em que a ilha abre ou fecha. Com a ilha escondida, a faixa invisível que a acorda (`STRIP_W × STRIP_H`, 240×6) continua no monitor onde estava, e o polling do cursor fica parado (0 % de CPU). Levar o mouse ao topo de outro monitor não faz nada.

A entrega tem duas partes:
1. **Faixas em todos os monitores** — no modo "Tela sob o cursor", cada monitor ganha uma faixa de despertar no topo central. Passar o mouse em qualquer uma abre a ilha naquele monitor.
2. **Monitor fixo** — o seletor "Tela da ilha" passa a listar os monitores conectados ("Monitor 2 — 2560×1440"), para fixar a ilha num monitor que não seja o principal.

Fora do escopo: app macOS (`NotchBuddy/`, que segue a tela com notch), ilha simultânea em vários monitores (continua havendo **uma** ilha, que se muda de tela), arrastar a ilha para outra posição.

## Modos do seletor "Tela da ilha"
| Valor salvo em `settings.screen` | Rótulo (pt-BR / en) | Onde a ilha abre | Faixas de despertar com a ilha escondida |
|---|---|---|---|
| `"primary"` (padrão, inalterado) | Tela principal / Main display | monitor principal | 1, no principal (como hoje) |
| `"cursor"` (inalterado) | Tela sob o cursor / Display under the cursor | monitor do cursor | **1 por monitor** (novo) |
| `"monitor:<id>"` (novo) | Monitor N — L×A / Display N — W×H | o monitor `<id>`; se não estiver conectado, o principal | 1, no monitor escolhido |

- `<id>` é o nome de dispositivo do Windows que `tauri::Monitor::name()` devolve (ex.: `\\.\DISPLAY2`). É estável enquanto a placa de vídeo e a porta não mudam, o que basta para um app pessoal.
- Para não errar de tela quando o Windows renumera os monitores (ex.: depois de uma atualização de driver), o valor salvo traz também posição e tamanho: `"monitor:\\.\DISPLAY2@2560x1440+1920+0"`. Na busca: nome **e** geometria → só nome → só geometria → principal.
- Ordem e numeração da lista: da esquerda para a direita, depois de cima para baixo (pela posição física), com "(principal)" no principal. O número exibido é só um rótulo, não é salvo.

## Modelo de dados
- `Settings.screen: String` (Rust, `settings.rs`) e `screen: "primary" | "cursor" | \`monitor:${string}\`` (TS, `core/state.ts`). Sem migração: os valores antigos continuam válidos.
- Novo comando Tauri `list_monitors` → `MonitorInfo[]`:
  ```ts
  { id: string; label: string; primary: boolean; x: number; y: number; width: number; height: number; scale: number }
  ```
  (`width`/`height` em pixels físicos, que é o que o usuário reconhece: "2560×1440".)
- Nenhum dado novo em disco além do valor de `screen`.

## Fluxo — Parte 1 (faixas em todos os monitores)
1. A ilha recolhe (`set_collapsed(true)`) com `screen == "cursor"`.
2. O Rust posiciona a janela da ilha como faixa no monitor do cursor (como hoje) e cria uma **faixa nativa** (janela Win32 sem WebView, ver PLANO) no topo central de **cada um dos outros** monitores.
3. O cursor entra numa faixa nativa → o Rust emite `wake` para a janela da ilha → o front chama `fsm.mouseEntered()`, o mesmo caminho do `mouseenter` da faixa atual.
4. A ilha chama `set_collapsed(false)` → `apply_geometry` com `"cursor"` → como o cursor já está no novo monitor, a ilha abre nele. As faixas nativas são escondidas.
5. Conectar, desconectar, reorganizar ou mudar a escala de um monitor com a ilha escondida: as faixas nativas recebem `WM_DISPLAYCHANGE` / `WM_DPICHANGED` e são recriadas. Sem polling.

## Fluxo — Parte 2 (monitor fixo)
1. Na janela de Ajustes, o seletor chama `list_monitors` ao abrir e quando o Rust emite `monitors-changed`.
2. O usuário escolhe "Monitor 2" → `save_settings` com `screen = "monitor:…"` → `apply_geometry` move a ilha (caminho que já existe, `lib.rs:82`).
3. Se o monitor salvo some, a ilha vai para o principal **sem apagar a preferência**. Quando ele volta, a ilha volta para ele (detectado pelo `current_screen_key` / `screen-changed` que já existe, ou pelas faixas nativas se a ilha estiver escondida).

## Impactos
- `windows/src-tauri/src/island.rs` — `target_monitor` entende `monitor:`; gerencia as faixas nativas.
- Novo `windows/src-tauri/src/wake_strips.rs` — janelas Win32 das faixas.
- `windows/src-tauri/src/lib.rs` — comando `list_monitors`; `set_collapsed` mostra/esconde as faixas.
- `windows/src-tauri/Cargo.toml` — feature `Win32_System_LibraryLoader` (e `Win32_Graphics_Gdi`, se precisar) no crate `windows`, que já é usado. **Nenhuma dependência nova.**
- `windows/src/core/state.ts`, `core/bridge.ts`, `main.ts`, `island/island.ts` — tipo, comando, evento `wake`.
- `windows/src/settings/main.ts` + `i18n/en.ts` + `i18n/pt-BR.ts` — seletor dinâmico e textos.
- `windows/README.md` — documentar o comportamento com vários monitores.

## Regras do projeto que continuam valendo
- **0 % de CPU com a ilha escondida**: as faixas nativas não fazem polling; quem avisa a entrada do mouse é o Windows (`WM_MOUSEMOVE`), e a thread de polling continua parada.
- A faixa nunca pode roubar foco nem aparecer no Alt-Tab nem na barra de tarefas (`WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW`).
- Invisível: nada aparece na tela por causa das faixas.

## Casos de borda
- **Um monitor só**: nenhuma faixa nativa é criada; comportamento idêntico ao atual.
- **Monitores com escalas diferentes** (100 % + 150 %): tamanho da faixa calculado por monitor (`STRIP_W × scale`); a ilha reafirma o tamanho físico ao mudar de tela (já faz isso em `apply_geometry`).
- **Monitor acima/abaixo ou deslocado** (topo não alinhado): a faixa fica no topo **daquele** monitor (`position.y`), não em y = 0.
- **Barra de tarefas no topo**: a faixa fica por cima dela, como hoje (janela topmost); conferir que um clique no topo central da barra continua chegando à barra (a faixa tem 6 px e só acorda a ilha no hover, não consome cliques — validar).
- **Jogo/app em tela cheia** num monitor: a faixa topmost não pode atrapalhar. Mesmo comportamento da faixa atual; não piorar.
- **Monitor desconectado com a ilha aberta nele**: `screen-changed` reposiciona (já existe).
- **Monitor fixo desconectado**: fallback para o principal; o seletor mostra "Monitor desconectado" como opção selecionada, para o usuário saber que a preferência foi mantida.
- **Hover na faixa nativa durante a animação de recolher** (os 420 ms antes do `setCollapsed(true)`): as faixas só existem depois do recolhimento; não há corrida.
- **Faixa nativa e faixa da ilha no mesmo monitor**: nunca; o monitor que tem a janela da ilha não ganha faixa nativa.
- **Cursor parado sobre a área de uma faixa no momento em que ela é criada**: não acorda (é preciso que o cursor *entre*), igual à faixa atual.
- **Ilha pausada** (`set_paused`) ou fixada: o despertar segue as mesmas regras do `mouseenter` atual; a FSM decide.
