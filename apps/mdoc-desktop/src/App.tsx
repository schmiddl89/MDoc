import {
  useCallback,
  useDeferredValue,
  useEffect,
  useMemo,
  useRef,
  useState,
} from 'react'
import { invoke, isTauri } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { documentDir, dirname, join } from '@tauri-apps/api/path'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { message, open, save } from '@tauri-apps/plugin-dialog'
import ReactMarkdown from 'react-markdown'
import rehypeKatex from 'rehype-katex'
import remarkGfm from 'remark-gfm'
import remarkMath from 'remark-math'
import 'katex/dist/katex.min.css'
import './editor.css'
import { AppMenu } from './AppMenu'
import { MappingDialog } from './MappingDialog'
import { guideMarkdown, useLocale, type AppSettingsDto, type Locale } from './i18n'

interface DocumentStats {
  blocks: number
  words: number
  characters: number
}

interface ExportResult {
  blocks: number
  path: string
}

interface ImportResult {
  markdown: string
  blocks: number
}

interface AppPaths {
  configDir: string
  sharedProfilePath: string
  sharedTemplatePath: string | null
  sharedTemplatePathFile: string
  defaultTemplatePath: string | null
  defaultTemplateAvailable: boolean
}

interface LaunchOptions {
  mapStyles: boolean
  guide: boolean
  documentPath: string | null
}

const TEMPLATE_STORAGE_KEY = 'mdoc.templatePath'
const STYLE_PROFILE_STORAGE_KEY = 'mdoc.styleProfilePath'
const SPLIT_STORAGE_KEY = 'mdoc.splitPercent'
const SPLIT_MIN = 20
const SPLIT_MAX = 80

function storedSplitPercent(): number {
  const raw = window.localStorage.getItem(SPLIT_STORAGE_KEY)
  const value = raw ? Number(raw) : 50
  if (!Number.isFinite(value)) return 50
  return Math.min(SPLIT_MAX, Math.max(SPLIT_MIN, value))
}

function persistSplitPercent(value: number) {
  const clamped = Math.min(SPLIT_MAX, Math.max(SPLIT_MIN, Math.round(value)))
  window.localStorage.setItem(SPLIT_STORAGE_KEY, String(clamped))
  if (isTauri()) {
    void invoke('set_split_percent', { percent: clamped }).catch(() => {
      /* optional */
    })
  }
  return clamped
}

function storedPath(key: string): string | null {
  return window.localStorage.getItem(key)
}

function fileName(
  path: string | null,
  guideMode: boolean,
  untitled: string,
  guideName: string,
): string {
  if (guideMode) return guideName
  if (!path) return untitled
  return path.split(/[\\/]/).at(-1) ?? path
}

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error)
}

/** Zeilenenden vereinheitlichen — Textarea/Windows sonst oft falsches „dirty“. */
function normalizeMarkdown(value: string): string {
  return value.replace(/\r\n/g, '\n').replace(/\r/g, '\n')
}

async function finishWindowClose(): Promise<void> {
  await invoke('accept_window_close')
  try {
    await getCurrentWindow().close()
  } catch {
    await getCurrentWindow().destroy()
  }
}

async function defaultSavePath(
  currentPath: string | null,
  extension: string,
  baseFallback: string,
): Promise<string | undefined> {
  const baseName =
    fileName(currentPath, false, baseFallback, baseFallback).replace(/\.[^.]+$/, '') ||
    baseFallback
  const file = `${baseName}.${extension}`
  try {
    if (currentPath) {
      return await join(await dirname(currentPath), file)
    }
    return await join(await documentDir(), file)
  } catch {
    return undefined
  }
}

function App() {
  const { locale, setLocale, t, guide, ready: localeReady } = useLocale()
  const [markdown, setMarkdown] = useState(() => guideMarkdown(locale))
  const [savedMarkdown, setSavedMarkdown] = useState(() => guideMarkdown(locale))
  const [documentPath, setDocumentPath] = useState<string | null>(null)
  const [tauriStats, setTauriStats] = useState<DocumentStats>({
    blocks: 0,
    words: 0,
    characters: guideMarkdown(locale).length,
  })
  const [status, setStatus] = useState(() => t('ready'))
  const [busy, setBusy] = useState(false)
  const [templatePath, setTemplatePath] = useState<string | null>(() =>
    storedPath(TEMPLATE_STORAGE_KEY),
  )
  const [styleProfilePath, setStyleProfilePath] = useState<string | null>(() =>
    storedPath(STYLE_PROFILE_STORAGE_KEY),
  )
  const [splitPercent, setSplitPercent] = useState(storedSplitPercent)
  const [mappingOpen, setMappingOpen] = useState(false)
  const [guideMode, setGuideMode] = useState(false)
  const [appPaths, setAppPaths] = useState<AppPaths | null>(null)
  const [ribbonPlacement, setRibbonPlacement] = useState<'home' | 'ownTab'>('home')
  const workspaceRef = useRef<HTMLElement | null>(null)
  const draggingSplit = useRef(false)
  const isDirtyRef = useRef(false)
  const confirmCloseRef = useRef<() => Promise<boolean>>(async () => true)
  const closingRef = useRef(false)
  const guideSeedRef = useRef(normalizeMarkdown(guideMarkdown(locale)))
  const markdownRef = useRef(markdown)
  const savedMarkdownRef = useRef(savedMarkdown)
  markdownRef.current = markdown
  savedMarkdownRef.current = savedMarkdown
  const runningInTauri = isTauri()
  const isDirty = normalizeMarkdown(markdown) !== normalizeMarkdown(savedMarkdown)
  isDirtyRef.current = isDirty
  const previewMarkdown = useDeferredValue(markdown)
  const usingSharedMapping =
    !!styleProfilePath &&
    !!appPaths?.sharedProfilePath &&
    styleProfilePath === appPaths.sharedProfilePath
  const browserStats = useMemo<DocumentStats>(
    () => ({
      blocks: markdown.trim() ? markdown.trim().split(/\n\s*\n/).length : 0,
      words: markdown.trim() ? markdown.trim().split(/\s+/).length : 0,
      characters: markdown.length,
    }),
    [markdown],
  )
  const stats = runningInTauri ? tauriStats : browserStats
  const title = useMemo(
    () =>
      `${isDirty ? '● ' : ''}${fileName(
        documentPath,
        guideMode,
        t('untitled'),
        t('guideFileName'),
      )}`,
    [documentPath, guideMode, isDirty, t],
  )

  // Startinhalt / Hilfe-Fenster: Guide-Text der aktuellen Locale nachziehen.
  useEffect(() => {
    if (!localeReady) return
    const nextGuide = normalizeMarkdown(guide)
    const deGuide = normalizeMarkdown(guideMarkdown('de'))
    const enGuide = normalizeMarkdown(guideMarkdown('en'))
    const current = normalizeMarkdown(markdownRef.current)
    const stillSeed =
      current === guideSeedRef.current || current === deGuide || current === enGuide
    if (!(guideMode || (stillSeed && !documentPath))) return
    if (
      current === nextGuide &&
      normalizeMarkdown(savedMarkdownRef.current) === nextGuide
    ) {
      if (guideMode) setStatus(t('statusHelp'))
      return
    }
    guideSeedRef.current = nextGuide
    setMarkdown(guide)
    setSavedMarkdown(guide)
    if (guideMode) setStatus(t('statusHelp'))
  }, [guide, guideMode, localeReady, documentPath, t])

  useEffect(() => {
    if (!runningInTauri) return
    const timeout = window.setTimeout(() => {
      void invoke<DocumentStats>('analyze_markdown', { markdown })
        .then(setTauriStats)
        .catch((error: unknown) =>
          setStatus(t('statusParserError', { error: errorMessage(error) })),
        )
    }, 180)
    return () => window.clearTimeout(timeout)
  }, [markdown, runningInTauri, t])

  const saveDocument = useCallback(
    async (choosePath = false): Promise<boolean> => {
      try {
        let target = choosePath ? null : documentPath
        if (!target) {
          target = await save({
            title: t('dialogSaveMarkdown'),
            defaultPath: await defaultSavePath(
              documentPath,
              'md',
              t('documentBaseName'),
            ),
            filters: [{ name: t('filterMarkdown'), extensions: ['md', 'markdown'] }],
          })
        }
        if (!target) return false
        setBusy(true)
        await invoke('write_markdown', { path: target, markdown })
        setDocumentPath(target)
        setSavedMarkdown(markdown)
        setStatus(t('statusSaved', { path: target }))
        return true
      } catch (error) {
        setStatus(t('statusSaveFailed', { error: errorMessage(error) }))
        return false
      } finally {
        setBusy(false)
      }
    },
    [documentPath, markdown, t],
  )

  const confirmDiscardOrSave = useCallback(async (): Promise<boolean> => {
    if (!isDirtyRef.current) return true
    if (runningInTauri) {
      try {
        const yes = t('btnSave')
        const no = t('btnDontSave')
        const cancel = t('btnCancel')
        const choice = await message(t('dialogUnsavedBody'), {
          title: t('dialogUnsavedTitle'),
          kind: 'warning',
          buttons: { yes, no, cancel },
        })
        if (choice === yes || choice === 'Yes' || choice === 'Speichern') {
          return await saveDocument()
        }
        if (choice === no || choice === 'No' || choice === 'Nicht speichern') return true
        return false
      } catch (error) {
        setStatus(t('statusConfirmFailed', { error: errorMessage(error) }))
        return false
      }
    }
    if (!window.confirm(t('confirmDiscardBrowser'))) return false
    return true
  }, [runningInTauri, saveDocument, t])
  confirmCloseRef.current = confirmDiscardOrSave

  const loadDocumentFromPath = useCallback(
    async (selected: string) => {
      setBusy(true)
      const isDocx = selected.toLowerCase().endsWith('.docx')
      setStatus(isDocx ? t('statusImportingDocx') : t('statusLoadingDocument'))
      try {
        setGuideMode(false)
        if (isDocx) {
          const imported = await invoke<ImportResult>('import_docx', {
            path: selected,
            styleProfilePath,
          })
          setDocumentPath(null)
          setMarkdown(imported.markdown)
          setSavedMarkdown('')
          setStatus(
            t('statusImportedBlocks', {
              blocks: imported.blocks,
              path: selected,
            }),
          )
          return
        }
        const contents = await invoke<string>('read_markdown', { path: selected })
        setDocumentPath(selected)
        setMarkdown(contents)
        setSavedMarkdown(contents)
        setStatus(t('statusOpened', { path: selected }))
      } catch (error) {
        setStatus(t('statusOpenFailed', { error: errorMessage(error) }))
      } finally {
        setBusy(false)
      }
    },
    [styleProfilePath, t],
  )

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === 's') {
        event.preventDefault()
        void saveDocument(event.shiftKey)
      }
    }
    window.addEventListener('keydown', onKeyDown)
    return () => window.removeEventListener('keydown', onKeyDown)
  }, [saveDocument])

  useEffect(() => {
    if (!runningInTauri) return
    let cancelled = false
    void invoke<AppSettingsDto>('get_app_settings')
      .then((settings) => {
        if (cancelled) return
        setRibbonPlacement(settings.ribbonPlacement === 'ownTab' ? 'ownTab' : 'home')
        if (settings.splitPercent != null && Number.isFinite(settings.splitPercent)) {
          const clamped = Math.min(
            SPLIT_MAX,
            Math.max(SPLIT_MIN, Math.round(settings.splitPercent)),
          )
          setSplitPercent(clamped)
          window.localStorage.setItem(SPLIT_STORAGE_KEY, String(clamped))
        }
      })
      .catch(() => {
        /* defaults */
      })
    return () => {
      cancelled = true
    }
  }, [runningInTauri])

  useEffect(() => {
    if (!runningInTauri || !localeReady) return
    let cancelled = false
    void invoke<AppPaths>('get_app_paths')
      .then((paths) => {
        if (cancelled) return
        setAppPaths(paths)
        if (!storedPath(STYLE_PROFILE_STORAGE_KEY) && paths.sharedProfilePath) {
          setStyleProfilePath(paths.sharedProfilePath)
        }
        if (!templatePath && paths.sharedTemplatePath) {
          setTemplatePath(paths.sharedTemplatePath)
          window.localStorage.setItem(TEMPLATE_STORAGE_KEY, paths.sharedTemplatePath)
          setStatus(t('statusSharedTemplate', { path: paths.sharedTemplatePath }))
        } else if (templatePath && !paths.sharedTemplatePath) {
          void invoke('save_shared_template_path', { path: templatePath }).catch(() => {
            /* optional */
          })
        } else if (!templatePath) {
          setStatus(t('statusNoTemplate'))
        }
        void invoke('ensure_shared_template_styles_cache').catch(() => {
          /* optional */
        })
      })
      .catch(() => {
        /* optional */
      })
    return () => {
      cancelled = true
    }
  }, [runningInTauri, templatePath, localeReady, t])

  useEffect(() => {
    if (!runningInTauri || !localeReady) return
    let cancelled = false
    void invoke<LaunchOptions>('get_launch_options')
      .then(async (options) => {
        if (cancelled) return
        if (options.guide) {
          setGuideMode(true)
          setMarkdown(guide)
          setSavedMarkdown(guide)
          guideSeedRef.current = normalizeMarkdown(guide)
          setDocumentPath(null)
          setStatus(t('statusHelp'))
          try {
            await getCurrentWindow().setTitle(t('helpTitle'))
          } catch {
            /* optional */
          }
        }
        if (options.mapStyles) setMappingOpen(true)
        if (options.documentPath) await loadDocumentFromPath(options.documentPath)
      })
      .catch((error: unknown) => {
        if (!cancelled) {
          setStatus(t('statusLaunchOptionsFailed', { error: errorMessage(error) }))
        }
      })
    return () => {
      cancelled = true
    }
  }, [loadDocumentFromPath, runningInTauri, localeReady, guide, t])

  useEffect(() => {
    if (!runningInTauri) return
    let disposed = false
    let unlisten: (() => void) | undefined
    void listen('mdoc-close-requested', () => {
      if (closingRef.current) return
      void (async () => {
        closingRef.current = true
        try {
          if (isDirtyRef.current) {
            const proceed = await confirmCloseRef.current()
            if (!proceed) {
              closingRef.current = false
              return
            }
          }
          await finishWindowClose()
        } catch (error) {
          closingRef.current = false
          setStatus(t('statusCloseFailed', { error: errorMessage(error) }))
        }
      })()
    }).then((fn) => {
      if (disposed) {
        fn()
        return
      }
      unlisten = fn
    })
    return () => {
      disposed = true
      unlisten?.()
    }
  }, [runningInTauri, t])

  useEffect(() => {
    const onPointerMove = (event: PointerEvent) => {
      if (!draggingSplit.current || !workspaceRef.current) return
      const rect = workspaceRef.current.getBoundingClientRect()
      if (rect.width <= 0) return
      const next = ((event.clientX - rect.left) / rect.width) * 100
      setSplitPercent(Math.min(SPLIT_MAX, Math.max(SPLIT_MIN, next)))
    }
    const stopDragging = () => {
      if (!draggingSplit.current) return
      draggingSplit.current = false
      document.body.classList.remove('is-resizing-split')
      setSplitPercent((current) => persistSplitPercent(current))
    }
    window.addEventListener('pointermove', onPointerMove)
    window.addEventListener('pointerup', stopDragging)
    window.addEventListener('pointercancel', stopDragging)
    return () => {
      window.removeEventListener('pointermove', onPointerMove)
      window.removeEventListener('pointerup', stopDragging)
      window.removeEventListener('pointercancel', stopDragging)
      document.body.classList.remove('is-resizing-split')
    }
  }, [])

  async function openDocument() {
    if (!(await confirmDiscardOrSave())) return
    try {
      const selected = await open({
        title: t('dialogOpenTitle'),
        multiple: false,
        filters: [
          {
            name: t('filterOpen'),
            extensions: ['md', 'markdown', 'mdown', 'docx'],
          },
        ],
      })
      if (typeof selected !== 'string') return
      await loadDocumentFromPath(selected)
    } catch (error) {
      setStatus(t('statusOpenFailed', { error: errorMessage(error) }))
    }
  }

  async function exportDocument() {
    try {
      const target = await save({
        title: t('dialogExportTitle'),
        defaultPath: await defaultSavePath(documentPath, 'docx', t('documentBaseName')),
        filters: [{ name: t('filterWordDoc'), extensions: ['docx'] }],
      })
      if (!target) return
      setBusy(true)
      setStatus(t('statusWritingDocx'))
      const result = await invoke<ExportResult>('export_docx', {
        path: target,
        markdown,
        settings: { templatePath, styleProfilePath },
      })
      setStatus(t('statusExported', { blocks: result.blocks, path: result.path }))
    } catch (error) {
      setStatus(t('statusExportFailed', { error: errorMessage(error) }))
    } finally {
      setBusy(false)
    }
  }

  async function chooseTemplate() {
    try {
      const selected = await open({
        title: t('dialogTemplateTitle'),
        multiple: false,
        filters: [{ name: t('filterTemplate'), extensions: ['dotm', 'dotx', 'docx'] }],
      })
      if (typeof selected !== 'string') return
      setBusy(true)
      const styles = await invoke<unknown[]>('inspect_template', { path: selected })
      setTemplatePath(selected)
      window.localStorage.setItem(TEMPLATE_STORAGE_KEY, selected)
      await invoke('save_shared_template_path', { path: selected })
      setStatus(
        t('statusTemplateLoaded', { count: styles.length, path: selected }),
      )
    } catch (error) {
      setStatus(t('statusTemplateFailed', { error: errorMessage(error) }))
    } finally {
      setBusy(false)
    }
  }

  function openMappingDialog() {
    setMappingOpen(true)
  }

  async function clearExportConfiguration() {
    setTemplatePath(null)
    window.localStorage.removeItem(TEMPLATE_STORAGE_KEY)
    try {
      await invoke('save_shared_template_path', { path: null })
    } catch {
      /* optional */
    }
    if (appPaths?.sharedProfilePath) {
      setStyleProfilePath(appPaths.sharedProfilePath)
      window.localStorage.setItem(STYLE_PROFILE_STORAGE_KEY, appPaths.sharedProfilePath)
    } else {
      setStyleProfilePath(null)
      window.localStorage.removeItem(STYLE_PROFILE_STORAGE_KEY)
    }
    setStatus(t('statusTemplateCleared'))
  }

  async function newDocument() {
    if (!(await confirmDiscardOrSave())) return
    setGuideMode(false)
    setMarkdown('')
    setSavedMarkdown('')
    setDocumentPath(null)
    setStatus(t('statusNewDocument'))
  }

  async function openHelpWindow() {
    try {
      await invoke('open_guide_window')
      setStatus(t('statusHelpOpened'))
    } catch (error) {
      setStatus(t('statusHelpFailed', { error: errorMessage(error) }))
    }
  }

  async function onLanguageChange(next: Locale) {
    if (next === locale) return
    await setLocale(next)
  }

  async function onRibbonChange(next: 'home' | 'ownTab') {
    if (next === ribbonPlacement) return
    const previous = ribbonPlacement
    setRibbonPlacement(next)
    try {
      await invoke('set_ribbon_placement', { placement: next })
      setStatus(next === 'ownTab' ? t('statusRibbonOwnTab') : t('statusRibbonHome'))
    } catch (error) {
      setRibbonPlacement(previous)
      setStatus(t('statusRibbonFailed', { error: errorMessage(error) }))
    }
  }

  return (
    <main className="app-shell">
      <header className="titlebar">
        <div className="brand" aria-label="MDoc">
          <img
            className="brand-logo"
            src="/logo_white.svg"
            alt="MDoc"
            width={168}
            height={52}
          />
        </div>
        <div className="document-title" title={documentPath ?? undefined}>
          {title}
        </div>
        <div className="title-actions">
          <button type="button" onClick={() => void newDocument()} disabled={busy}>
            {t('newDoc')}
          </button>
          <button type="button" onClick={() => void openDocument()} disabled={busy}>
            {t('open')}
          </button>
          <button type="button" onClick={() => void saveDocument()} disabled={busy}>
            {t('save')}
          </button>
          <button
            type="button"
            className={
              templatePath || appPaths?.defaultTemplateAvailable ? 'configuration-active' : ''
            }
            onClick={() => void chooseTemplate()}
            disabled={busy}
            title={
              templatePath ??
              appPaths?.defaultTemplatePath ??
              t('templateTooltipFallback')
            }
          >
            {t('template')}
            {templatePath ? ' ✓' : appPaths?.defaultTemplateAvailable ? ' ·' : ' …'}
          </button>
          <button
            type="button"
            className={usingSharedMapping || styleProfilePath ? 'configuration-active' : ''}
            onClick={openMappingDialog}
            disabled={busy}
            title={styleProfilePath ?? t('mappingTooltipFallback')}
          >
            {t('mapping')}
            {usingSharedMapping || styleProfilePath ? ' ✓' : ' …'}
          </button>
          {templatePath && (
            <button
              type="button"
              className="configuration-clear"
              onClick={clearExportConfiguration}
              disabled={busy}
              title={t('clearTemplateTooltip')}
              aria-label={t('clearTemplateAria')}
            >
              ×
            </button>
          )}
          <button
            type="button"
            className="primary-action"
            onClick={() => void exportDocument()}
            disabled={busy}
          >
            {t('exportDocx')}
          </button>
          <AppMenu
            busy={busy}
            runningInTauri={runningInTauri}
            ribbonPlacement={ribbonPlacement}
            onHelp={() => void openHelpWindow()}
            onLanguageChange={(next) => void onLanguageChange(next)}
            onRibbonChange={(next) => void onRibbonChange(next)}
          />
        </div>
      </header>

      <section
        ref={workspaceRef}
        className="workspace"
        style={{
          gridTemplateColumns: `minmax(0, ${splitPercent}fr) 0 minmax(0, ${100 - splitPercent}fr)`,
        }}
      >
        <div className="pane editor-pane">
          <div className="pane-heading">
            <span>{t('editor')}</span>
            <span>UTF-8</span>
          </div>
          <textarea
            aria-label={t('editorAria')}
            value={markdown}
            onChange={(event) => setMarkdown(event.target.value)}
            placeholder={t('editorPlaceholder')}
            spellCheck
          />
        </div>

        <div
          className="split-handle"
          role="separator"
          aria-orientation="vertical"
          aria-label={t('splitAria')}
          aria-valuemin={SPLIT_MIN}
          aria-valuemax={SPLIT_MAX}
          aria-valuenow={Math.round(splitPercent)}
          tabIndex={0}
          onPointerDown={(event) => {
            event.preventDefault()
            draggingSplit.current = true
            document.body.classList.add('is-resizing-split')
            event.currentTarget.setPointerCapture(event.pointerId)
          }}
          onDoubleClick={() => {
            setSplitPercent(persistSplitPercent(50))
          }}
          onKeyDown={(event) => {
            let next: number | null = null
            if (event.key === 'ArrowLeft') next = splitPercent - 2
            if (event.key === 'ArrowRight') next = splitPercent + 2
            if (event.key === 'Home') next = SPLIT_MIN
            if (event.key === 'End') next = SPLIT_MAX
            if (next === null) return
            event.preventDefault()
            setSplitPercent(persistSplitPercent(next))
          }}
        />

        <div className="pane preview-pane">
          <div className="pane-heading">
            <span>{t('preview')}</span>
          </div>
          <article className="markdown-preview">
            <ReactMarkdown
              remarkPlugins={[remarkGfm, remarkMath]}
              rehypePlugins={[rehypeKatex]}
            >
              {previewMarkdown}
            </ReactMarkdown>
          </article>
        </div>
      </section>

      <footer className="statusbar" aria-live="polite">
        <span className={busy ? 'working' : ''}>{busy ? t('working') : status}</span>
        <span>
          {t('stats', {
            words: stats.words,
            characters: stats.characters,
            blocks: stats.blocks,
          })}
        </span>
      </footer>

      {mappingOpen && (
        <MappingDialog
          templatePath={templatePath}
          onClose={() => setMappingOpen(false)}
          onSaved={(profilePath, profileName) => {
            setStyleProfilePath(profilePath)
            window.localStorage.setItem(STYLE_PROFILE_STORAGE_KEY, profilePath)
            if (templatePath) {
              void invoke('save_shared_template_path', { path: templatePath }).catch(() => {
                /* optional */
              })
            }
            setStatus(t('statusProfileSaved', { name: profileName }))
          }}
        />
      )}
    </main>
  )
}

export default App
