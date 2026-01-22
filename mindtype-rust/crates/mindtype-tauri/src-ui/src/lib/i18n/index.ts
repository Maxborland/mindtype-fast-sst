import { writable, derived, get } from 'svelte/store';

// Import all translation files
import en from './en.json';
import ru from './ru.json';
import es from './es.json';
import de from './de.json';
import fr from './fr.json';
import zh from './zh.json';

// Supported locales
export const SUPPORTED_LOCALES = ['en', 'ru', 'es', 'de', 'fr', 'zh'] as const;
export type Locale = typeof SUPPORTED_LOCALES[number];

// Locale display names
export const LOCALE_NAMES: Record<Locale, string> = {
  en: 'English',
  ru: 'Русский',
  es: 'Español',
  de: 'Deutsch',
  fr: 'Français',
  zh: '中文'
};

// Translation dictionaries
type TranslationDict = Record<string, string>;
const translations: Record<Locale, TranslationDict> = {
  en,
  ru,
  es,
  de,
  fr,
  zh
};

// Detect browser language and map to supported locale
function detectBrowserLocale(): Locale {
  if (typeof navigator === 'undefined') return 'en';

  const browserLang = navigator.language.split('-')[0];
  if (SUPPORTED_LOCALES.includes(browserLang as Locale)) {
    return browserLang as Locale;
  }
  return 'en';
}

// Load saved locale from localStorage
function loadSavedLocale(): Locale {
  if (typeof localStorage === 'undefined') return detectBrowserLocale();

  const saved = localStorage.getItem('mindtype-locale');
  if (saved && SUPPORTED_LOCALES.includes(saved as Locale)) {
    return saved as Locale;
  }
  return detectBrowserLocale();
}

// Current locale store
export const locale = writable<Locale>(loadSavedLocale());

// Subscribe to locale changes and save to localStorage
locale.subscribe((value) => {
  if (typeof localStorage !== 'undefined') {
    localStorage.setItem('mindtype-locale', value);
  }
});

// Translation function store
export const t = derived(locale, ($locale) => {
  return (key: string, params?: Record<string, string | number>): string => {
    let text = translations[$locale]?.[key] || translations['en']?.[key] || key;

    // Simple parameter substitution: {param} -> value
    if (params) {
      for (const [param, value] of Object.entries(params)) {
        text = text.replace(new RegExp(`\\{${param}\\}`, 'g'), String(value));
      }
    }

    return text;
  };
});

// Helper function for use outside of Svelte components
export function translate(key: string, params?: Record<string, string | number>): string {
  const $locale = get(locale);
  let text = translations[$locale]?.[key] || translations['en']?.[key] || key;

  if (params) {
    for (const [param, value] of Object.entries(params)) {
      text = text.replace(new RegExp(`\\{${param}\\}`, 'g'), String(value));
    }
  }

  return text;
}

// Set locale function
export function setLocale(newLocale: Locale): void {
  if (SUPPORTED_LOCALES.includes(newLocale)) {
    locale.set(newLocale);
  }
}

// Get all translations for current locale (useful for debugging)
export function getAllTranslations(): TranslationDict {
  return translations[get(locale)] || translations['en'];
}
