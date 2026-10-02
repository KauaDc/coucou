// Interface language. Follows Windows (WebView2 inherits the system language):
// any Portuguese → pt-BR, everything else → English. Read once at startup.

import { en } from "./en";
import { ptBR } from "./pt-BR";

export type Key = keyof typeof en;
export type Lang = "en" | "pt-BR";

function detect(): Lang {
  // `npm run dev` in a browser: ?lang=pt-BR / ?lang=en to check both.
  if (import.meta.env.DEV) {
    const forced = new URLSearchParams(location.search).get("lang");
    if (forced === "pt-BR" || forced === "en") return forced;
  }
  const first = navigator.languages?.[0] ?? navigator.language ?? "en";
  return first.toLowerCase().startsWith("pt") ? "pt-BR" : "en";
}

export const LANG: Lang = detect();

const DICT: Record<Key, string> = LANG === "pt-BR" ? ptBR : en;

/** The string for `key`, with `{name}` placeholders filled from `vars`. Never throws. */
export function t(key: Key, vars?: Record<string, string | number>): string {
  const s = DICT[key] ?? en[key] ?? key;
  if (!vars) return s;
  return s.replace(/\{(\w+)\}/g, (m, k: string) => (k in vars ? String(vars[k]) : m));
}

/** Tags the document with the interface language (and an optional title key). */
export function applyDocumentLang(titleKey?: Key) {
  document.documentElement.lang = LANG;
  if (titleKey) document.title = t(titleKey);
}
