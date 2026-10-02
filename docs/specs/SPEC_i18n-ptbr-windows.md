# SPEC — Tradução pt-BR do app Windows

## Objetivo
Exibir a interface do Coucou para Windows (`windows/`) em português do Brasil quando o idioma do sistema for português, mantendo o inglês como padrão para os demais idiomas. Sem seletor nos Ajustes: o idioma segue o Windows.

Fora do escopo: app macOS (`NotchBuddy/`), site `docs/*.html`, README, documentação interna.

## Regras de escolha do idioma
| Idioma do sistema | Interface |
|---|---|
| `pt-BR`, `pt-PT`, `pt` (qualquer variante `pt*`) | pt-BR |
| qualquer outro | en |

- Front-end (ilha + janela de Ajustes): `navigator.languages[0]` (o WebView2 herda o idioma do Windows).
- Rust (menu da bandeja, título da janela de Ajustes, mensagens de erro devolvidas ao front): idioma de UI do Windows via `GetUserDefaultUILanguage` / `GetUserDefaultLocaleName`.
- O idioma é lido na inicialização. Mudar o idioma do Windows exige reiniciar o Coucou (aceitável).

## Modelo de dados
- `windows/src/i18n/en.ts` — dicionário de referência (`as const`); define o tipo das chaves.
- `windows/src/i18n/pt-BR.ts` — `Record<keyof typeof en, string>`: o `tsc` falha se faltar uma chave.
- Strings com variáveis usam placeholders `{nome}`: `"Uploading {name}"` → `"Enviando {name}"`.
- Plural simples via chaves `_one` / `_other` quando houver contagem.
- Rust: `match` simples `Lang::En | Lang::PtBr` num módulo `i18n.rs` (poucas strings, sem crate novo).

## O que é traduzido
- Views da ilha: `views/views.ts`, `views/integrations.ts`, `views/upload.ts`, `views/chat.ts`, `views/ticker.ts`.
- Textos desenhados em canvas: `upload/canvas.ts` ("Drop your files here", "Uploading …", botões).
- Rótulos de passos das ferramentas do Claude Code em `island/hooks.ts` — **hoje estão em francês** ("Exécute", "Lit"…); passam a vir do dicionário (en: "Runs", "Reads"…; pt-BR: "Executa", "Lê"…).
- Janela de Ajustes: `settings/main.ts`, `settings.html` ("Loading…", título).
- Tempo relativo em `views/integrations.ts` ("just now", "5 min ago"…).
- Datas/horas: já usam `toLocale*String(undefined)` → passam a receber o locale escolhido para ficar coerente com o texto.
- Rust: `tray.rs` (Open Coucou, Settings…, Pause, Quit), título `Settings — Coucou` em `lib.rs`, mensagens de `hooks.rs` exibidas ao usuário ("No change.", "Can't read …").
- `<html lang>` de `index.html` e `settings.html` definido em runtime.

## O que NÃO é traduzido
- Nomes próprios: Claude Code, VS Code, n8n, Stripe, GitHub, Vercel, Coucou, Mochi, `settings.json`.
- Logs, comentários, chaves de eventos IPC, ids.
- O chat: o `SYSTEM_PROMPT` já instrui "Respond in the user's language" — nada a mudar.
- Textos vindos de APIs externas (nomes de workflows, títulos de reuniões, erros devolvidos por serviços).

## Impactos
- **Layout:** pt-BR é ~20–30 % mais longo. A ilha tem larguras fixas (`core/layout.ts`) e o canvas de upload posiciona texto em coordenadas absolutas (botão centralizado em x=198). Cada tela precisa ser conferida; traduções podem ser encurtadas para caber.
- **Detecção de rate limit** em `island/hooks.ts:188` compara texto (`"rate limit"`, `"limite d"`) — é texto de entrada do Claude Code, não da UI; não mexer.
- Nenhuma mudança em hooks, socket, Keychain/Credential Manager, settings persistidos.
- Performance: dicionário carregado uma vez; zero custo com a ilha escondida.

## Casos de borda
- Chave ausente em pt-BR → impossível em build (tipo), mas `t()` cai no inglês por segurança.
- Placeholder sem valor → mantém `{nome}` literal (visível em teste, não quebra).
- `navigator.languages` vazio → inglês.
- Modo `npm run dev` no navegador → usa o idioma do navegador (útil para testar: `?lang=pt-BR` força o idioma, só em dev).
- Textos em `title`/tooltip e `placeholder` de inputs também entram.
