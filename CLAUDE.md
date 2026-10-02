# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## 1. Regras de Contexto e Eficiência (Anti-Token Drain)

- **Proibição de Skills Externas:** É estritamente **proibido** utilizar ou invocar frameworks de agentes, plugins ou skills de terceiros (como `Superpowers`, `Composio`, etc.). Toda a operação deve ser estritamente nativa. (Os arquivos em `docs/superpowers/` são documentos escritos à mão — é só o nome da pasta, não autoriza usar o framework.)
- **Uso do Graphify:** Se `graphify-out/graph.json` existir, prefira `graphify query "<pergunta>"`, `graphify path "<A>" "<B>"` e `graphify explain "<conceito>"` antes de ler código bruto, e rode `graphify update .` após modificar o código. Hoje o diretório **não existe** neste repositório — ignore até que seja gerado.
- **Tratamento de Caminhos (Windows):** O caminho do projeto contém acento (`...\Kauã\...`) e espaços — sempre entre aspas. Use caminhos absolutos ou a sintaxe `@caminho/do/arquivo`. Nunca encadeie `cd` com operadores de escrita no terminal.
- **RTK:** prefixe comandos com `rtk` (`rtk npm test`, `rtk git status`, `rtk grep <pattern>`). Se não houver filtro dedicado, passa direto.

## 2. Processo de Trabalho Obrigatório

### Cenário A: Ajustes rápidos, bugs simples ou testes falhando
1. Alteração cirúrgica direta no arquivo indicado pelo usuário com a tag `@`.
2. Rodar `rtk npm test` (ou o teste focado) imediatamente.

### Cenário B: Novas funcionalidades ou mudanças de impacto amplo
Antes de escrever código de implementação:
1. **Plan Mode.**
2. `docs/specs/SPEC_{tarefa}.md` — objetivo de negócio, modelo de dados, fluxo, impactos, casos de borda.
3. `docs/plans/PLANO_{tarefa}.md` — estratégia técnica, ordem das camadas (model → controller → IPC → renderer), padrões e tratamento de erro.
4. `docs/checklist/TODO_{tarefa}.md` — checklist acionável `[ ]`, quebrado por arquivo e etapa incremental.
5. **Pare.** Não escreva código no projeto. Resuma e aguarde autorização explícita.

# Coucou — guide for AI coding agents

Coucou is a native macOS app: Mochi, a small animated character living in the MacBook notch, shows Claude Code sessions and a few integrations, and lets the user approve, answer, chat and drop files from the notch.

## Where things are
- `NotchBuddy/Sources/App/` — all Swift code. `NotchBuddy/Resources/sounds/` — the 28 WAV sounds. `NotchBuddy/project.yml` — XcodeGen project (never edit the `.xcodeproj` by hand).
- `docs/SPEC.md`, `docs/INTEGRATIONS.md` — behaviour, views, states, integrations (in French).
- `design/prototype/notch-buddy.html` — original prototype, the visual source of truth. `design/captures/` — target screenshots.
- `docs/*.html` — the GitHub Pages site (privacy, terms, support, legal notice).

## Build
```
cd NotchBuddy && xcodegen && xcodebuild -scheme NotchBuddy -configuration Debug build
```

## Rules
- Swift 6, SwiftUI + AppKit. No third-party dependencies unless truly unavoidable. The character is drawn in code (`Canvas` + `TimelineView`), no Rive/Lottie/images.
- Secrets live in the Keychain, never on disk or in git.
- No telemetry. Network calls only to services the user configured.
- Never block Claude Code: if the app doesn't answer, the hook exits immediately.
- Never overwrite `~/.claude/settings.json`: dated backup, merge, show the diff, write only after the user confirms.
- Never send an email or approve a Claude Code permission without an explicit click.
- Performance: 0 % CPU when the island is hidden.
- Keep the bundle identifier `fr.louisraille.NotchBuddy` (Keychain items, preferences and permissions depend on it).
- Visual changes must match the prototype and the screenshots in `design/captures/`.
