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
import { MappingDialog } from './MappingDialog'
import GUIDE_MARKDOWN from './assets/Hilfe.md?raw'

const INITIAL_MARKDOWN = GUIDE_MARKDOWN

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

function storedPath(key: string): string | null {
  return window.localStorage.getItem(key)
}

function fileName(path: string | null, guideMode = false): string {
  if (guideMode) return 'Hilfe.md'
  if (!path) return 'Unbenannt.md'
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
  // Flag setzen, dann close() — löst CloseRequested erneut aus und wird durchgelassen.
  // destroy() als Fallback, falls close an der ACL scheitert.
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
): Promise<string | undefined> {
  const baseName = fileName(currentPath).replace(/\.[^.]+$/, '') || 'Dokument'
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
  const [markdown, setMarkdown] = useState(INITIAL_MARKDOWN)
  const [savedMarkdown, setSavedMarkdown] = useState(INITIAL_MARKDOWN)
  const [documentPath, setDocumentPath] = useState<string | null>(null)
  const [tauriStats, setTauriStats] = useState<DocumentStats>({
    blocks: 0,
    words: 0,
    characters: INITIAL_MARKDOWN.length,
  })
  const [status, setStatus] = useState('Bereit')
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
  const workspaceRef = useRef<HTMLElement | null>(null)
  const draggingSplit = useRef(false)
  const isDirtyRef = useRef(false)
  const confirmCloseRef = useRef<() => Promise<boolean>>(async () => true)
  const closingRef = useRef(false)
  const runningInTauri = isTauri()
  const isDirty = normalizeMarkdown(markdown) !== normalizeMarkdown(savedMarkdown)
  isDirtyRef.current = isDirty
  /** Vorschau verzögert — Öffnen/Tippen blockiert die UI weniger. */
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
    () => `${isDirty ? '● ' : ''}${fileName(documentPath, guideMode)}`,
    [documentPath, guideMode, isDirty],
  )

  useEffect(() => {
    if (!runningInTauri) return
    const timeout = window.setTimeout(() => {
      void invoke<DocumentStats>('analyze_markdown', { markdown })
        .then(setTauriStats)
        .catch((error: unknown) => setStatus(`Parserfehler: ${errorMessage(error)}`))
    }, 180)
    return () => window.clearTimeout(timeout)
  }, [markdown, runningInTauri])

  const saveDocument = useCallback(
    async (choosePath = false): Promise<boolean> => {
      try {
        let target = choosePath ? null : documentPath
        if (!target) {
          target = await save({
            title: 'Markdown speichern',
            defaultPath: await defaultSavePath(documentPath, 'md'),
            filters: [{ name: 'Markdown', extensions: ['md', 'markdown'] }],
          })
        }
        if (!target) return false
        setBusy(true)
        await invoke('write_markdown', { path: target, markdown })
        setDocumentPath(target)
        setSavedMarkdown(markdown)
        setStatus(`Gespeichert: ${target}`)
        return true
      } catch (error) {
        setStatus(`Speichern fehlgeschlagen: ${errorMessage(error)}`)
        return false
      } finally {
        setBusy(false)
      }
    },
    [documentPath, markdown],
  )

  /** Speichern / Nicht speichern / Abbrechen — true = fortfahren. */
  const confirmDiscardOrSave = useCallback(async (): Promise<boolean> => {
    if (!isDirtyRef.current) return true
    if (runningInTauri) {
      try {
        const choice = await message(
          'Das aktuelle Dokument enthält ungespeicherte Änderungen. Speichern?',
          {
            title: 'Ungespeicherte Änderungen',
            kind: 'warning',
            buttons: {
              yes: 'Speichern',
              no: 'Nicht speichern',
              cancel: 'Abbrechen',
            },
          },
        )
        if (choice === 'Speichern' || choice === 'Yes') {
          return await saveDocument()
        }
        if (choice === 'Nicht speichern' || choice === 'No') return true
        return false
      } catch (error) {
        setStatus(`Rückfrage fehlgeschlagen: ${errorMessage(error)}`)
        return false
      }
    }
    if (!window.confirm('Ungespeicherte Änderungen verwerfen?')) return false
    return true
  }, [runningInTauri, saveDocument])
  confirmCloseRef.current = confirmDiscardOrSave

  const loadDocumentFromPath = useCallback(
    async (selected: string) => {
      setBusy(true)
      const isDocx = selected.toLowerCase().endsWith('.docx')
      setStatus(isDocx ? 'DOCX wird importiert …' : 'Dokument wird geladen …')
      try {
        setGuideMode(false)
        if (isDocx) {
          const imported = await invoke<ImportResult>('import_docx', {
            path: selected,
            styleProfilePath,
          })
          // Noch nicht als .md gespeichert → als ungespeichert markieren.
          setDocumentPath(null)
          setMarkdown(imported.markdown)
          setSavedMarkdown('')
          setStatus(`${imported.blocks} Blöcke aus ${selected} importiert`)
          return
        }
        const contents = await invoke<string>('read_markdown', { path: selected })
        setDocumentPath(selected)
        setMarkdown(contents)
        setSavedMarkdown(contents)
        setStatus(`Geöffnet: ${selected}`)
      } catch (error) {
        setStatus(`Öffnen fehlgeschlagen: ${errorMessage(error)}`)
      } finally {
        setBusy(false)
      }
    },
    [styleProfilePath],
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
          setStatus(`Vorlage (geteilt): ${paths.sharedTemplatePath}`)
        } else if (templatePath && !paths.sharedTemplatePath) {
          // localStorage → AppData migrieren (Word-Add-in liest nur die Datei).
          void invoke('save_shared_template_path', { path: templatePath }).catch(() => {
            /* optional */
          })
        } else if (!templatePath) {
          setStatus(
            'Noch keine Vorlage gewählt — bitte .dotx/.dotm wählen und Mapping speichern (für Word-Einfügen).',
          )
        }
        // mtime+Größe: styles.xml nur neu lesen, wenn sich die Vorlage geändert hat.
        void invoke('ensure_shared_template_styles_cache').catch(() => {
          /* optional — Word kann die .dotm sperren */
        })
      })
      .catch(() => {
        /* optional */
      })
    return () => {
      cancelled = true
    }
  }, [runningInTauri, templatePath])

  useEffect(() => {
    if (!runningInTauri) return
    let cancelled = false
    void invoke<LaunchOptions>('get_launch_options')
      .then(async (options) => {
        if (cancelled) return
        if (options.guide) {
          setGuideMode(true)
          setMarkdown(GUIDE_MARKDOWN)
          setSavedMarkdown(GUIDE_MARKDOWN)
          setDocumentPath(null)
          setStatus('Hilfe')
          try {
            await getCurrentWindow().setTitle('MDoc – Hilfe')
          } catch {
            /* optional */
          }
        }
        if (options.mapStyles) setMappingOpen(true)
        if (options.documentPath) await loadDocumentFromPath(options.documentPath)
      })
      .catch((error: unknown) => {
        if (!cancelled) {
          setStatus(`Startoptionen konnten nicht gelesen werden: ${errorMessage(error)}`)
        }
      })
    return () => {
      cancelled = true
    }
  }, [loadDocumentFromPath, runningInTauri])

  useEffect(() => {
    if (!runningInTauri) return
    let disposed = false
    let unlisten: (() => void) | undefined
    // Rust blockiert Close und sendet dieses Event — zuverlässiger als onCloseRequested.
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
          setStatus(`Schließen fehlgeschlagen: ${errorMessage(error)}`)
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
  }, [runningInTauri])

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
      setSplitPercent((current) => {
        window.localStorage.setItem(SPLIT_STORAGE_KEY, String(Math.round(current)))
        return current
      })
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
        title: 'Markdown oder Word-Dokument öffnen',
        multiple: false,
        filters: [
          {
            name: 'Markdown oder Word-Dokument',
            extensions: ['md', 'markdown', 'mdown', 'docx'],
          },
        ],
      })
      if (typeof selected !== 'string') return
      await loadDocumentFromPath(selected)
    } catch (error) {
      setStatus(`Öffnen fehlgeschlagen: ${errorMessage(error)}`)
    }
  }

  async function exportDocument() {
    try {
      const target = await save({
        title: 'Als Word-Dokument exportieren',
        defaultPath: await defaultSavePath(documentPath, 'docx'),
        filters: [{ name: 'Word-Dokument', extensions: ['docx'] }],
      })
      if (!target) return
      setBusy(true)
      setStatus('DOCX wird geschrieben …')
      const result = await invoke<ExportResult>('export_docx', {
        path: target,
        markdown,
        settings: { templatePath, styleProfilePath },
      })
      setStatus(`${result.blocks} Blöcke nach ${result.path} exportiert`)
    } catch (error) {
      setStatus(`DOCX-Export fehlgeschlagen: ${errorMessage(error)}`)
    } finally {
      setBusy(false)
    }
  }

  async function chooseTemplate() {
    try {
      const selected = await open({
        title: 'Word-Vorlage auswählen',
        multiple: false,
        filters: [
          { name: 'Word-Vorlage', extensions: ['dotm', 'dotx', 'docx'] },
        ],
      })
      if (typeof selected !== 'string') return
      setBusy(true)
      const styles = await invoke<unknown[]>('inspect_template', { path: selected })
      setTemplatePath(selected)
      window.localStorage.setItem(TEMPLATE_STORAGE_KEY, selected)
      await invoke('save_shared_template_path', { path: selected })
      setStatus(`Vorlage mit ${styles.length} Formatvorlagen geladen: ${selected}`)
    } catch (error) {
      setStatus(`Vorlage konnte nicht geladen werden: ${errorMessage(error)}`)
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
    setStatus(
      'Vorlage zurückgesetzt — für Word-Einfügen bitte erneut .dotx/.dotm wählen und Mapping speichern.',
    )
  }

  async function newDocument() {
    if (!(await confirmDiscardOrSave())) return
    setGuideMode(false)
    setMarkdown('')
    setSavedMarkdown('')
    setDocumentPath(null)
    setStatus('Neues Dokument')
  }

  async function openHelpWindow() {
    try {
      await invoke('open_guide_window')
      setStatus('Hilfe in neuem Fenster geöffnet')
    } catch (error) {
      setStatus(`Hilfe konnte nicht geöffnet werden: ${errorMessage(error)}`)
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
            Neu
          </button>
          <button type="button" onClick={() => void openDocument()} disabled={busy}>
            Öffnen
          </button>
          <button type="button" onClick={() => void saveDocument()} disabled={busy}>
            Speichern
          </button>
          <button
            type="button"
            onClick={() => void openHelpWindow()}
            disabled={busy}
            title="Kurzanleitung in neuem Fenster"
          >
            Hilfe
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
              'Word-Vorlage auswählen (Fallback: Normal.dotm)'
            }
          >
            Vorlage{templatePath ? ' ✓' : appPaths?.defaultTemplateAvailable ? ' ·' : ' …'}
          </button>
          <button
            type="button"
            className={usingSharedMapping || styleProfilePath ? 'configuration-active' : ''}
            onClick={openMappingDialog}
            disabled={busy}
            title={styleProfilePath ?? 'Formatvorlagen zuordnen'}
          >
            Mapping{(usingSharedMapping || styleProfilePath) ? ' ✓' : ' …'}
          </button>
          {templatePath && (
            <button
              type="button"
              className="configuration-clear"
              onClick={clearExportConfiguration}
              disabled={busy}
              title="Benutzer-Vorlage zurücksetzen (wieder Normal.dotm)"
              aria-label="Benutzer-Vorlage zurücksetzen"
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
            DOCX exportieren
          </button>
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
            <span>Markdown</span>
            <span>UTF-8</span>
          </div>
          <textarea
            aria-label="Markdown-Editor"
            value={markdown}
            onChange={(event) => setMarkdown(event.target.value)}
            placeholder="Markdown eingeben …"
            spellCheck
          />
        </div>

        <div
          className="split-handle"
          role="separator"
          aria-orientation="vertical"
          aria-label="Aufteilung zwischen Editor und Vorschau"
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
            setSplitPercent(50)
            window.localStorage.setItem(SPLIT_STORAGE_KEY, '50')
          }}
          onKeyDown={(event) => {
            let next: number | null = null
            if (event.key === 'ArrowLeft') next = splitPercent - 2
            if (event.key === 'ArrowRight') next = splitPercent + 2
            if (event.key === 'Home') next = SPLIT_MIN
            if (event.key === 'End') next = SPLIT_MAX
            if (next === null) return
            event.preventDefault()
            const clamped = Math.min(SPLIT_MAX, Math.max(SPLIT_MIN, next))
            setSplitPercent(clamped)
            window.localStorage.setItem(SPLIT_STORAGE_KEY, String(Math.round(clamped)))
          }}
        />

        <div className="pane preview-pane">
          <div className="pane-heading">
            <span>Vorschau</span>
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
        <span className={busy ? 'working' : ''}>{busy ? 'Wird verarbeitet …' : status}</span>
        <span>
          {stats.words} Wörter · {stats.characters} Zeichen · {stats.blocks} Blöcke
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
            setStatus(
              `Style-Profil „${profileName}“ gespeichert — Word-Einfügen nutzt diese Vorlage und das Mapping.`,
            )
          }}
        />
      )}
    </main>
  )
}

export default App
