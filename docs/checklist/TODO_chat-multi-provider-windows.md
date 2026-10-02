# TODO — Chat com vários provedores (app Windows)

Spec: `docs/specs/SPEC_chat-multi-provider-windows.md` · Plano: `docs/plans/PLANO_chat-multi-provider-windows.md`

### Etapa 1 — Conferir a API do Gemini com uma chave real
> Sem chave na implementação: lista de modelos e `DEFAULT_MODEL` vieram de uma busca (2026-10-02). Os formatos estão cobertos por testes unitários com respostas de exemplo. Conferir junto com a Etapa 8.
- [x] Lista padrão de modelos e padrão de `geminiModel` (página oficial de modelos; `gemini-2.5-pro` dava 404 para conta nova)
- [ ] `generateContent` com `google_search`: conferir formato de `candidates`, `parts`, `thoughtSignature`, `finishReason`
- [ ] Segunda rodada reenviando o `content` do modelo como veio: confirmar que não dá erro de assinatura
- [ ] PDF e imagem via `inlineData`: confirmar limite de tamanho
- [x] Ler os termos atuais da Gemini API sobre uso de dados no plano gratuito (para o `privacy.html`)
- [ ] Anotar divergências na SPEC

### Etapa 2 — Modelo (Rust)
- [x] `windows/src-tauri/src/secrets.rs` — `"gemini-api-key"` em `KNOWN_KEYS`
- [x] `windows/src-tauri/src/settings.rs` — `ChatProvider` (padrão `Anthropic`, valor desconhecido → `Anthropic`), `chat_provider`, `gemini_model`
- [x] `settings.rs` — testes: JSON antigo carrega; `chatProvider` inválido não zera os outros ajustes
- [x] `cargo test` + `cargo clippy` passam (28 testes; avisos restantes do clippy já existiam em `hooks.rs`/`island.rs`)

### Etapa 3 — Núcleo do chat (Rust)
- [x] `windows/src-tauri/src/chat.rs` — `Chat` com provedor do histórico, `ChatContext`, `ChatReply`, `SYSTEM_PROMPT`, constantes, `base64`, `read_file` → `FilePayload`
- [x] `chat.rs` — `send` (despacho, reset ao trocar provedor, desfazer histórico em erro, descartar resposta de provedor antigo), `http_error`
- [x] `windows/src-tauri/src/claude.rs` — reduzir a `user_message` + `call`; comportamento idêntico ao atual (fallback beta, web search, recusa)
- [x] `windows/src-tauri/src/integrations.rs` — import de `base64_for` aponta para `chat`
- [x] `cargo build` passa
- [ ] Chat Claude funciona igual a antes (teste manual rápido)

### Etapa 4 — Provedor Gemini (Rust)
- [x] `windows/src-tauri/src/gemini.rs` — `user_message`, `call`, validação do modelo, bloqueio, extração de texto sem `thought`
- [x] `gemini.rs` — testes unitários (corpo, extração, bloqueio, modelo inválido)
- [x] `cargo test` + `cargo clippy` passam

### Etapa 5 — IPC (Rust)
- [x] `windows/src-tauri/src/lib.rs` — `mod chat; mod gemini;`, `chat_send` usa `chat::send` com o snapshot dos settings
- [x] `lib.rs` — ao salvar settings com provedor diferente: `chat.reset()` + avisar a ilha
- [x] `cargo build` passa

### Etapa 6 — Renderer (TS)
- [x] `windows/src/core/state.ts` — `chatProvider`, `geminiModel`, padrões, `providerLabel()`, limpar `chatHistory` ao trocar provedor
- [x] `windows/src/settings/main.ts` — `PROVIDERS`, `chatSection` (provedor, chave e modelo por provedor), presença das duas chaves no carregamento
- [x] `windows/src/i18n/en.ts` e `pt-BR.ts` — `settings.chat.*`, `empty.ask` e `placeholder.searching` com `{provider}`
- [x] Chamadores de `empty.ask` / `placeholder.searching` passam `providerLabel()`
- [x] `tsc --noEmit` + `vite build` passam

### Etapa 7 — Docs
- [x] `docs/privacy.html` — Google na tabela + item Gemini (en + fr), com o aviso do plano gratuito
- [x] `docs/terms.html`, `docs/support.html` — citar Google/Gemini
- [x] `docs/INTEGRATIONS.md` §5, `windows/README.md`, `README.md` (tabela de chaves)
- [x] `CHANGELOG.md` — entrada

### Etapa 8 — Teste manual final
- [ ] `settings.json` antigo → abre com Claude e o modelo de antes
- [ ] Claude: conversa de 3 rodadas, pesquisa na web, PDF, imagem, contexto de janela
- [ ] Gemini: o mesmo roteiro
- [ ] Trocar provedor com conversa aberta → histórico some, próxima mensagem vai para o novo
- [ ] Sem chave do provedor ativo → nota "Falta a chave da API"; a chave do outro continua salva
- [ ] Chave Gemini inválida → mensagem da API
- [x] Cota da pesquisa esgotada (429) → resposta chega sem pesquisa, e o Mochi avisa que não pode checar a web (confirmado pelo usuário em 2026-10-02)
- [ ] Textos "Perguntar ao Gemini" / "O Gemini está pesquisando…" em pt-BR e en
