import { createContext, useContext, useMemo, type ReactNode } from "react";

import { en } from "@/i18n/locales/en";
import { vi } from "@/i18n/locales/vi";
import type { AppLanguage, TranslationKey } from "@/i18n/types";

type I18nContextValue = {
  language: AppLanguage;
  setLanguage: (language: AppLanguage) => void;
  t: (key: TranslationKey) => string;
};

const dictionaries = { en, vi };
const I18nContext = createContext<I18nContextValue | null>(null);

export function I18nProvider({
  children,
  language,
  setLanguage,
}: {
  children: ReactNode;
  language: AppLanguage;
  setLanguage: (language: AppLanguage) => void;
}) {
  const value = useMemo<I18nContextValue>(() => {
    const dictionary = dictionaries[language] ?? en;
    return {
      language,
      setLanguage,
      t: (key) => dictionary[key] ?? en[key] ?? key,
    };
  }, [language, setLanguage]);

  return <I18nContext.Provider value={value}>{children}</I18nContext.Provider>;
}

export function useI18n() {
  const context = useContext(I18nContext);
  if (!context) {
    return {
      language: "en" as AppLanguage,
      setLanguage: () => {},
      t: (key: TranslationKey) => en[key] ?? key,
    };
  }
  return context;
}

export function normalizeLanguage(value: unknown): AppLanguage {
  return value === "vi" ? "vi" : "en";
}
