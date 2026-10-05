# SPEC — Perguntas de múltipla escolha do Claude Code (app Windows)

## Objetivo
Quando o Claude Code usa a ferramenta `AskUserQuestion`, hoje o Mochi mostra o card de **aprovação** (Negar / Permitir). Com isso, a pergunta vira um "sim ou não" sem sentido: "Permitir" deixa a pergunta seguir para o terminal sem resposta, e "Negar" cancela a pergunta.

O objetivo é que a ilha mostre a pergunta **como ela é**:
1. Escolha única (`multiSelect: false`): um clique numa opção = resposta.
2. Múltipla escolha (`multiSelect: true`): marcar várias opções e confirmar.
3. Várias perguntas na mesma chamada (1 a 4): uma de cada vez, com "1/3", "2/3"… e Voltar.
4. Resposta livre ("Outra…"): campo de texto, como no terminal.
5. Sempre dá para desistir e **responder no terminal**.

Fora do escopo: app macOS (`NotchBuddy/`), `preview` das opções (só existe no Agent SDK), perguntas feitas fora da `AskUserQuestion` (texto do Claude terminando em "?").

## Por que acontece hoje
- `AskUserQuestion` passa pelo fluxo de permissão → dispara o hook `PermissionRequest` com `tool_name: "AskUserQuestion"`.
- `windows/src/island/hooks.ts` trata todo `PermissionRequest` igual: monta `pendingApproval` e abre a view `approval`.
- O relay (`windows/hook/src/main.rs`) só sabe devolver `allow` ou `deny`.

## Como o Claude Code aceita a resposta
Documentado em https://code.claude.com/docs/en/agent-sdk/user-input e no formato de saída do `PermissionRequest` (https://code.claude.com/docs/en/hooks): `allow` com `updatedInput`.

```json
{"hookSpecificOutput":{"hookEventName":"PermissionRequest","decision":{
  "behavior":"allow",
  "updatedInput":{
    "questions":[ ...as perguntas originais, intactas... ],
    "answers":{
      "Como formatar a saída?":"Resumo",
      "Quais seções incluir?":"Introdução, Conclusão"
    }
  }
}}}
```
- Chave = texto exato de `question`; valor = `label` escolhido. Multi-seleção: labels unidos por `", "`. Texto livre: o próprio texto.
- `questions` precisa voltar **idêntico** ao recebido.
- A doc descreve isso para o `canUseTool` do SDK. **Confirmado no Claude Code 2.1.287** (02/10/2026) que o hook `PermissionRequest` aceita o mesmo `updatedInput`: 3 perguntas numa chamada, com uma de múltipla escolha, respondidas pelo `coucou-hook.exe` real.

## O que os testes reais mostraram
- O Claude Code mostra a pergunta (ou a aprovação) **no terminal ao mesmo tempo** em que roda o hook; quem responder primeiro vence, e o prompt do terminal é cancelado quando o hook responde.
- Quando o **terminal** vence, o Claude Code **não mata o relay**: ele continua esperando até a sessão acabar (ou até o `timeout` do hook). Por isso a desconexão do pipe não basta para limpar o card: a ilha também considera a pergunta resolvida quando chega, da mesma sessão, um `PostToolUse`/`PostToolUseFailure` da mesma ferramenta, `Stop`, `StopFailure`, `SessionEnd` ou `UserPromptSubmit`. Nesse caso ela tira o card e libera o relay (`approvalDecline`). `Notification` não conta: ele chega justamente porque o prompt está esperando.

## Sem timeout de 110 s
Hoje uma aprovação (e, por tabela, uma pergunta) some da ilha depois de 110 s e o terminal assume. Isso sai: **o card fica na tela até alguém responder**, seja pela ilha, pelo botão "Responder no terminal" ou porque o Claude Code desistiu. Vale para perguntas **e** aprovações, porque as duas usam o mesmo canal.

O timeout está em cinco lugares, e todos mudam juntos:
| Onde | Hoje | Depois |
|---|---|---|
| `windows/hook/src/main.rs` `DECISION_BUDGET` | 110 s | sem limite (espera o pipe fechar) |
| `windows/src-tauri/src/pipe.rs` `DECISION_TIMEOUT` | 108 s | sem limite |
| `windows/src/island/hooks.ts` timer do card | 110 s | removido |
| `windows/src-tauri/src/hooks.rs` `timeout` gravado no `settings.json` | 120 s | 86400 s (24 h) |
| Claude Code (lê o `timeout` acima e mata o hook) | 120 s | 24 h |

O Claude Code exige um número, então "sem timeout" na prática vira 24 h.

O timer de hoje servia para **limpar o card** quando o terminal assumia. Sem ele, a limpeza vem dos eventos seguintes da sessão (ver "O que os testes reais mostraram") e de **o pipe fechar**: se o Claude Code matar o `coucou-hook` (Ctrl+C, sessão fechada, a pessoa respondeu no terminal), o servidor do pipe percebe a desconexão e avisa a ilha (`hook-cancelled`), que tira o card. Sem isso, um card órfão ficaria na tela para sempre.

O que continua valendo, porque é o que garante a regra de nunca travar o Claude Code:
- 300 ms para conectar ao pipe (Coucou fechado → o terminal pergunta na hora);
- 800 ms para a ilha confirmar que o card está na tela (ilha pausada ou travada → o terminal pergunta na hora).

Quem já tem os hooks instalados continua com `timeout: 120` no `settings.json`. A tela de configurações precisa avisar que os hooks estão desatualizados e oferecer a reinstalação, que segue a regra de sempre: backup, merge, diff e confirmação.

A aprovação também ganha o botão "Responder no terminal" (e o Esc), porque sem o timeout ele passa a ser a única saída se a pessoa preferir o terminal.

## Modelo de dados (renderer)
```ts
interface QuestionOption { label: string; description?: string }
interface AskQuestion { question: string; header?: string; options: QuestionOption[]; multiSelect: boolean }
interface PendingQuestion {
  requestId: string;
  sessionId: string;
  questions: AskQuestion[];      // 1–4
  index: number;                 // pergunta exibida
  answers: (string[] | null)[];  // labels marcados (ou [textoLivre]) por pergunta
}
```
`State.pendingQuestion` fica ao lado de `State.pendingApproval`; nunca os dois ao mesmo tempo para o mesmo request.

## Fluxo
1. `PermissionRequest` com `tool_name === "AskUserQuestion"` e `tool_input.questions` válido → `pendingQuestion`, ack ao relay, estado `question`, som, `island.alert("question")` (ou badge se outro agente está em foco, igual à aprovação).
2. View `question` mostra: cabeçalho (`header`, "2/3"), texto da pergunta, opções como botões (descrição em linha menor).
   - Escolha única: clicar → grava e avança (ou envia, se for a última).
   - Multi: clicar alterna marcação; botão **Próxima/Enviar** habilitado com ≥ 1 marcada.
   - **Outra…** abre um campo de texto; Enter confirma.
   - **Voltar** (a partir da 2ª) e **Responder no terminal** (sempre).
   - Atalhos: `1`–`4` escolhem a opção; Enter confirma; Esc = responder no terminal.
3. Ao final → `Bridge.questionAnswer(requestId, answers)` → Rust → pipe → relay monta o JSON acima.
4. "Responder no terminal" → `approvalDecline` (o relay não imprime nada; o terminal pergunta normalmente).
5. Perguntas seguidas (várias chamadas de `AskUserQuestion`): cada uma chega como um novo `PermissionRequest` depois que a anterior foi respondida — o fluxo se repete naturalmente.

## Impactos
- `windows/hook/src/main.rs`: novo tipo de resposta; precisa guardar `tool_input.questions` **sem truncar** (hoje strings > 2000 chars são cortadas antes de encaminhar).
- `windows/src-tauri/src/pipe.rs` + `lib.rs`: novo comando `question_answer`, linha no pipe com JSON; espera sem timeout + detecção de desconexão.
- `windows/src-tauri/src/hooks.rs` + tela de configurações: `timeout` 86400 e aviso de hooks desatualizados.
- `windows/src/island/hooks.ts`, `core/state.ts`, `core/bridge.ts`, `views/views.ts`, `island/island.ts`, `island/fsm.ts` (pin), `i18n/en.ts` + `pt-BR.ts`, `style.css`.
- `Notification` com "?" continua mostrando a view `question` antiga (só leitura) quando não há `pendingQuestion`.

## Casos de borda
- `questions` ausente, vazio, > 4 perguntas, opção sem `label` → cai no card antigo de "responda no terminal" + `approvalDecline` (nunca adivinhar).
- Texto livre vazio → não confirma.
- Duas perguntas com o mesmo texto → as respostas colidiriam na chave; declinar para o terminal.
- Chega outro `PermissionRequest` com a pergunta aberta → declina o novo (mesma regra "um card, um request").
- Claude Code mata o hook (Ctrl+C, sessão fechada, timeout de 24 h) → pipe fecha → `hook-cancelled` → card some.
- Coucou fecha/crasha com o card aberto → pipe fecha do lado do servidor → o relay lê EOF, sai sem imprimir nada → terminal pergunta.
- Hooks instalados com o `timeout: 120` antigo → o Claude Code ainda corta em 120 s; o card some via `hook-cancelled` e as configurações mostram "hooks desatualizados".
- Coucou pausado → `approvalDecline` imediato (já existe).
- Labels/perguntas com aspas, acentos, emoji, quebras de linha → JSON gerado com `serde_json`, nunca `format!`.
- Modo `bypassPermissions`/regra allow para `AskUserQuestion`: o `PermissionRequest` pode não disparar — então o Mochi não interfere e o terminal pergunta (comportamento atual).
- Nunca responder sem clique explícito.
