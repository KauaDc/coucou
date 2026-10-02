# PLANO — Chat com vários provedores (app Windows)

Spec: `docs/specs/SPEC_chat-multi-provider-windows.md`

## Estratégia
Separar "o que é comum a qualquer chat" (histórico, contexto de arquivo/janela, despacho) de "como falar com cada API". Cada provedor fica num arquivo próprio com a mesma forma de função; um terceiro provedor depois é só mais um arquivo e um braço no `match`.

## Camadas, na ordem

### 1. Modelo (Rust)
- `settings.rs`: enum `ChatProvider { Anthropic, Gemini }` com `#[serde(rename_all = "lowercase")]` e `Default = Anthropic`; campos `chat_provider` e `gemini_model`, ambos com `#[serde(default…)]`. Um `chatProvider` desconhecido não pode derrubar o `load()` inteiro (hoje `unwrap_or_default` zeraria **todos** os ajustes): usar um `deserialize_with` que cai para `Anthropic`.
- `secrets.rs`: `"gemini-api-key"` em `KNOWN_KEYS`.

### 2. Núcleo do chat (Rust)
- Novo `chat.rs` com o que hoje está misturado em `claude.rs`:
  - `struct Chat { provider: Mutex<Option<ChatProvider>>, messages: Mutex<Vec<Value>> }` — o histórico sabe de qual provedor é. `send` compara com o provedor atual e faz `reset()` se mudou.
  - `ChatContext`, `ChatReply`, `SYSTEM_PROMPT`, `MAX_TOKENS`, `MAX_INLINE_TEXT`, `base64`/`base64_for` (Stripe usa — atualizar o import em `integrations.rs`).
  - `enum FilePayload { Binary { kind: Document|Image, mime, bytes_b64 }, Text(String) }` + `read_file(path) -> Option<FilePayload>` (lógica atual de `file_block`, sem formato de API).
  - `pub async fn send(chat, settings_snapshot, query, context)` → monta a mensagem do usuário pedindo ao provedor (`provider::user_message(payload, texts)`), empurra, chama, trata erro/recusa, empurra a resposta.
  - Erro comum `http_error(provider_label, status, body)` lendo `error.message` (as duas APIs usam esse caminho).
- `claude.rs` fica só com: `user_message(...)`, `call(key, model, history) -> Result<Turn, String>` onde `Turn { stored: Value, text: String }` ou `Refusal(String)`.
- Novo `gemini.rs` com as mesmas funções:
  - `user_message` → `{ role: "user", parts: [...] }` (`inlineData` para binário).
  - `call` → `POST …/models/{model}:generateContent`, header `x-goog-api-key`, `tools: [{ google_search: {} }]`, `systemInstruction`, `generationConfig.maxOutputTokens`.
  - Validação do `model` (`[A-Za-z0-9._-]{1,64}`) antes de montar a URL.
  - Recusa: `promptFeedback.blockReason` ou `finishReason` ∈ {SAFETY, PROHIBITED_CONTENT, BLOCKLIST, SPII, RECITATION}.
  - `stored` = `candidates[0].content` inteiro (preserva `thoughtSignature`); `text` = partes com `text` e sem `thought: true`.
- Padrões: mesmo `reqwest::Client` com timeout 90 s, mensagens via `crate::i18n::t(en, pt)`, nada de chave ou conteúdo no `log.rs`.

### 3. IPC (Rust)
- `lib.rs`: `chat_send` passa a ler `(chat_provider, model, gemini_model)` dos settings e chamar `chat::send`. Assinatura do comando inalterada.
- Onde os settings são salvos (comando existente de salvar ajustes): se `chat_provider` mudou, `chat.reset()` e emitir o evento que a ilha já usa para limpar `State.chatHistory` (conferir qual; se não houver, a ilha limpa ao receber os settings novos).
- `mod chat; mod gemini;` no topo.

### 4. Renderer (TS)
- `core/state.ts`: tipos `chatProvider: "anthropic" | "gemini"`, `geminiModel: string` + padrões em `DEFAULT_SETTINGS`. Helper `providerLabel()` → "Claude" / "Gemini". Ao receber settings com provedor diferente, limpar `chatHistory`.
- `settings/main.ts`: `apiSection` vira `chatSection(present: Record<Provider, boolean>)`:
  - tabela `PROVIDERS = { anthropic: { key: "anthropic-api-key", placeholder: "sk-ant-...", models: [...], modelField: "model" }, gemini: { key: "gemini-api-key", placeholder: "AIza...", models: [...], modelField: "geminiModel" } }`;
  - `<select>` de provedor que redesenha chave/status/modelos; `refresh()` usa a chave do provedor ativo.
  - Carregamento inicial (linha ~480) busca a presença das duas chaves.
- `i18n/en.ts` e `pt-BR.ts`: `settings.chat.title`, `settings.chat.provider`, `empty.ask` e `placeholder.searching` com `{provider}`; ajustar quem chama `t(...)` para passar `providerLabel()`.

### 5. Docs
`privacy.html` (en+fr), `terms.html`, `support.html`, `INTEGRATIONS.md`, `windows/README.md`, `README.md`, `CHANGELOG.md`.

## Tratamento de erro
| Situação | Onde | Resultado |
|---|---|---|
| Sem chave | `chat::send` | nota "Falta a chave da API. Abra os Ajustes." |
| HTTP ≠ 2xx | `http_error` | "`<Claude|Gemini>` API `<status>`: `<error.message>`", histórico desfeito |
| Rede | `call` | "Erro de rede: …", histórico desfeito |
| Recusa/bloqueio | cada provedor | "O `<Claude|Gemini>` recusou este pedido.", histórico desfeito |
| Sem texto | `chat::send` | "A resposta veio sem texto.", histórico desfeito |
| Modelo inválido | `gemini::call` | "Modelo inválido.", sem chamada de rede |
| Provedor trocou no meio | `chat::send` | resposta descartada se `chat.provider` ≠ provedor da requisição |

## Testes
- Unitários em `gemini.rs`: montagem do corpo (texto, arquivo binário, contexto de janela), extração de texto ignorando `thought`, detecção de bloqueio, validação de modelo.
- Unitário em `settings.rs`: `settings.json` antigo (sem os campos novos) e com `chatProvider` desconhecido carregam sem perder os outros ajustes.
- `chat.rs`: troca de provedor zera o histórico.
- Manuais: ver Etapa final do TODO.
