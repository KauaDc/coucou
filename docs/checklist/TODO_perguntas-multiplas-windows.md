# TODO — Perguntas de múltipla escolha (app Windows)

## 0. Prova de conceito
- [x] Hook de teste que responde um `AskUserQuestion` com `updatedInput.answers` fixo e confirmar no Claude Code real que a resposta é aceita (escolha única e multi)

## 0b. Fim do timeout de 110 s
- [x] `hook/src/main.rs`: `PermissionRequest` espera sem prazo; sai em EOF/erro do pipe
- [x] `src-tauri/src/pipe.rs`: remover `DECISION_TIMEOUT`; `select!` resposta × desconexão do cliente → `hook-cancelled`
- [x] `src/island/hooks.ts`: remover timer 110 s; `releaseCard(requestId)` via `onEvent("hook-cancelled")`
- [x] `src-tauri/src/hooks.rs`: `PermissionRequest` → 86400; `HookStatus.outdated`
- [x] Configurações: aviso "Hooks desatualizados — Reinstalar" (fluxo backup/diff/confirmação)
- [x] Aprovação: botão "Responder no terminal" + Esc
- [x] Comentários que citam 108/110 s atualizados
- [x] Testes: 86400 no merge, `outdated` com 120, relay sem resposta + pipe fechado → stdout vazio

## 1. Relay — `windows/hook/src/main.rs`
- [x] `read_event` guarda `tool_input.questions` sem truncar quando `tool_name == "AskUserQuestion"`
- [x] `decision_json` aceita `answers {json}` e monta `updatedInput { questions, answers }`
- [x] Validação: chaves batem com `question`, valores strings não vazias; senão `None`
- [x] Testes: formato documentado, acentos/aspas, chave desconhecida, sem questions
- [x] `cargo test`

## 2. App Rust — `windows/src-tauri/src/`
- [x] `pipe.rs`: `answer_questions` envia `answers {json}` pelo request pendente (log só com contagem)
- [x] `lib.rs`: comando `question_answer` + registro no `invoke_handler`
- [x] `cargo test`

## 3. Bridge e estado — `windows/src/core/`
- [x] `bridge.ts`: `questionAnswer(requestId, answers)`
- [x] `state.ts`: tipos `AskQuestion` / `PendingQuestion`, campo `pendingQuestion`

## 4. Hook no renderer — `windows/src/island/hooks.ts`
- [x] `parseQuestions(tool_input)` com as validações da spec (1–4 perguntas, 2–4 opções, textos únicos)
- [x] Ramo `AskUserQuestion` no `PermissionRequest` (ack, estado, alerta/badge, som)
- [x] "Um card, um request" considera `pendingQuestion`
- [x] `hook-cancelled` limpa `pendingQuestion`

## 5. View — `windows/src/views/views.ts`, `island/island.ts`, `style.css`
- [x] `buildQuestion` interativo: header, progresso, pergunta, opções com descrição
- [x] Escolha única: clique grava e avança/envia
- [x] Multi: alternar marcação + Próxima/Enviar
- [x] "Outra…" com campo de texto
- [x] Voltar e "Responder no terminal"
- [x] Ações em `ViewActions` implementadas em `island.ts`
- [x] Atalhos 1–4 / Enter / Esc
- [x] Modo só-leitura mantido para `Notification` com "?"
- [x] Altura do card com 4 opções conferida

## 6. Textos — `windows/src/i18n/`
- [x] Chaves novas em `en.ts` e `pt-BR.ts`; `question.sub` atualizado

## 6b. Achado no teste real
- [x] Terminal vence → o relay não morre: card liberado por `PostToolUse`/`Stop`/`StopFailure`/`SessionEnd`/`UserPromptSubmit` da mesma sessão (`settledElsewhere` em `hooks.ts`)
- [x] Clicar na pílula do Claude reabre o card pendente

## 7. Verificação
- [x] `npm run build` em `windows/`
- [x] Real com Claude Code + relay real (pipe falso no lugar do app): 3 perguntas com multi; terminal respondendo primeiro
- [x] Preview no navegador (`windows/dev/question-preview.html`): escolha única, multi, Voltar, "Outra…", No terminal, aprovação
- [ ] Manual no app instalado: duas chamadas seguidas, card aberto > 2 min, Ctrl+C com card aberto, fechar o Coucou com card aberto
- [x] `CHANGELOG.md`
