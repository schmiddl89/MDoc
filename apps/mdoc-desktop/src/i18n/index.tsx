import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useState,
  type ReactNode,
} from 'react'
import { invoke, isTauri } from '@tauri-apps/api/core'
import HELP_EN from '../assets/Help.md?raw'
import HELP_DE from '../assets/Hilfe.md?raw'
import { de } from './de'
import { en, type MessageKey } from './en'

export type Locale = 'de' | 'en'

const STORAGE_KEY = 'mdoc.uiLanguage'
const dicts: Record<Locale, Record<MessageKey, string>> = { de, en }

export function guideMarkdown(locale: Locale): string {
  return locale === 'de' ? HELP_DE : HELP_EN
}

export function detectSystemLocale(): Locale {
  try {
    const lang = (navigator.language || 'en').toLowerCase()
    return lang.startsWith('de') ? 'de' : 'en'
  } catch {
    return 'en'
  }
}

export function readStoredLocale(): Locale | null {
  try {
    const raw = window.localStorage.getItem(STORAGE_KEY)?.trim().toLowerCase()
    if (raw === 'de' || raw === 'en') return raw
  } catch {
    /* ignore */
  }
  return null
}

function writeStoredLocale(locale: Locale) {
  try {
    window.localStorage.setItem(STORAGE_KEY, locale)
  } catch {
    /* ignore */
  }
}

export function formatMessage(
  locale: Locale,
  key: MessageKey,
  params?: Record<string, string | number>,
): string {
  let text = dicts[locale][key] ?? en[key] ?? String(key)
  if (params) {
    for (const [name, value] of Object.entries(params)) {
      text = text.replaceAll(`{${name}}`, String(value))
    }
  }
  return text
}

type LocaleContextValue = {
  locale: Locale
  ready: boolean
  setLocale: (locale: Locale) => Promise<void>
  t: (key: MessageKey, params?: Record<string, string | number>) => string
  guide: string
}

const LocaleContext = createContext<LocaleContextValue | null>(null)

export function LocaleProvider({ children }: { children: ReactNode }) {
  const [locale, setLocaleState] = useState<Locale>(
    () => readStoredLocale() ?? detectSystemLocale(),
  )
  const [ready, setReady] = useState(!isTauri())

  useEffect(() => {
    if (!isTauri()) {
      setReady(true)
      return
    }
    let cancelled = false
    void invoke<string | null>('get_ui_language')
      .then((value) => {
        if (cancelled) return
        const normalized = value?.trim().toLowerCase()
        if (normalized === 'de' || normalized === 'en') {
          setLocaleState(normalized)
          writeStoredLocale(normalized)
        } else {
          const fallback = readStoredLocale() ?? detectSystemLocale()
          setLocaleState(fallback)
          writeStoredLocale(fallback)
          void invoke('set_ui_language', { lang: fallback }).catch(() => {
            /* optional */
          })
        }
      })
      .catch(() => {
        /* keep local/system default */
      })
      .finally(() => {
        if (!cancelled) setReady(true)
      })
    return () => {
      cancelled = true
    }
  }, [])

  const setLocale = useCallback(async (next: Locale) => {
    setLocaleState(next)
    writeStoredLocale(next)
    if (isTauri()) {
      try {
        await invoke('set_ui_language', { lang: next })
      } catch {
        /* optional */
      }
    }
  }, [])

  const t = useCallback(
    (key: MessageKey, params?: Record<string, string | number>) =>
      formatMessage(locale, key, params),
    [locale],
  )

  const value = useMemo<LocaleContextValue>(
    () => ({
      locale,
      ready,
      setLocale,
      t,
      guide: guideMarkdown(locale),
    }),
    [locale, ready, setLocale, t],
  )

  return <LocaleContext.Provider value={value}>{children}</LocaleContext.Provider>
}

export function useLocale(): LocaleContextValue {
  const ctx = useContext(LocaleContext)
  if (!ctx) {
    throw new Error('useLocale must be used within LocaleProvider')
  }
  return ctx
}
