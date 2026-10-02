# SPEC — Chat com vários provedores: Claude + Gemini (app Windows)

## Objetivo
Hoje o chat da ilha (Mochi) só fala com a API da Anthropic (`windows/src-tauri/src/claude.rs`). Quem já tem uma chave do Google AI Studio, ou prefere o Gemini, não consegue usar o chat. Esta mudança:

1. Deixa o usuário **escolher o provedor** do chat nos Ajustes: **Claude (Anthropic)** ou **Gemini (Google)**.
2. Guarda **uma chave por provedor** no Credential Manager. Trocar de provedor não apaga a outra chave.
3. Mantém tudo o que o chat faz hoje nos dois provedores: várias rodadas, pesquisa na web, arquivo (PDF/imagem/texto) e contexto de janela na primeira mensagem.
4. Deixa o código pronto para um terceiro provedor (OpenAI etc.) sem refazer nada, **sem** implementá-lo agora.

Fora do escopo: app macOS (`NotchBuddy/`), streaming de resposta, lista de modelos buscada da API, OpenAI/Mistral/Ollama, mudar o fluxo dos hooks do Claude Code (não tem nada a ver com o chat).

## API do Gemini (a conferir na Etapa 1 do TODO)
- `POST https://generativelanguage.googleapis.com/v1beta/models/{model}:generateContent`
- Autenticação: header `x-goog-api-key: <chave>` (nunca na query string, para a chave não aparecer em log de URL).
- Corpo:
```json
{
  "systemInstruction": { "parts": [{ "text": "<SYSTEM_PROMPT>" }] },
  "contents": [
    { "role": "user",  "parts": [{ "inlineData": { "mimeType": "application/pdf", "data": "<b64>" } }, { "text": "File: x.pdf" }, { "text": "pergunta" }] },
    { "role": "model", "parts": [ ...partes devolvidas, tal como vieram... ] }
  ],
  "tools": [{ "google_search": {} }]
}
```
- Sem `maxOutputTokens`: nos modelos com raciocínio, os pensamentos contam nesse limite, e um teto baixo devolve uma resposta sem texto.
- Resposta: `candidates[0].content.parts[].text` (juntar só as partes com `text` e sem `thought: true`), `candidates[0].finishReason`, `promptFeedback.blockReason`.
- Erro HTTP vem como `{ "error": { "code", "message", "status" } }` — mesmo caminho `error.message` que a Anthropic.
- Os modelos Gemini 3.x devolvem `thoughtSignature` nas partes: o histórico guarda o `content` do modelo **inteiro, sem mexer**, como já se faz com os blocos `tool_use` do Claude.
- IDs de modelo (conferidos em ai.google.dev/gemini-api/docs/models, 2026-10-02): `gemini-3.8-flash` (padrão), `gemini-3.7-flash`, `gemini-3.5-flash-lite`, `gemini-3.1-pro-preview`. A série 2.5 só funciona para quem já usava: `gemini-2.5-pro` devolve 404 para contas novas.

## Comportamento

### Ajustes
A seção "Claude" vira **"Chat"**:
```
● Chat
  Provedor   [ Claude (Anthropic) ▾ ]
  Chave da API  [••••••• (salva)]  [Salvar chave] [Remover]
  Modelo     [ Claude Opus 5 ▾ ]
```
- Trocar o provedor troca, na hora, o campo de chave (placeholder `sk-ant-...` ou `AIza...`), o status (bolinha) e a lista de modelos daquele provedor.
- Cada provedor lembra o seu próprio modelo. Voltar para o Claude reencontra o modelo Claude escolhido antes.
- A bolinha da seção reflete a chave **do provedor ativo**.
- Um modelo salvo que não está na lista continua aparecendo como opção (comportamento atual de `apiSection`).

### Ilha
- Textos que dizem "Claude" e se referem ao chat passam a usar o nome do provedor ativo: `empty.ask` ("Perguntar ao Claude" / "Perguntar ao Gemini"), `placeholder.searching`. Textos sobre **Claude Code** não mudam.
- A conversa em andamento usa o provedor com que começou. **Trocar o provedor nos Ajustes zera o histórico do chat** (os formatos não são compatíveis); a próxima mensagem começa do zero no provedor novo.
- Sem chave do provedor ativo → a mesma nota de hoje: "Falta a chave da API. Abra os Ajustes."

### Recusa / bloqueio
- Claude: igual a hoje (`stop_reason: "refusal"`).
- Gemini: `promptFeedback.blockReason` presente, ou `finishReason` em `SAFETY`, `PROHIBITED_CONTENT`, `BLOCKLIST`, `SPII`, `RECITATION` → desfaz a mensagem do histórico e mostra "O Gemini recusou este pedido." `MAX_TOKENS` com texto → mostra o texto que veio.

## Modelo de dados

### `settings.json` (`windows/src-tauri/src/settings.rs`, `windows/src/core/state.ts`)
```json
{
  "chatProvider": "anthropic",
  "model": "claude-opus-5",
  "geminiModel": "gemini-3.8-flash"
}
```
- `chatProvider`: `"anthropic" | "gemini"`, padrão `"anthropic"` (`#[serde(default)]`), para um `settings.json` antigo continuar carregando igual.
- `model` **não muda de nome**: continua sendo o modelo do Claude (compatibilidade). `geminiModel` é novo, com padrão próprio.
- Valor desconhecido em `chatProvider` → cai para `anthropic`.

### Segredos (`secrets.rs`)
- Nova chave `"gemini-api-key"` em `KNOWN_KEYS`. `anthropic-api-key` continua igual.

### Comandos Tauri
- `chat_send` e `chat_reset` **não mudam de assinatura**. O Rust lê `chatProvider` + modelo dos settings e despacha.

## Impactos
- `docs/privacy.html` (en + fr): Google na tabela de chaves e um item "Google (Gemini API)" dizendo o que é enviado, com link para a política do Google. **Avisar que no plano gratuito da Gemini API o Google pode usar os dados para melhorar os produtos** (conferir o texto atual dos termos antes de escrever).
- `docs/terms.html`, `docs/support.html`: citar o Google/Gemini ao lado da Anthropic.
- `docs/INTEGRATIONS.md` (§5), `windows/README.md`, `README.md` (tabela de chaves), `CHANGELOG.md`.
- Nenhuma dependência nova (`reqwest`, `serde_json`, `keyring` já existem).
- O nome do provedor nunca vai para o log com a chave; `log.rs` não registra nem chave nem conteúdo de mensagem.

## Casos de borda
- Chave Gemini inválida → 400/403 com `error.message` → nota "Gemini API 400: API key not valid…".
- 429 (cota do plano gratuito) → nota com a mensagem da API; histórico desfeito.
- Modelo Gemini com caractere fora de `[A-Za-z0-9._-]` → recusado no Rust antes da chamada (o id vai no caminho da URL).
- Arquivo: GIF/WEBP/PNG/JPEG/PDF viram `inlineData`; texto até 200 KB vira `text`; maior que isso é ignorado, como hoje. Arquivo inline acima de 18 MB em base64 (a requisição inteira tem limite de 20 MB) → ignorado com a mesma regra, sem quebrar o chat.
- Resposta Gemini sem `candidates` ou só com partes `thought` → "A resposta veio sem texto." e histórico desfeito.
- 429 com `google_search` ligado → uma nova tentativa **sem** a ferramenta. O grounding com Google Search tem cota própria, bem menor (zero em alguns projetos gratuitos). Se a nova tentativa passar, a pesquisa fica desligada até reiniciar o app e o system prompt ganha uma frase avisando que não há pesquisa na web (o modelo diz quando não consegue checar algo atual); se der 429 de novo, a cota geral acabou e a mensagem da API aparece.
- Outro erro da pesquisa na web → mostra a mensagem da API, sem nova tentativa.
- Troca de provedor com uma requisição em andamento → a resposta que chegar é descartada do histórico novo (o histórico guarda o provedor; se não bater, ignora).
