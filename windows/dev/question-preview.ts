// Dev harness: puts an AskUserQuestion card (or an approval card) on the island
// in a plain browser, so it can be clicked through without Claude Code. The
// answers that would go to Claude Code are printed below the island. Not part of
// the app bundle.

import "../src/style.css";
import { Bridge } from "../src/core/bridge";
import { State, type AskQuestion } from "../src/core/state";
import { Island } from "../src/island/island";
import { parseQuestions } from "../src/island/hooks";

const log = document.getElementById("log")!;

const SETS: Record<string, unknown[]> = {
  three: [
    {
      question: "Qual banco de dados devemos usar neste projeto novo?",
      header: "Banco",
      multiSelect: false,
      options: [
        { label: "Postgres", description: "Relacional, robusto, bom para produção" },
        { label: "SQLite", description: "Arquivo local, zero configuração" },
        { label: "MySQL", description: "Relacional, muito difundido" },
        { label: "MongoDB", description: "Documentos JSON, esquema flexível" },
      ],
    },
    {
      question: "Quais recursos incluir na primeira versão?",
      header: "Recursos",
      multiSelect: true,
      options: [
        { label: "Login", description: "E-mail e senha" },
        { label: "Pagamentos", description: "Stripe Checkout" },
        { label: "Notificações", description: "" },
      ],
    },
    {
      question: "Pode publicar?",
      header: "Deploy",
      multiSelect: false,
      options: [
        { label: "Sim", description: "" },
        { label: "Ainda não", description: "" },
      ],
    },
  ],
  single: [
    {
      question: "Qual cor?",
      header: "Cor",
      multiSelect: false,
      options: [{ label: "Azul", description: "" }, { label: "Verde", description: "" }],
    },
  ],
};

// Bridge calls are no-ops outside Tauri; show what would have been sent.
Bridge.questionAnswer = async (id, answers) => {
  log.textContent = `questionAnswer(${id}) →\n${JSON.stringify(answers, null, 2)}`;
  return null;
};
Bridge.approvalDecision = async (id, d) => {
  log.textContent = `approvalDecision(${id}, ${d})`;
  return null;
};
Bridge.approvalDecline = async (id) => {
  log.textContent = `approvalDecline(${id}) — terminal`;
  return null;
};

const island = new Island(document.getElementById("root")!);
island.applySettings();
State.loadIntegrationTasks();


let n = 0;
function show(set: string) {
  log.textContent = "";
  const id = `preview-${++n}`;
  State.pendingQuestion = null;
  State.pendingApproval = null;
  if (set === "approval") {
    State.pendingApproval = { requestId: id, sessionId: "s", tool: "Bash", command: "Bash · npm run deploy" };
  } else {
    const questions = parseQuestions({ questions: SETS[set] }) as AskQuestion[];
    State.pendingQuestion = {
      requestId: id, sessionId: "s", questions, index: 0,
      picked: questions.map(() => []), custom: questions.map(() => null), typing: false,
    };
  }
  State.isPinned = true;
  island.alert(set === "approval" ? "approval" : "question");
}

for (const b of document.querySelectorAll<HTMLButtonElement>("#controls button")) {
  b.addEventListener("click", () => show(b.dataset.set!));
}
window.setTimeout(() => show(new URLSearchParams(location.search).get("set") ?? "three"), 300);
