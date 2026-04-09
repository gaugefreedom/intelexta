import i18n from "i18next";
import { initReactI18next } from "react-i18next";
import LanguageDetector from "i18next-browser-languagedetector";

import enCommon from "./locales/en/common.json";
import ptBRCommon from "./locales/pt-BR/common.json";

i18n
  .use(LanguageDetector)
  .use(initReactI18next)
  .init({
    fallbackLng: "en",
    supportedLngs: ["en", "pt-BR"],
    load: "currentOnly",
    ns: ["common"],
    defaultNS: "common",
    detection: {
      order: ["querystring", "localStorage"],
      caches: ["localStorage"],
      lookupQuerystring: "lang",
      lookupLocalStorage: "intelexta_locale",
      convertDetectedLanguage: (lng: string) => {
        if (lng === "pt" || lng.startsWith("pt-")) return "pt-BR";
        return lng;
      },
    },
    resources: {
      en: { common: enCommon },
      "pt-BR": { common: ptBRCommon },
    },
    interpolation: {
      escapeValue: false,
    },
  });

export default i18n;
