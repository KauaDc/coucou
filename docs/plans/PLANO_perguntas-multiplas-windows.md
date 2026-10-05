# PLANO — Perguntas de múltipla escolha (app Windows)

Spec: `docs/specs/SPEC_perguntas-multiplas-windows.md`.

## Estratégia
Reaproveitar o canal da aprovação (pipe com request pendente, ack, decline) e trocar só o **conteúdo** da resposta: em vez de uma palavra (`allow`/`deny`), uma linha `answers <json>`. O relay é quem conhece as `questions` originais, então é ele quem monta o `updatedInput` — o app nunca reenvia as perguntas (menos coisa para dar errado, e o texto não passa pela truncagem).

## Ordem das camadas

### 0. Fim do timeout de 110 s (antes de tudo: muda o canal que as perguntas vão usar)
- **Relay** (`hook/src/main.rs`): `PermissionRequest` deixa de ter `DECISION_BUDGET`. A thread principal faz `rx.recv()` sem prazo; quem encerra é o pipe (EOF ou erro de leitura → `None`). `FIRE_AND_FORGET_BUDGET` e `CONNECT_TIMEOUT` ficam como estão.
- **Pipe** (`src-tauri/src/pipe.rs`): remover `DECISION_TIMEOUT`. Depois do ack, `tokio::select!` entre `rx.recv()` e um `pipe.read()` de 1 byte: se a leitura der 0/erro, o cliente morreu → `emit_to(WINDOW_LABEL, "hook-cancelled", {request_id})`, tirar o id do `Pending` e retornar. O `ACK_TIMEOUT` de 800 ms fica.
- **Ilha** (`src/island/hooks.ts`): remover o `setTimeout(…, 110_000)` e `pendingTimeout`. A limpeza do card vira a função `releaseCard(requestId)`, chamada por `onEvent("hook-cancelled")`, e só age se o id for o do card aberto.
- **Instalação** (`src-tauri/src/hooks.rs`): `("PermissionRequest", 86400)`. `HookStatus` ganha `outdated: bool` (alguma entrada nossa com `timeout` diferente do de `HOOK_EVENTS`). Configurações mostram "Hooks desatualizados — Reinstalar", que passa pelo fluxo já existente de preview, diff e confirmação.
- **Aprovação** (`views/views.ts`): botão "Responder no terminal" + Esc → `approvalDecline`.
- Atualizar os comentários que citam 108/110 s (`pipe.rs` cabeçalho, `main.rs` cabeçalho, `hooks.rs`).
- Testes: `merged()` grava 86400; `outdated` detecta 120; relay sem resposta + pipe fechado → nada no stdout.

### 1. Relay — `windows/hook/src/main.rs`
- Antes de `truncate_strings`, se `tool_name == "AskUserQuestion"`, guardar uma cópia de `tool_input.questions` (o `read_event` passa a devolver também esse `Option<Value>`).
- `decision_json(decision, questions)`:
  - `"allow" | "always" | "deny"` → como hoje.
  - `"answers {…}"` → parse do objeto; só aceita se `questions` existe e **toda** chave bate com algum `question` e todo valor é string não vazia. Monta com `serde_json::json!` o `decision: {behavior: "allow", updatedInput: {questions, answers}}`.
  - Qualquer outra coisa → `None` (silêncio = terminal pergunta).
- Testes unitários: formato documentado, acentos/aspas, chave desconhecida → `None`, sem `questions` → `None`.

### 2. App Rust — `windows/src-tauri/src/pipe.rs` e `lib.rs`
- `pipe::answer_questions(app, request_id, answers: serde_json::Map)` → serializa e envia `answers {json}` pelo mesmo `Reply` pendente; log sem o conteúdo das respostas (só contagem).
- Comando Tauri `question_answer(request_id, answers)` registrado no `invoke_handler`.
- O `wait_for_decision` já aceita qualquer string (o timeout já saiu na etapa 0).

### 3. Bridge / estado — `windows/src/core/bridge.ts`, `core/state.ts`
- `Bridge.questionAnswer(requestId, answers: Record<string,string>)`.
- Tipos `AskQuestion`, `PendingQuestion`; `State.pendingQuestion: PendingQuestion | null`.

### 4. Roteamento do hook — `windows/src/island/hooks.ts`
- No `PermissionRequest`: se `tool_name === "AskUserQuestion"`, chamar `parseQuestions(tool_input)`; válido → `pendingQuestion` + ack + estado `question` + `alert("question")`/badge; inválido → `approvalDecline` e view `question` só-leitura.
- Regra "um card, um request" passa a olhar `pendingApproval || pendingQuestion`.
- `hook-cancelled` (etapa 0) limpa `pendingApproval` e `pendingQuestion`.

### 5. View — `windows/src/views/views.ts`, `island/island.ts`, `style.css`
- `buildQuestion(actions)` em dois modos: interativo (há `pendingQuestion`) e o atual só-leitura.
- Botões de opção construídos uma vez por pergunta (chave `requestId:index`), igual ao cuidado do `buildApproval` para não engolir clique.
- Novas ações em `ViewActions`: `pickOption(i)`, `submitQuestion()`, `backQuestion()`, `otherAnswer(text)`, `answerInTerminal()`.
- `island.ts` implementa as ações; ao enviar: som `approve`, limpa estado, despina, volta à view padrão (espelho de `decide`).
- Atalhos de teclado no handler existente (1–4, Enter, Esc).
- Altura do card: medir com 4 opções + descrição; se passar do limite do layout, descrição vira 1 linha com reticências.

### 6. Textos — `windows/src/i18n/en.ts`, `pt-BR.ts`
`question.next`, `question.send`, `question.back`, `question.other`, `question.otherPlaceholder`, `question.terminal`, `question.progress` ("{n}/{total}"), `question.multiHint`. Ajustar `question.sub` (hoje diz que o Coucou não responde).

## Padrões e erros
- Nunca enviar resposta sem clique; nunca adivinhar: qualquer formato inesperado → declinar para o terminal.
- JSON sempre via `serde_json`, nada de concatenação.
- Sem dependências novas.
- Logs sem o conteúdo das respostas (privacidade).

## Verificação
- `cargo test` em `windows/hook` e `windows/src-tauri`.
- `npm run build` (typecheck) em `windows/`.
- Teste manual com Claude Code real: escolha única, multi, 3 perguntas numa chamada, "Outra…", duas chamadas seguidas, "Responder no terminal", card aberto > 2 min, Ctrl+C no terminal com card aberto, fechar o Coucou com card aberto.
