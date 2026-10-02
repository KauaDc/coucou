# TODO — Integração Discloud (app Windows)

Spec: `docs/specs/SPEC_discloud-windows.md` · Plano: `docs/plans/PLANO_discloud-windows.md`

### Etapa 1 — Conferir a API com um token real
> Sem token disponível na implementação: o parser aceita `apps` como lista ou objeto e campos ausentes. Conferir junto com a Etapa 9.
- [ ] `GET /app/all` e `GET /app/all/status` com `curl -H "api-token: …"`: confirmar que `apps` é lista e os nomes dos campos
- [ ] `GET /app/{id}/logs`: confirmar `terminal.small` / `big` / `url`
- [ ] Ver se a resposta traz headers de rate limit; ajustar o intervalo de 60 s se preciso
- [ ] Anotar divergências na SPEC

### Etapa 2 — Segredo (Rust)
- [x] `windows/src-tauri/src/secrets.rs` — `"discloud-token"` em `KNOWN_KEYS`

### Etapa 3 — Poller somente leitura (Rust)
- [x] `windows/src-tauri/src/integrations.rs` — comentário do topo menciona Discloud
- [x] `integrations.rs` — `discloud_get(path)` com header `api-token` e mapeamento 401/429/rede
- [x] `integrations.rs` — `as_list` (lista ou objeto)
- [x] `integrations.rs` — `poll_discloud`: `/app/all` + `/app/all/status`, junção, ordenação, `emit`
- [x] `integrations.rs` — `DiscloudMemory` (online por app, `quiet_until`, `primed`, `skip_next`) e escolha do evento
- [x] `integrations.rs` — `start()`: `spawn(…, "integration_discloud", 10, 60, poll_discloud)`; `poll_once()`: ramo novo
- [x] `cargo build` + `cargo clippy` passam (+ 7 testes unitários em `integrations::tests`)

### Etapa 4 — Registro no front
- [x] `windows/src/core/state.ts` — `INTEGRATION_AGENTS` + `TOGGLEABLE_INTEGRATION_IDS` (não mexer em `DEFAULT_SETTINGS`)
- [x] `windows/src/island/integrations.ts` — `KEY_FOR.integration_discloud`
- [x] `windows/src/island/island.ts` — URL do dashboard
- [x] `windows/src/settings/main.ts` — entrada Discloud com o campo do token + `"discloud-token"` na lista `keys`
- [x] `windows/src-tauri/src/settings.rs` — conferir que o id novo não é filtrado
- [x] `tsc --noEmit` + `vite build` passam

### Etapa 5 — Card e detalhe somente leitura (front)
- [x] `windows/src/views/integrations.ts` — `OPEN_URLS`, `hasIntegrationData`
- [x] `views/integrations.ts` — `discloudCard` (até 4 linhas, "+N apps", "Nenhum app")
- [x] `views/integrations.ts` — `discloudSelected` + `discloudDetail` (status, CPU/RAM, uptime, voltar)
- [x] `views/integrations.ts` — ramo em `renderIntegrationCard`
- [x] `windows/src/i18n/en.ts` e `pt-BR.ts` — chaves de leitura (`int.kind.apps`, `int.online`, `int.offline`, `int.noApps`, `int.moreApps`, `int.uptime`, `int.openPanel`)
- [x] `tsc --noEmit` + `vite build` passam
- [ ] Teste manual: token → card aparece; derrubar o app no painel → badge vermelho + som; religar → verde

### Etapa 6 — Comandos (Rust)
- [x] `integrations.rs` — validação (`PAUSED`, `enabled`, `appId` regex sem `all`, `action` por `match`)
- [x] `integrations.rs` — `discloud_action`: `quiet_until`, `PUT`, `log::line` sem segredos, poll agora + após 8 s
- [x] `integrations.rs` — `discloud_logs`: corte de 200 linhas / 32 KB
- [x] `windows/src-tauri/src/lib.rs` — wrappers `#[tauri::command]` + `generate_handler!`
- [x] `cargo build` + `cargo clippy` passam

### Etapa 7 — Ações e logs na UI (front)
- [x] `windows/src/core/bridge.ts` — `discloudAction`, `discloudLogs`
- [x] `views/integrations.ts` — botões Reiniciar/Parar/Iniciar, confirmação inline de 4 s, `pending`, `notice` de 6 s
- [x] `views/integrations.ts` — logs com `logsCache`, `<pre>` via `textContent`, scroll preservado, "Atualizar logs"
- [x] `views/integrations.ts` — botões desabilitados com app pausada ou integração desligada
- [x] `windows/src/style.css` — `.int-logs`, `.int-action`, `.danger`, `.armed`
- [x] `i18n/en.ts` e `pt-BR.ts` — chaves de ação e logs
- [x] `tsc --noEmit` + `vite build` passam

### Etapa 8 — Docs
- [x] `docs/privacy.html` — Discloud na tabela (en + fr) e item explicando que as ações só saem com clique
- [x] `docs/INTEGRATIONS.md` — seção Discloud (Windows)
- [x] `windows/README.md` — lista de integrações
- [x] `CHANGELOG.md` — entrada

### Etapa 9 — Teste manual final
- [ ] Sem token / integração desligada → nenhuma requisição (conferir no log)
- [ ] Pausar pela bandeja → poll e botões param
- [ ] Token inválido → "Chave de API inválida (401)"
- [ ] Parar com confirmação → app cai **sem** som de erro; Iniciar → volta
- [ ] Reiniciar → spinner, estado atualizado em ~8 s
- [ ] Clique único em Parar e esperar 4 s → nada acontece
- [ ] Logs abrem, rolam para o fim, não pulam a cada poll
- [ ] Ilha em pt-BR e en
