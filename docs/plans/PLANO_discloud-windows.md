# PLANO — Integração Discloud (app Windows)

Referência: `docs/specs/SPEC_discloud-windows.md`.

## Estratégia
Copiar o formato da Vercel (`integrations.rs:328`, `views/integrations.ts:122`) e acrescentar duas coisas que nenhuma integração tem hoje: **memória de estado por app** (para os eventos) e **comandos de escrita** (ações). Primeiro só leitura de ponta a ponta, depois as ações. Sem dependências novas.

Ordem: Rust (segredo → poller → comandos) → ponte → estado → UI → i18n → docs.

## Camada 1 — Rust: segredo (`secrets.rs`)
- `"discloud-token"` em `KNOWN_KEYS`. Sem isso `secrets::get` devolve `None` e nada acontece.

## Camada 2 — Rust: poller (`integrations.rs`)
- `start()`: `spawn(app, "integration_discloud", 10, 60, poll_discloud)`.
- `poll_once()`: ramo `"integration_discloud"`.
- Helper `fn discloud_get(path) -> Result<Value, String>` com o header `api-token`, timeout de `client()`, e mapeamento de erro:
  - 401 → `status_error(401, …)`, 429 → "Limite da API, tentando de novo", rede → "Sem conexão: …".
- Helper `fn as_list(v: &Value) -> Vec<&Value>`: `apps` como array **ou** objeto.
- `poll_discloud`:
  1. `GET /app/all` → apps (id, name, online, ramKilled, exitCode, ram). Falhou → `emit` com `error` e retorna.
  2. `GET /app/all/status` → mapa `id → (cpu, memory, startedAt)`. Falhou → segue sem métricas.
  3. Junta, ordena (offline primeiro, depois nome), `emit` com `data: { apps }`.
- Estado por app, separado do `SEEN` atual (que guarda um id só):
  ```rust
  struct DiscloudMemory {
      online: HashMap<String, bool>,
      quiet_until: HashMap<String, Instant>, // ação do usuário em andamento
      primed: bool,
      skip_next: bool,                       // após 429
  }
  static DISCLOUD: LazyLock<Mutex<DiscloudMemory>>
  ```
  - `primed == false` → preenche e não gera evento.
  - Para cada app: compara com o anterior; ignora se `quiet_until > now`; remove ids que sumiram.
  - Escolhe um evento: erros primeiro; `detail` = "Ficou offline" / "Sem memória" / "Saiu com código N"; com mais de um app, sufixo "+N outros".
- Textos via `crate::i18n::t(en, pt)`, como os outros pollers.

## Camada 3 — Rust: comandos (`integrations.rs` + `lib.rs`)
```rust
#[derive(Serialize)] #[serde(rename_all = "camelCase")]
pub struct DiscloudActionResult { ok: bool, message: String }

pub async fn discloud_action(app: AppHandle, app_id: String, action: String) -> DiscloudActionResult
pub async fn discloud_logs(app_id: String) -> Result<DiscloudLogs, String>
```
- Validação antes de qualquer rede: `PAUSED` → recusa; `enabled(app, "integration_discloud")` → senão recusa; `app_id` casa `^[A-Za-z0-9_-]{1,64}$` e `!= "all"`; `action ∈ {start, stop, restart}` (`match` com `&'static str`, nunca interpolar a string recebida).
- `PUT /app/{id}/{action}`; antes da chamada, `quiet_until[id] = now + 180 s`.
- Depois: `tauri::async_runtime::spawn` → `poll_discloud` agora e outra vez após 8 s.
- `log::line` registra só `"discloud {action} {id} → {status}"`, **nunca** o corpo dos logs nem o token.
- `discloud_logs`: `GET /app/{id}/logs` → `terminal.small` (ou `big`), cortado nas últimas ~200 linhas / 32 KB, e `terminal.url`.
- `lib.rs`: wrappers `#[tauri::command] async fn discloud_action/discloud_logs` registrados em `generate_handler!` (junto de `refresh_integration`, `lib.rs:426`).

## Camada 4 — Ponte (`core/bridge.ts`)
```ts
discloudAction: (appId: string, action: "start" | "stop" | "restart") =>
  call<{ ok: boolean; message: string }>("discloud_action", { appId, action }),
discloudLogs: (appId: string) => call<{ text: string; url: string | null }>("discloud_logs", { appId }),
```

## Camada 5 — Estado e registro (`core/state.ts`, `island/*`, `settings/main.ts`)
- `INTEGRATION_AGENTS`: `task("integration_discloud", "Discloud", "#14B8A6", "n8n")` (cor teal livre na paleta atual; ajustar se destoar).
- `TOGGLEABLE_INTEGRATION_IDS`: + `integration_discloud`. **Não** entra em `DEFAULT_SETTINGS`.
- `island/integrations.ts` `KEY_FOR`: `integration_discloud: "discloud-token"`.
- `island/island.ts:124` e `views/integrations.ts:50` (`OPEN_URLS`): `https://discloud.com/dashboard`.
- `settings/main.ts:273`: entrada `{ id, name: "Discloud", color, fields: [{ key: "discloud-token", label: t("settings.int.token"), secret: true }] }`, e `"discloud-token"` na lista `keys` (`settings/main.ts:481`).
- Conferir `settings.rs`: lista padrão em Rust (`settings.rs:40`) não muda; só garantir que um id desconhecido não é filtrado.

## Camada 6 — UI (`views/integrations.ts` + `style.css`)
- `hasIntegrationData`: `integration_discloud` → `info.loaded` (inclui conta vazia, que mostra "Nenhum app").
- `renderIntegrationCard`: ramo próprio como o da Vercel, porque o detalhe depende de **qual app** foi clicado:
  - estado de módulo `let discloudSelected: string | null`; `discloudCard(onPick)` grava o id e chama `hooks.openDetail()`.
  - `discloudDetail(onBack)` lê o app selecionado de `State.integrations`; se ele sumiu → volta para o card.
- Ações no detalhe:
  - `armed: { id, action, until }` para a confirmação inline de 4 s (Parar/Reiniciar). Timeout chama `State.notify()` para redesenhar.
  - `pending: Set<string>` → botões `disabled` enquanto há requisição.
  - Resultado: `notice` (texto + cor) por 6 s.
- Logs: `logsCache: Map<id, { text, url, at }>`; busca ao abrir o detalhe se não houver cache de < 30 s; botão "Atualizar logs". `<pre class="int-logs">` com `textContent` (nunca `innerHTML`: o log é texto arbitrário). Rolar para o fim após renderizar.
- O detalhe é redesenhado a cada `State.notify()`; guardar e restaurar o `scrollTop` do `<pre>` para os logs não pularem.
- `style.css`: `.int-logs` (monoespaçado, 11px, altura máx. ~120px, `overflow:auto`, `white-space:pre-wrap`) e `.int-action` / `.int-action.danger` / `.int-action.armed`. Reusar tokens de cor existentes.

## Camada 7 — i18n (`i18n/en.ts`, `i18n/pt-BR.ts`)
Chaves novas (as duas línguas): `int.kind.apps`, `int.online`, `int.offline`, `int.noApps`, `int.moreApps`, `int.restart`, `int.stop`, `int.start`, `int.confirmStop`, `int.confirmRestart`, `int.restarting`, `int.stopping`, `int.starting`, `int.logs`, `int.refreshLogs`, `int.noLogs`, `int.openPanel`, `int.uptime`. Textos do Rust via `i18n::t`.

## Camada 8 — Docs
- `docs/privacy.html` (en + fr): Discloud na tabela de chaves e um item próprio: lê status/logs e envia iniciar/parar/reiniciar **só com clique**.
- `docs/INTEGRATIONS.md`: seção Discloud (Windows apenas). `windows/README.md`: lista de integrações. `CHANGELOG.md`.

## Tratamento de erro (resumo)
| Situação | Onde | Resultado |
|---|---|---|
| Sem token / desligada / pausada | spawn, comandos | nenhuma rede; comando devolve `ok:false` |
| 401 | poll | card idle com erro |
| 429 | poll | mantém dados, aviso, pula 1 ciclo |
| `/status` falha | poll | apps sem métricas |
| Ação falha | comando | `ok:false` + `message` da API |
| `appId`/`action` inválidos | comando | recusa sem rede |
| JSON inesperado | parsers | campos ausentes viram `null`/padrão, sem `unwrap` que dê `panic` |

## Verificação
- `cargo build` + `cargo clippy` em `windows/src-tauri`; `rtk npm run build` em `windows/`; `rtk npm test` se houver testes do front.
- Teste manual com um token real e um app de teste (ver TODO, Etapa 9).
