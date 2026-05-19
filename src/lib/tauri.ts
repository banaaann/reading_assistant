import { invoke } from '@tauri-apps/api/core'
import { getCurrentWindow } from '@tauri-apps/api/window'
import type {
  AppConfig,
  LookupResult,
  StarredEntry,
  StarredQuery,
  ToggleStarredResponse,
} from './types'

export const isTauriRuntime =
  typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window

export const appWindow = isTauriRuntime
  ? getCurrentWindow()
  : ({
      label: 'main',
      hide: async () => {},
      setAlwaysOnTop: async () => {},
    } as unknown as ReturnType<typeof getCurrentWindow>)

const BROWSER_CONFIG: AppConfig = {
  hotkey: 'Ctrl+Shift+D',
  trigger_source: 'keyboard',
  auto_start: false,
  max_text_length: 280,
  close_on_focus_loss: true,
  popup_width: 380,
  popup_height: 400,
  popup_font_scale: 0.92,
}

export function getConfig() {
  if (!isTauriRuntime) {
    return Promise.resolve(BROWSER_CONFIG)
  }
  return invoke<AppConfig>('get_config')
}

export function saveConfig(config: AppConfig) {
  if (!isTauriRuntime) {
    return Promise.resolve(config)
  }
  return invoke<AppConfig>('save_config', { config })
}

export function triggerLookup() {
  return invoke<void>('trigger_lookup')
}

export function manualLookup(text: string) {
  return invoke<LookupResult>('manual_lookup', { request: { text } })
}

export function listStarred(query?: StarredQuery) {
  if (!isTauriRuntime) {
    void query
    return Promise.resolve([])
  }
  return invoke<StarredEntry[]>('list_starred', { query })
}

export function toggleStarred(payload: LookupResult['payload']) {
  return invoke<ToggleStarredResponse>('toggle_starred', { payload })
}

export function removeStarred(id: string) {
  return invoke<void>('remove_starred', { id })
}

export function showMainWindow() {
  return invoke<void>('show_main_window')
}

export function hideCurrentWindow() {
  return appWindow.hide()
}

export function startPopupDrag() {
  return invoke<void>('start_popup_drag')
}

export function resizePopupToContent(contentHeight: number) {
  if (!isTauriRuntime) {
    return Promise.resolve()
  }
  return invoke<void>('resize_popup_to_content', { contentHeight })
}
