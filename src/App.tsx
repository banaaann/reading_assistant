import { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from 'react'
import type { CSSProperties } from 'react'
import { listen } from '@tauri-apps/api/event'
import { register, unregisterAll } from '@tauri-apps/plugin-global-shortcut'
import './App.css'
import { DashboardView } from './components/DashboardView'
import { LookupPopup } from './components/LookupPopup'
import {
  appWindow,
  getConfig,
  hideCurrentWindow,
  isTauriRuntime,
  listStarred,
  manualLookup,
  removeStarred,
  resizePopupToContent,
  saveConfig,
  startPopupDrag,
  toggleStarred,
  triggerLookup,
} from './lib/tauri'
import type {
  AppConfig,
  LookupErrorEvent,
  LookupEvent,
  LookupLoadingEvent,
  LookupResult,
  StarredEntry,
} from './lib/types'

const DEFAULT_CONFIG: AppConfig = {
  hotkey: 'Ctrl+Shift+D',
  trigger_source: 'keyboard',
  auto_start: false,
  max_text_length: 280,
  close_on_focus_loss: true,
  popup_width: 380,
  popup_height: 400,
  popup_font_scale: 0.92,
}

function isModifierKey(key: string) {
  return ['Control', 'Shift', 'Alt', 'Meta'].includes(key)
}

function normalizeMainKey(key: string) {
  if (key === ' ') return 'Space'
  if (key.length === 1) return key.toUpperCase()
  return key
}

function formatHotkeyFromEvent(event: KeyboardEvent) {
  const parts: string[] = []

  if (event.ctrlKey) parts.push('Ctrl')
  if (event.altKey) parts.push('Alt')
  if (event.shiftKey) parts.push('Shift')
  if (event.metaKey) parts.push('Meta')

  if (!isModifierKey(event.key)) {
    parts.push(normalizeMainKey(event.key))
  }

  return parts.join('+')
}

function App() {
  const isPopupWindow = appWindow.label === 'popup'

  const [config, setConfig] = useState<AppConfig>(DEFAULT_CONFIG)
  const [configDraft, setConfigDraft] = useState<AppConfig>(DEFAULT_CONFIG)
  const [latestResult, setLatestResult] = useState<LookupResult | null>(null)
  const [starredEntries, setStarredEntries] = useState<StarredEntry[]>([])
  const [selectedEntryId, setSelectedEntryId] = useState<string | null>(null)
  const [testInput, setTestInput] = useState('Reading complex articles improves vocabulary.')
  const [filterType, setFilterType] = useState('')
  const [search, setSearch] = useState('')
  const [status, setStatus] = useState('')
  const [savingConfig, setSavingConfig] = useState(false)
  const [lookupBusy, setLookupBusy] = useState(false)
  const [popupBusy, setPopupBusy] = useState(false)
  const [popupQueryText, setPopupQueryText] = useState('')
  const [popupError, setPopupError] = useState('')
  const [isRecordingHotkey, setIsRecordingHotkey] = useState(false)
  const [popupContentOverflows, setPopupContentOverflows] = useState(false)
  const popupRootRef = useRef<HTMLDivElement | null>(null)

  const loadStarred = useCallback(async () => {
    if (isPopupWindow) {
      return
    }

    const entries = await listStarred({
      search: search || undefined,
      entry_type: filterType || undefined,
    })
    setStarredEntries(entries)
    setSelectedEntryId((current) => current ?? entries[0]?.id ?? null)
  }, [filterType, isPopupWindow, search])

  const setupHotkey = useCallback(
    async (nextConfig: AppConfig) => {
      if (isPopupWindow || !isTauriRuntime) {
        return
      }

      await unregisterAll()
      if (nextConfig.trigger_source !== 'keyboard') {
        return
      }

      await register(nextConfig.hotkey, async (event) => {
        if (event.state !== 'Pressed') {
          return
        }

        try {
          setStatus('')
          await triggerLookup()
        } catch (error) {
          setStatus(error instanceof Error ? error.message : String(error))
        }
      })
    },
    [isPopupWindow],
  )

  useEffect(() => {
    if (isPopupWindow) {
      void appWindow.setAlwaysOnTop(true)
      void getConfig()
        .then((savedConfig) => {
          setConfig(savedConfig)
          setConfigDraft(savedConfig)
        })
        .catch((error) => setStatus(error instanceof Error ? error.message : String(error)))

      const unlistenLoading = listen<LookupLoadingEvent>('lookup-loading', (event) => {
        setPopupBusy(true)
        setPopupQueryText(event.payload.text)
        setPopupError('')
        setLatestResult(null)
      })

      const unlistenResult = listen<LookupEvent>('lookup-result', (event) => {
        setPopupBusy(false)
        setPopupQueryText(
          event.payload.result.payload.kind === 'word'
            ? event.payload.result.payload.lemma
            : event.payload.result.payload.source_text,
        )
        setPopupError('')
        setLatestResult(event.payload.result)
      })

      const unlistenError = listen<LookupErrorEvent>('lookup-error', (event) => {
        setPopupBusy(false)
        setPopupQueryText(event.payload.text ?? '')
        setPopupError(event.payload.message)
        setLatestResult(null)
      })

      const keyHandler = (keyboardEvent: KeyboardEvent) => {
        if (keyboardEvent.key === 'Escape') {
          void appWindow.hide()
        }
      }

      window.addEventListener('keydown', keyHandler)

      return () => {
        void unlistenLoading.then((unlisten) => unlisten())
        void unlistenResult.then((unlisten) => unlisten())
        void unlistenError.then((unlisten) => unlisten())
        window.removeEventListener('keydown', keyHandler)
      }
    }

    let mounted = true
    void (async () => {
      try {
        const savedConfig = await getConfig()
        if (!mounted) return
        setConfig(savedConfig)
        setConfigDraft(savedConfig)
        await setupHotkey(savedConfig)
        await loadStarred()
      } catch (error) {
        if (!mounted) return
        setStatus(error instanceof Error ? error.message : String(error))
      }
    })()

    const unlistenPromise = isTauriRuntime
      ? listen('starred-changed', () => {
          void loadStarred()
        })
      : Promise.resolve(() => {})

    return () => {
      mounted = false
      void unlistenPromise.then((unlisten) => unlisten())
    }
  }, [isPopupWindow, loadStarred, setupHotkey])

  useLayoutEffect(() => {
    if (!isPopupWindow || !isTauriRuntime || !popupRootRef.current) {
      return
    }

    let animationFrame = 0
    let lastHeight = 0

    const requestResize = () => {
      window.cancelAnimationFrame(animationFrame)
      animationFrame = window.requestAnimationFrame(() => {
        const node = popupRootRef.current
        if (!node) {
          return
        }

        const shell = node.querySelector<HTMLElement>('.popup-shell')
        const rootStyle = window.getComputedStyle(node)
        const verticalPadding =
          Number.parseFloat(rootStyle.paddingTop) + Number.parseFloat(rootStyle.paddingBottom)
        const contentHeight = Math.ceil((shell?.scrollHeight ?? node.scrollHeight) + verticalPadding)
        const maxHeight = config.popup_height
        const nextOverflows = contentHeight > maxHeight + 1
        setPopupContentOverflows((current) =>
          current === nextOverflows ? current : nextOverflows,
        )
        if (Math.abs(contentHeight - lastHeight) < 2) {
          return
        }

        lastHeight = contentHeight
        void resizePopupToContent(contentHeight)
      })
    }

    const observer = new ResizeObserver(requestResize)
    observer.observe(popupRootRef.current)
    requestResize()

    return () => {
      window.cancelAnimationFrame(animationFrame)
      observer.disconnect()
    }
  }, [
    config.popup_font_scale,
    config.popup_height,
    isPopupWindow,
    latestResult,
    popupBusy,
    popupError,
    popupQueryText,
    status,
  ])

  useEffect(() => {
    if (isPopupWindow || !isRecordingHotkey) {
      return
    }

    const handleKeydown = (event: KeyboardEvent) => {
      event.preventDefault()
      event.stopPropagation()

      if (event.key === 'Escape') {
        setIsRecordingHotkey(false)
        setStatus('已取消快捷键录制')
        return
      }

      const hotkey = formatHotkeyFromEvent(event)
      if (!hotkey || isModifierKey(event.key)) {
        return
      }

      setConfigDraft((current) => ({
        ...current,
        hotkey,
        trigger_source: 'keyboard',
      }))
      setIsRecordingHotkey(false)
      setStatus(`已识别快捷键：${hotkey}`)
    }

    window.addEventListener('keydown', handleKeydown, true)
    return () => window.removeEventListener('keydown', handleKeydown, true)
  }, [isPopupWindow, isRecordingHotkey])

  useEffect(() => {
    if (isPopupWindow) {
      return
    }

    const handle = window.setTimeout(() => {
      void loadStarred()
    }, 160)

    return () => window.clearTimeout(handle)
  }, [filterType, isPopupWindow, loadStarred, search])

  const selectedEntry = useMemo(
    () => starredEntries.find((entry) => entry.id === selectedEntryId) ?? null,
    [selectedEntryId, starredEntries],
  )

  const saveSettings = useCallback(async () => {
    setSavingConfig(true)
    try {
      const saved = await saveConfig(configDraft)
      setConfig(saved)
      setConfigDraft(saved)
      setIsRecordingHotkey(false)
      await setupHotkey(saved)
      setStatus('设置已自动保存')
    } catch (error) {
      setStatus(error instanceof Error ? error.message : String(error))
    } finally {
      setSavingConfig(false)
    }
  }, [configDraft, setupHotkey])

  useEffect(() => {
    if (isPopupWindow || isRecordingHotkey) {
      return
    }

    if (JSON.stringify(configDraft) === JSON.stringify(config)) {
      return
    }

    const handle = window.setTimeout(() => {
      void saveSettings()
    }, 500)

    return () => window.clearTimeout(handle)
  }, [config, configDraft, isPopupWindow, isRecordingHotkey, saveSettings])

  const runManualLookup = useCallback(async () => {
    setLookupBusy(true)
    try {
      const result = await manualLookup(testInput)
      setLatestResult(result)
      setPopupBusy(false)
      setPopupError('')
      setPopupQueryText(testInput)
      setStatus('')
    } catch (error) {
      setStatus(error instanceof Error ? error.message : String(error))
    } finally {
      setLookupBusy(false)
    }
  }, [testInput])

  const toggleLatestStarred = useCallback(async () => {
    if (!latestResult) {
      return
    }

    try {
      const response = await toggleStarred(latestResult.payload)
      setLatestResult({
        ...latestResult,
        is_starred: response.is_starred,
      })
      if (!isPopupWindow) {
        await loadStarred()
      }
    } catch (error) {
      setStatus(error instanceof Error ? error.message : String(error))
    }
  }, [isPopupWindow, latestResult, loadStarred])

  const deleteEntry = useCallback(
    async (id: string) => {
      try {
        await removeStarred(id)
        if (selectedEntryId === id) {
          setSelectedEntryId(null)
        }
        await loadStarred()
      } catch (error) {
        setStatus(error instanceof Error ? error.message : String(error))
      }
    },
    [loadStarred, selectedEntryId],
  )

  if (isPopupWindow) {
    return (
      <div
        ref={popupRootRef}
        className={`window-root popup-root ${popupContentOverflows ? 'popup-overflowing' : ''}`}
        style={{ '--popup-font-scale': config.popup_font_scale } as CSSProperties}
      >
        <LookupPopup
          result={latestResult}
          busy={popupBusy}
          queryText={popupQueryText}
          errorMessage={popupError}
          onToggleStarred={toggleLatestStarred}
          onClose={hideCurrentWindow}
          onStartDrag={startPopupDrag}
        />
        {status ? <div className="status-banner popup-status">{status}</div> : null}
      </div>
    )
  }

  return (
    <div className="window-root">
      <DashboardView
        config={config}
        configDraft={configDraft}
        isRecordingHotkey={isRecordingHotkey}
        onStartHotkeyCapture={() => {
          setIsRecordingHotkey(true)
          setStatus('请直接按下新的快捷键，按 Esc 取消')
        }}
        onStopHotkeyCapture={() => {
          setIsRecordingHotkey(false)
          setStatus('已取消快捷键录制')
        }}
        onConfigDraftChange={(nextConfig) => {
          if (nextConfig.trigger_source !== 'keyboard' && isRecordingHotkey) {
            setIsRecordingHotkey(false)
          }
          setConfigDraft(nextConfig)
        }}
        testInput={testInput}
        onTestInputChange={setTestInput}
        onManualLookup={runManualLookup}
        latestResult={latestResult}
        starredEntries={starredEntries}
        selectedEntryId={selectedEntry?.id ?? null}
        onSelectEntry={setSelectedEntryId}
        onDeleteEntry={deleteEntry}
        filterType={filterType}
        onFilterTypeChange={setFilterType}
        search={search}
        onSearchChange={setSearch}
        onToggleLatestStarred={toggleLatestStarred}
        savingConfig={savingConfig}
        lookupBusy={lookupBusy}
      />
      {status ? <div className="status-banner">{status}</div> : null}
    </div>
  )
}

export default App
