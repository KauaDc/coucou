// Português (Brasil). Must define every key of en.ts — the type enforces it.

import type { en } from "./en";

export const ptBR: Record<keyof typeof en, string> = {
  // Island chrome
  "tab.overview": "Visão geral",
  "tab.ask": "Perguntar",
  "tab.drop": "Soltar",
  "tab.settings": "Ajustes",
  "tab.mute": "Silenciar",

  // Overview
  "overview.open": "Abrir",

  // Empty
  "empty.title": "Nada rodando agora.",
  "empty.sub": "Solte um arquivo ou janela, ou me pergunte algo.",
  "empty.ask": "Perguntar ao {provider}",

  // Approval
  "approval.who": "pede permissão",
  "approval.deny": "Negar",
  "approval.allow": "Permitir",

  // Question
  "question.who": "O Claude Code tem uma pergunta",
  "question.fallback": "O Claude precisa de uma resposta.",
  "question.sub": "Responda no terminal — o Coucou ainda não responde por você.",

  // Error
  "error.workflowStopped": "Workflow parado.",
  "error.sessionStopped": "Sessão parada por um erro.",
  "error.noDetail": "Sem detalhes disponíveis.",
  "error.retry": "Tentar de novo",
  "error.openN8n": "Abrir no n8n",

  // Finished
  "finished.who": "Claude Code terminou",
  "finished.fallback": "Sessão concluída",
  "finished.openTerminal": "Abrir terminal",
  "finished.ok": "OK",

  // Confused
  "confused.title": "Muitas cutucadas de uma vez.",
  "confused.sub": "Só um instante — volto ao trabalho em três segundos.",

  // In-island settings
  "islandSettings.sound": "Som",
  "islandSettings.autoClose": "Fechar sozinho · {s}s",
  "islandSettings.more": "Ajustes…",

  // Placeholders
  "placeholder.mail": "Enviar por e-mail não está nesta versão.",
  "placeholder.searching": "O {provider} está pesquisando…",
  "placeholder.result": "Resultado",

  // Chat
  "chat.placeholder": "Pergunte qualquer coisa…",
  "chat.continue": "Continuar…",
  "chat.send": "Enviar",

  // Upload
  "upload.dropHere": "Solte seus arquivos aqui",
  "upload.tag.pdf": "PDF",
  "upload.tag.images": "Imagens",
  "upload.tag.code": "Código",
  "upload.tag.docs": "Docs",
  "upload.uploading": "Enviando {name}",
  "upload.file": "arquivo",
  "upload.fileCap": "Arquivo",
  "upload.isReady": " está pronto.",
  "upload.isReadyFull": "{name} está pronto.",
  "upload.whatToDo": "O que você quer fazer com ele?",
  "upload.ask": "Fazer uma pergunta",
  "upload.askAbout": "Perguntar sobre ele",
  "upload.cancel": "Cancelar",

  // Integrations
  "int.justNow": "agora",
  "int.ago": "há {t}",
  "int.hooksMissing": "Hooks não instalados",
  "int.keyMissing": "Chave não configurada",
  "int.loading": "Conectado · carregando…",
  "int.openVSCode": "Abrir o Visual Studio Code",
  "int.openN8n": "Abrir o n8n",
  "int.open": "Abrir {name}",
  "int.refresh": "Atualizar",
  "int.settings": "Ajustes…",
  "int.kind.integration": "Integração",
  "int.kind.deployments": "Deploys",
  "int.kind.emails": "E-mails",
  "int.kind.overview": "Visão geral",
  "int.kind.payments": "Pagamentos",
  "int.kind.recent": "Recentes",
  "int.kind.schedule": "Agenda",
  "int.kind.workflow": "Workflow",
  "int.details": "Detalhes",
  "int.ready": "Pronto",
  "int.canceled": "Cancelado",
  "int.error": "Erro",
  "int.deployment": "Deploy",
  "int.totalStars": "Estrelas",
  "int.repositories": "Repositórios",
  "int.payment": "Pagamento",
  "int.untitled": "Sem título",
  "int.noCalls": "Nenhuma reunião marcada",
  "int.meeting": "Reunião",
  "int.workflow": "Workflow",
  "int.success": "Sucesso",
  "int.failed": "Falhou",
  "int.completed": "Concluído com sucesso.",
  "int.noErrorDetail": "Sem detalhes do erro.",

  // Claude Code steps (ticker)
  "tool.Bash": "Executa",
  "tool.Read": "Lê",
  "tool.Write": "Escreve",
  "tool.Edit": "Edita",
  "tool.Glob": "Procura",
  "tool.Grep": "Busca",
  "tool.WebSearch": "Pesquisa web",
  "tool.WebFetch": "Baixa",
  "tool.TodoWrite": "Tarefas",
  "tool.Task": "Agente",
  "tool.LS": "Lista",
  "tool.MultiEdit": "Edita",
  "tool.NotebookEdit": "Notebook",
  "tool.PowerShell": "Executa",
  "step.session": "Sessão",
  "step.tool": "Ferramenta",
  "step.failed": "⚠ falhou",
  "step.subagent": "+ subagente",
  "step.subagentDone": "• subagente concluído",

  // Settings window
  "settings.windowTitle": "Ajustes — Coucou",
  "settings.loading": "Carregando…",
  "settings.hooks.installed":
    "O Coucou está conectado às suas sessões do Claude Code. Chamadas de ferramentas, perguntas e pedidos de permissão aparecem na ilha, e você pode respondê-los por lá.",
  "settings.hooks.notInstalled":
    "Instale os hooks para ver suas sessões do Claude Code na ilha e aprovar permissões sem largar o que está fazendo.",
  "settings.hooks.relay": "Relay",
  "settings.hooks.relayMissing":
    "O coucou-hook.exe ainda não está no lugar. Reinicie o Coucou; se continuar falhando, compile com `cargo build -p coucou-hook`.",
  "settings.hooks.reinstall": "Reinstalar hooks…",
  "settings.hooks.install": "Instalar hooks…",
  "settings.hooks.relayNotInstalled": "O relay ainda não está instalado.",
  "settings.hooks.uninstall": "Desinstalar hooks…",
  "settings.back": "Voltar",
  "settings.hooks.previewInstall":
    "Isto é exatamente o que vai mudar no seu settings.json. Seus próprios hooks não são tocados.",
  "settings.hooks.previewRemove": "Isto remove só as entradas do Coucou. Seus próprios hooks não são tocados.",
  "settings.hooks.backup": "Backup → {path}",
  "settings.hooks.write": "Fazer backup e gravar",
  "settings.hooks.remove": "Fazer backup e remover",
  "settings.hooks.done":
    "Pronto. Ajustes anteriores salvos como {backup}. Abra uma nova sessão do Claude Code para carregar os hooks.",
  "settings.couldNotWrite": "Não foi possível gravar: {err}",
  "settings.cancel": "Cancelar",
  "settings.api.saved": "Chave salva no Gerenciador de Credenciais do Windows.",
  "settings.api.none": "Nenhuma chave ainda — o chat precisa de uma.",
  "settings.api.stored": "(salva)",
  "settings.api.save": "Salvar chave",
  "settings.api.remove": "Remover",
  "settings.api.savedOk": "Salva. Ela nunca vai para o disco.",
  "settings.couldNotSave": "Não foi possível salvar: {err}",
  "settings.api.removed": "Chave removida.",
  "settings.couldNotRemove": "Não foi possível remover: {err}",
  "settings.chat.title": "Chat",
  "settings.chat.provider": "Provedor",
  "settings.api.key": "Chave da API",
  "settings.api.model": "Modelo",
  "settings.int.title": "Integrações",
  "settings.int.note":
    "Escolha até {max} pílulas para mostrar ao lado do Mochi — {used}/{max} em uso. As chaves ficam no Gerenciador de Credenciais do Windows, nunca no disco.",
  "settings.int.secretKey": "Chave secreta",
  "settings.int.token": "Token",
  "settings.int.instanceUrl": "URL da instância",
  "settings.int.integrationToken": "Token da integração",
  "settings.save": "Salvar",
  "settings.general.title": "Geral",
  "settings.general.sound": "Som",
  "settings.general.autoClose": "Fechar sozinho",
  "settings.general.autoCloseHint": "segundos depois que você sai da ilha",
  "settings.general.screen": "Tela da ilha",
  "settings.general.screenMain": "Tela principal",
  "settings.general.screenCursor": "Tela sob o cursor",
  "settings.general.autostart": "Abrir ao iniciar o Windows",
  "settings.privacy": "Sem telemetria. Requisições de rede só vão para os serviços que você mesmo configura.",
};
