import { useEffect, useId, useRef, useState, type ReactNode } from 'react'
import { useLocale, type Locale } from './i18n'

export type RibbonPlacement = 'home' | 'ownTab'

interface AppMenuProps {
  busy: boolean
  runningInTauri: boolean
  ribbonPlacement: RibbonPlacement
  onHelp: () => void
  onLanguageChange: (locale: Locale) => void
  onRibbonChange: (placement: RibbonPlacement) => void
}

export function AppMenu({
  busy,
  runningInTauri,
  ribbonPlacement,
  onHelp,
  onLanguageChange,
  onRibbonChange,
}: AppMenuProps) {
  const { locale, t } = useLocale()
  const [open, setOpen] = useState(false)
  const rootRef = useRef<HTMLDivElement | null>(null)
  const menuId = useId()

  useEffect(() => {
    if (!open) return
    const onPointerDown = (event: PointerEvent) => {
      if (!rootRef.current?.contains(event.target as Node)) {
        setOpen(false)
      }
    }
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape') setOpen(false)
    }
    window.addEventListener('pointerdown', onPointerDown)
    window.addEventListener('keydown', onKeyDown)
    return () => {
      window.removeEventListener('pointerdown', onPointerDown)
      window.removeEventListener('keydown', onKeyDown)
    }
  }, [open])

  return (
    <div className="app-menu" ref={rootRef}>
      <button
        type="button"
        className={open ? 'app-menu-trigger is-open' : 'app-menu-trigger'}
        aria-label={t('menuLabel')}
        aria-haspopup="menu"
        aria-expanded={open}
        aria-controls={menuId}
        disabled={busy}
        title={t('menuLabel')}
        onClick={() => setOpen((value) => !value)}
      >
        <span className="app-menu-icon" aria-hidden="true">
          <span />
          <span />
          <span />
        </span>
      </button>

      {open ? (
        <div className="app-menu-panel" id={menuId} role="menu">
          <button
            type="button"
            role="menuitem"
            className="app-menu-item"
            onClick={() => {
              setOpen(false)
              onHelp()
            }}
          >
            {t('help')}
          </button>

          <MenuSection title={t('langLabel')} hint={t('langTooltip')}>
            <div className="pref-switch pref-switch-menu" role="group" aria-label={t('langLabel')}>
              <button
                type="button"
                className={locale === 'de' ? 'pref-option is-active' : 'pref-option'}
                aria-pressed={locale === 'de'}
                onClick={() => void onLanguageChange('de')}
              >
                DE
              </button>
              <button
                type="button"
                className={locale === 'en' ? 'pref-option is-active' : 'pref-option'}
                aria-pressed={locale === 'en'}
                onClick={() => void onLanguageChange('en')}
              >
                EN
              </button>
            </div>
          </MenuSection>

          <MenuSection title={t('ribbonLabel')} hint={t('ribbonTooltip')}>
            <div
              className="pref-switch pref-switch-menu"
              role="group"
              aria-label={t('ribbonLabel')}
            >
              <button
                type="button"
                className={ribbonPlacement === 'home' ? 'pref-option is-active' : 'pref-option'}
                aria-pressed={ribbonPlacement === 'home'}
                disabled={!runningInTauri}
                onClick={() => void onRibbonChange('home')}
              >
                {t('ribbonHome')}
              </button>
              <button
                type="button"
                className={ribbonPlacement === 'ownTab' ? 'pref-option is-active' : 'pref-option'}
                aria-pressed={ribbonPlacement === 'ownTab'}
                disabled={!runningInTauri}
                onClick={() => void onRibbonChange('ownTab')}
              >
                {t('ribbonOwnTab')}
              </button>
            </div>
          </MenuSection>
        </div>
      ) : null}
    </div>
  )
}

function MenuSection({
  title,
  hint,
  children,
}: {
  title: string
  hint: string
  children: ReactNode
}) {
  return (
    <div className="app-menu-section" title={hint}>
      <div className="app-menu-section-title">{title}</div>
      {children}
    </div>
  )
}
