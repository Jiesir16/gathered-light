import { useEffect, useState } from "react";
import type { Lang } from "../types";
import { i18n } from "../i18n";

const langKey = "gathered_light_lang";

export function useLang() {
  const [lang, setLangState] = useState<Lang>(() => (localStorage.getItem(langKey) as Lang) || "zh");

  useEffect(() => {
    document.documentElement.lang = lang;
    localStorage.setItem(langKey, lang);
  }, [lang]);

  return {
    lang,
    setLang: setLangState,
    t: i18n[lang]
  };
}
