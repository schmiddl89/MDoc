import { useEffect, useMemo, useState } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { open, save } from '@tauri-apps/plugin-dialog'
import { useLocale } from './i18n'
import type { MessageKey } from './i18n/en'

export interface TemplateStyle {
  id: string
  name: string
  kind: 'paragraph' | 'character' | 'table' | 'numbering' | 'other'
}

export interface StyleProfile {
  name: string
  paragraphs: {
    normal?: string | null
    heading1?: string | null
    heading2?: string | null
    heading3?: string | null
    heading4?: string | null
    heading5?: string | null
    heading6?: string | null
    blockQuote?: string | null
    codeBlock?: string | null
    unorderedList?: string | null
    orderedList?: string | null
    table?: string | null
  }
  characters: {
    strong?: string | null
    emphasis?: string | null
    strongEmphasis?: string | null
    strikethrough?: string | null
    inlineCode?: string | null
    link?: string | null
  }
}

type RoleKind = 'paragraph' | 'character'
type Translate = (key: MessageKey, params?: Record<string, string | number>) => string

interface RoleDef {
  key: string
  label: string
  hint?: string
  kind: RoleKind
  /** Alias-Tokens zum Hervorheben passender Vorlagen in der Liste. */
  aliases: string[]
  get: (profile: StyleProfile) => string
  set: (profile: StyleProfile, value: string) => StyleProfile
}

interface RoleGroup {
  title: string
  description: string
  roles: RoleDef[]
}

function buildRoleGroups(t: Translate): RoleGroup[] {
  return [
    {
      title: t('roleGroupHeadings'),
      description: t('roleGroupHeadingsDesc'),
      roles: [
        roleHeading(1, t),
        roleHeading(2, t),
        roleHeading(3, t),
        roleHeading(4, t),
        roleHeading(5, t),
        roleHeading(6, t),
      ],
    },
    {
      title: t('roleGroupBlocks'),
      description: t('roleGroupBlocksDesc'),
      roles: [
        {
          key: 'normal',
          label: t('roleNormal'),
          kind: 'paragraph',
          aliases: ['standard', 'normal', 'bodytext'],
          get: (p) => p.paragraphs.normal ?? '',
          set: (p, v) => ({ ...p, paragraphs: { ...p.paragraphs, normal: emptyToNull(v) } }),
        },
        {
          key: 'blockQuote',
          label: t('roleBlockQuote'),
          kind: 'paragraph',
          aliases: ['zitat', 'quote', 'blockzitat'],
          get: (p) => p.paragraphs.blockQuote ?? '',
          set: (p, v) => ({
            ...p,
            paragraphs: { ...p.paragraphs, blockQuote: emptyToNull(v) },
          }),
        },
        {
          key: 'codeBlock',
          label: t('roleCodeBlock'),
          kind: 'paragraph',
          aliases: ['keinleerraum', 'nospacing', 'htmlpreformatted', 'code'],
          get: (p) => p.paragraphs.codeBlock ?? '',
          set: (p, v) => ({
            ...p,
            paragraphs: { ...p.paragraphs, codeBlock: emptyToNull(v) },
          }),
        },
        {
          key: 'table',
          label: t('roleTable'),
          kind: 'paragraph',
          aliases: ['tabellenraster', 'tablegrid', 'table'],
          get: (p) => p.paragraphs.table ?? '',
          set: (p, v) => ({ ...p, paragraphs: { ...p.paragraphs, table: emptyToNull(v) } }),
        },
      ],
    },
    {
      title: t('roleGroupLists'),
      description: t('roleGroupListsDesc'),
      roles: [
        {
          key: 'unorderedList',
          label: t('roleUnorderedList'),
          kind: 'paragraph',
          aliases: ['listenabsatz', 'listparagraph', 'listbullet'],
          get: (p) => p.paragraphs.unorderedList ?? '',
          set: (p, v) => ({
            ...p,
            paragraphs: { ...p.paragraphs, unorderedList: emptyToNull(v) },
          }),
        },
        {
          key: 'orderedList',
          label: t('roleOrderedList'),
          kind: 'paragraph',
          aliases: ['listenabsatz', 'listparagraph', 'listnumber'],
          get: (p) => p.paragraphs.orderedList ?? '',
          set: (p, v) => ({
            ...p,
            paragraphs: { ...p.paragraphs, orderedList: emptyToNull(v) },
          }),
        },
      ],
    },
    {
      title: t('roleGroupChars'),
      description: t('roleGroupCharsDesc'),
      roles: [
        {
          key: 'strong',
          label: t('roleStrong'),
          kind: 'character',
          aliases: ['fett', 'strong'],
          get: (p) => p.characters.strong ?? '',
          set: (p, v) => ({ ...p, characters: { ...p.characters, strong: emptyToNull(v) } }),
        },
        {
          key: 'emphasis',
          label: t('roleEmphasis'),
          kind: 'character',
          aliases: ['kursiv', 'emphasis', 'hervorhebung'],
          get: (p) => p.characters.emphasis ?? '',
          set: (p, v) => ({
            ...p,
            characters: { ...p.characters, emphasis: emptyToNull(v) },
          }),
        },
        {
          key: 'strongEmphasis',
          label: t('roleStrongEmphasis'),
          kind: 'character',
          aliases: ['fettkursiv', 'strongemphasis', 'bolditalic', 'buchtitel', 'booktitle'],
          get: (p) => p.characters.strongEmphasis ?? '',
          set: (p, v) => ({
            ...p,
            characters: { ...p.characters, strongEmphasis: emptyToNull(v) },
          }),
        },
        {
          key: 'strikethrough',
          label: t('roleStrikethrough'),
          kind: 'character',
          aliases: ['durchgestrichen', 'strikethrough'],
          get: (p) => p.characters.strikethrough ?? '',
          set: (p, v) => ({
            ...p,
            characters: { ...p.characters, strikethrough: emptyToNull(v) },
          }),
        },
        {
          key: 'inlineCode',
          label: t('roleInlineCode'),
          kind: 'character',
          aliases: ['htmlcode', 'code', 'quelltext'],
          get: (p) => p.characters.inlineCode ?? '',
          set: (p, v) => ({
            ...p,
            characters: { ...p.characters, inlineCode: emptyToNull(v) },
          }),
        },
        {
          key: 'link',
          label: t('roleLink'),
          kind: 'character',
          aliases: ['hyperlink'],
          get: (p) => p.characters.link ?? '',
          set: (p, v) => ({ ...p, characters: { ...p.characters, link: emptyToNull(v) } }),
        },
      ],
    },
  ]
}

function roleHeading(level: 1 | 2 | 3 | 4 | 5 | 6, t: Translate): RoleDef {
  const key = `heading${level}` as const
  return {
    key,
    label: t('roleHeading', { level }),
    kind: 'paragraph',
    aliases: [
      `berschrift${level}`,
      `heading${level}`,
      `ueberschrift${level}`,
      `uberschrift${level}`,
    ],
    get: (p) => {
      const map = p.paragraphs
      switch (level) {
        case 1:
          return map.heading1 ?? ''
        case 2:
          return map.heading2 ?? ''
        case 3:
          return map.heading3 ?? ''
        case 4:
          return map.heading4 ?? ''
        case 5:
          return map.heading5 ?? ''
        case 6:
          return map.heading6 ?? ''
      }
    },
    set: (p, v) => {
      const value = emptyToNull(v)
      const paragraphs = { ...p.paragraphs }
      switch (level) {
        case 1:
          paragraphs.heading1 = value
          break
        case 2:
          paragraphs.heading2 = value
          break
        case 3:
          paragraphs.heading3 = value
          break
        case 4:
          paragraphs.heading4 = value
          break
        case 5:
          paragraphs.heading5 = value
          break
        case 6:
          paragraphs.heading6 = value
          break
      }
      return { ...p, paragraphs }
    },
  }
}

function emptyToNull(value: string): string | null {
  const trimmed = value.trim()
  return trimmed ? trimmed : null
}

function normalizeKey(value: string): string {
  return value
    .replace(/ä/gi, 'ae')
    .replace(/ö/gi, 'oe')
    .replace(/ü/gi, 'ue')
    .replace(/ß/g, 'ss')
    .replace(/[^a-zA-Z0-9]/g, '')
    .toLowerCase()
}

function styleMatchesAlias(style: TemplateStyle, aliases: string[]): boolean {
  const candidates = [normalizeKey(style.id), normalizeKey(style.name)]
  return aliases.some((alias) => candidates.includes(normalizeKey(alias)))
}

function stylesForRole(
  kind: RoleKind,
  styles: TemplateStyle[],
  locale: string,
): TemplateStyle[] {
  const filtered = styles.filter((style) =>
    kind === 'paragraph'
      ? style.kind === 'paragraph' || style.kind === 'table' || style.kind === 'other'
      : style.kind === 'character' || style.kind === 'other',
  )
  const list = filtered.length > 0 ? filtered : styles
  return [...list].sort((a, b) => a.name.localeCompare(b.name, locale))
}

function partitionStyles(role: RoleDef, styles: TemplateStyle[], locale: string) {
  const pool = stylesForRole(role.kind, styles, locale)
  const recommended: TemplateStyle[] = []
  const rest: TemplateStyle[] = []
  for (const style of pool) {
    if (styleMatchesAlias(style, role.aliases)) recommended.push(style)
    else rest.push(style)
  }
  return { recommended, rest }
}

function styleLabel(style: TemplateStyle): string {
  if (!style.name || style.name === style.id) return style.id
  return `${style.name} (${style.id})`
}

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error)
}

interface MappingDialogProps {
  templatePath: string | null
  onClose: () => void
  onSaved: (profilePath: string, profileName: string) => void
}

export function MappingDialog({ templatePath, onClose, onSaved }: MappingDialogProps) {
  const { locale, t } = useLocale()
  const roleGroups = useMemo(() => buildRoleGroups(t), [t])
  const [profile, setProfile] = useState<StyleProfile | null>(null)
  const [styles, setStyles] = useState<TemplateStyle[]>([])
  const [effectiveTemplate, setEffectiveTemplate] = useState<string | null>(null)
  const [status, setStatus] = useState(() => t('mappingLoading'))
  const [busy, setBusy] = useState(true)

  useEffect(() => {
    let cancelled = false
    void (async () => {
      setBusy(true)
      setStatus(t('mappingLoading'))
      try {
        const resolved = await invoke<string | null>('resolve_template_path', {
          path: templatePath,
        })
        if (cancelled) return
        setEffectiveTemplate(resolved)
        const loaded = await invoke<StyleProfile>('load_shared_style_profile', {
          templatePath: resolved,
        })
        if (cancelled) return
        setProfile(loaded)
        if (resolved) {
          const templateStyles = await invoke<TemplateStyle[]>('inspect_template', {
            path: resolved,
          })
          if (cancelled) return
          setStyles(templateStyles)
          setStatus(
            t('mappingLoaded', {
              count: templateStyles.length,
              name: resolved.split(/[\\/]/).at(-1) ?? resolved,
            }),
          )
        } else {
          setStyles([])
          setStatus(t('mappingNoTemplate'))
        }
      } catch (error) {
        if (!cancelled) setStatus(t('mappingLoadFailed', { error: errorMessage(error) }))
      } finally {
        if (!cancelled) setBusy(false)
      }
    })()
    return () => {
      cancelled = true
    }
  }, [templatePath, t])

  const templateName = useMemo(
    () => (effectiveTemplate ? effectiveTemplate.split(/[\\/]/).at(-1) : null),
    [effectiveTemplate],
  )

  async function handleSave() {
    if (!profile) return
    setBusy(true)
    try {
      const path = await invoke<string>('save_shared_style_profile', { profile })
      onSaved(path, profile.name)
      onClose()
    } catch (error) {
      setStatus(t('mappingSaveFailed', { error: errorMessage(error) }))
    } finally {
      setBusy(false)
    }
  }

  async function handleAutoMap() {
    if (!effectiveTemplate) {
      setStatus(t('mappingAutoNeedsTemplate'))
      return
    }
    setBusy(true)
    try {
      const regenerated = await invoke<StyleProfile>('profile_from_template', {
        path: effectiveTemplate,
      })
      setProfile(regenerated)
      setStatus(t('mappingAutoDone', { name: regenerated.name }))
    } catch (error) {
      setStatus(t('mappingAutoFailed', { error: errorMessage(error) }))
    } finally {
      setBusy(false)
    }
  }

  async function handleExportJson() {
    if (!profile) return
    const target = await save({
      title: t('mappingExportTitle'),
      defaultPath: 'style-profile.json',
      filters: [{ name: t('filterStyleProfile'), extensions: ['json'] }],
    })
    if (!target) return
    try {
      await invoke('write_markdown', {
        path: target,
        markdown: JSON.stringify(profile, null, 2),
      })
      setStatus(t('mappingExported', { path: target }))
    } catch (error) {
      setStatus(t('mappingExportFailed', { error: errorMessage(error) }))
    }
  }

  async function handleImportJson() {
    const selected = await open({
      title: t('mappingImportTitle'),
      multiple: false,
      filters: [{ name: t('filterStyleProfile'), extensions: ['json'] }],
    })
    if (typeof selected !== 'string') return
    try {
      const imported = await invoke<StyleProfile>('inspect_style_profile', { path: selected })
      setProfile(imported)
      setStatus(t('mappingImported', { name: imported.name }))
    } catch (error) {
      setStatus(t('mappingImportFailed', { error: errorMessage(error) }))
    }
  }

  if (!profile) {
    return (
      <div
        className="mapping-overlay"
        role="dialog"
        aria-modal="true"
        aria-label={t('mappingTitle')}
      >
        <div className="mapping-dialog">
          <header className="mapping-header">
            <h2>{t('mappingTitle')}</h2>
            <button type="button" onClick={onClose} aria-label={t('mappingCloseAria')}>
              ×
            </button>
          </header>
          <p className="mapping-status">{status}</p>
        </div>
      </div>
    )
  }

  return (
    <div
      className="mapping-overlay"
      role="dialog"
      aria-modal="true"
      aria-label={t('mappingTitle')}
    >
      <div className="mapping-dialog">
        <header className="mapping-header">
          <div>
            <h2>{t('mappingTitle')}</h2>
            <p className="mapping-subtitle">
              {t('mappingSubtitle')}
              {templateName ? t('mappingSubtitleTemplate', { name: templateName }) : ''}
            </p>
          </div>
          <button type="button" onClick={onClose} aria-label={t('mappingCloseAria')}>
            ×
          </button>
        </header>

        <label className="mapping-name">
          <span>{t('mappingProfileName')}</span>
          <input
            value={profile.name}
            disabled={busy}
            onChange={(event) => setProfile({ ...profile, name: event.target.value })}
          />
        </label>

        <p className="mapping-status">{status}</p>

        <div className="mapping-roles">
          {roleGroups.map((group) => (
            <section key={group.title} className="mapping-group">
              <header className="mapping-group-header">
                <h3>{group.title}</h3>
                <p>{group.description}</p>
              </header>
              {group.roles.map((role) => {
                const { recommended, rest } = partitionStyles(role, styles, locale)
                const value = role.get(profile)
                const known =
                  recommended.some((style) => style.id === value) ||
                  rest.some((style) => style.id === value)
                const hasOptions = recommended.length + rest.length > 0
                return (
                  <label key={role.key} className="mapping-row">
                    <span>
                      {role.label}
                      {role.hint ? <small className="mapping-hint">{role.hint}</small> : null}
                    </span>
                    {hasOptions ? (
                      <select
                        value={value}
                        disabled={busy}
                        onChange={(event) => setProfile(role.set(profile, event.target.value))}
                      >
                        <option value="">{t('mappingUnset')}</option>
                        {!known && value ? (
                          <option value={value}>
                            {t('mappingCurrent', { value })}
                          </option>
                        ) : null}
                        {recommended.length > 0 ? (
                          <optgroup label={t('mappingRecommended')}>
                            {recommended.map((style) => (
                              <option key={style.id} value={style.id}>
                                {styleLabel(style)}
                              </option>
                            ))}
                          </optgroup>
                        ) : null}
                        {rest.length > 0 ? (
                          <optgroup
                            label={
                              role.kind === 'character'
                                ? t('mappingCharStyles')
                                : t('mappingParaStyles')
                            }
                          >
                            {rest.map((style) => (
                              <option key={style.id} value={style.id}>
                                {styleLabel(style)}
                              </option>
                            ))}
                          </optgroup>
                        ) : null}
                      </select>
                    ) : (
                      <input
                        value={value}
                        disabled={busy}
                        placeholder={t('mappingStyleIdPlaceholder')}
                        onChange={(event) => setProfile(role.set(profile, event.target.value))}
                      />
                    )}
                  </label>
                )
              })}
            </section>
          ))}
        </div>

        <footer className="mapping-actions">
          <button type="button" onClick={() => void handleImportJson()} disabled={busy}>
            {t('mappingImportJson')}
          </button>
          <button type="button" onClick={() => void handleExportJson()} disabled={busy}>
            {t('mappingExportJson')}
          </button>
          <button
            type="button"
            onClick={() => void handleAutoMap()}
            disabled={busy || !effectiveTemplate}
          >
            {t('mappingAutoFromTemplate')}
          </button>
          <span className="mapping-actions-spacer" />
          <button type="button" onClick={onClose} disabled={busy}>
            {t('mappingCancel')}
          </button>
          <button
            type="button"
            className="primary-action"
            onClick={() => void handleSave()}
            disabled={busy}
          >
            {t('mappingSave')}
          </button>
        </footer>
      </div>
    </div>
  )
}
