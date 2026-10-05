# PLANO — Tradução pt-BR do app Windows

Referência: `docs/specs/SPEC_i18n-ptbr-windows.md`.

## Estratégia
Sem biblioteca de i18n (regra do projeto: sem dependências evitáveis). Um módulo pequeno em TypeScript e um enum em Rust.

### Camada 1 — núcleo i18n (front)
`windows/src/i18n/index.ts`:
```ts
import { en } from "./en";
import { ptBR } from "./pt-BR";

export type Key = keyof typeof en;
export type Lang = "en" | "pt-BR";

function detect(): Lang {
  if (import.meta.env.DEV) {
    const forced = new URLSearchParams(location.search).get("lang");
    if (forced === "pt-BR" || forced === "en") return forced;
  }
  const first = navigator.languages?.[0] ?? navigator.language ?? "en";
  return first.toLowerCase().startsWith("pt") ? "pt-BR" : "en";
}

export const LANG: Lang = detect();
const DICT: Record<Key, string> = LANG === "pt-BR" ? ptBR : en;

export function t(key: Key, vars?: Record<string, string | number>): string {
  const s = DICT[key] ?? en[key];
  return vars ? s.replace(/\{(\w+)\}/g, (m, k) => (k in vars ? String(vars[k]) : m)) : s;
}
```
- `en.ts` exporta `export const en = { ... } as const;`
- `pt-BR.ts` exporta `export const ptBR: Record<keyof typeof en, string> = { ... };`
- Chaves agrupadas por prefixo de tela: `overview.*`, `permission.*`, `upload.*`, `chat.*`, `integrations.*`, `settings.*`, `tool.*`, `time.*`.
- `main.ts` e `settings/main.ts` fazem `document.documentElement.lang = LANG` no boot.
- Datas: `toLocaleDateString(LANG, …)` no lugar de `undefined`.

### Camada 2 — substituição nas telas (front)
Ordem, do menor risco para o maior:
1. `views/chat.ts`, `views/ticker.ts`, `views/upload.ts`
2. `views/views.ts`
3. `views/integrations.ts` (inclui tempo relativo)
4. `island/hooks.ts` (`TOOL_LABELS` francês → chaves `tool.*`)
5. `upload/canvas.ts` (texto em canvas: conferir medidas)
6. `settings/main.ts` + `settings.html`

Padrão: trocar só o literal, sem refatorar a view. `h("div", { text: "Workflow stopped." })` → `h("div", { text: t("error.workflowStopped") })`.
Strings definidas em constantes de módulo (ex.: `TOOL_LABELS`, `MODELS`, campos de integração) — como `LANG` é resolvido no import e não muda em runtime, podem continuar sendo constantes que chamam `t()`.

### Camada 3 — Rust
`windows/src-tauri/src/i18n.rs`:
- `pub enum Lang { En, PtBr }` + `pub fn lang() -> Lang` (cacheado em `OnceLock`), usando `GetUserDefaultLocaleName` (feature `Win32_Globalization` no crate `windows` já existente — sem dependência nova).
- `pub fn tr(key: &'static str) -> &'static str` com `match (lang(), key)`.
- Aplicar em `tray.rs`, título da janela em `lib.rs:331`, mensagens de usuário em `hooks.rs`.

### Camada 4 — verificação
Não há testes automatizados no app Windows. Verificação:
1. `rtk npm run build` em `windows/` (o `tsc --noEmit` pega chaves faltando/erradas).
2. `cargo build -p` do app Tauri.
3. `npm run dev` com `?lang=pt-BR` e `?lang=en`: passar por todas as views (overview, idle, permission, question, error, finished, overload, upload, chat, integrações, ajustes) e conferir corte/overflow.
4. Teste real no Windows com idioma pt-BR e en.
5. Grep final por literais em inglês remanescentes nas pastas tocadas.

## Tratamento de erro
- `t()` nunca lança: chave ausente → inglês → a própria chave.
- Rust: falha ao ler o locale → `Lang::En`.
- Mensagens de erro vindas do Rust que hoje são concatenadas (`Could not save: ${err}`) mantêm o `err` original; só o prefixo é traduzido.

## Riscos
- Overflow em larguras fixas → encurtar a tradução antes de mexer no layout; mudanças de layout só se inevitáveis e conferidas contra `design/captures/`.
- Strings esquecidas → grep final + passagem visual tela a tela.
