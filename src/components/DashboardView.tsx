import { Search, Star, Trash2 } from 'lucide-react'
import type { CSSProperties } from 'react'
import type { AppConfig, LookupResult, StarredEntry, TriggerSource } from '../lib/types'

type DashboardViewProps = {
  config: AppConfig
  configDraft: AppConfig
  isRecordingHotkey: boolean
  onStartHotkeyCapture: () => void
  onStopHotkeyCapture: () => void
  onConfigDraftChange: (config: AppConfig) => void
  testInput: string
  onTestInputChange: (value: string) => void
  onManualLookup: () => Promise<void> | void
  latestResult: LookupResult | null
  starredEntries: StarredEntry[]
  selectedEntryId: string | null
  onSelectEntry: (id: string) => void
  onDeleteEntry: (id: string) => Promise<void> | void
  filterType: string
  onFilterTypeChange: (value: string) => void
  search: string
  onSearchChange: (value: string) => void
  onToggleLatestStarred: () => Promise<void> | void
  savingConfig: boolean
  lookupBusy: boolean
}

const TRIGGER_OPTIONS: Array<{ value: TriggerSource; label: string }> = [
  { value: 'keyboard', label: '键盘快捷键' },
  { value: 'mouse_middle', label: '鼠标中键' },
  { value: 'mouse_x1', label: '鼠标侧键 1' },
  { value: 'mouse_x2', label: '鼠标侧键 2' },
]

export function DashboardView({
  config,
  configDraft,
  isRecordingHotkey,
  onStartHotkeyCapture,
  onStopHotkeyCapture,
  onConfigDraftChange,
  testInput,
  onTestInputChange,
  onManualLookup,
  latestResult,
  starredEntries,
  selectedEntryId,
  onSelectEntry,
  onDeleteEntry,
  filterType,
  onFilterTypeChange,
  search,
  onSearchChange,
  onToggleLatestStarred,
  savingConfig,
  lookupBusy,
}: DashboardViewProps) {
  const selectedEntry =
    starredEntries.find((entry) => entry.id === selectedEntryId) ?? starredEntries[0] ?? null
  const detail = selectedEntry?.payload ?? latestResult?.payload ?? null

  return (
    <div className="dashboard-shell">
      <header className="hero-strip">
        <div>
          <p className="eyebrow">Reading Assistant Pro</p>
          <h1>本地阅读助手</h1>
          <p className="hero-copy">
            在任意阅读界面选中文本后，用键盘快捷键或鼠标触发查词。默认优先联网查询与翻译，失败时回退到本地词库。
          </p>
        </div>
        <div className="hero-metrics">
          <div>
            <span>当前触发方式</span>
            <strong>
              {TRIGGER_OPTIONS.find((item) => item.value === config.trigger_source)?.label ??
                '键盘快捷键'}
            </strong>
          </div>
          <div>
            <span>星标总数</span>
            <strong>{starredEntries.length}</strong>
          </div>
        </div>
      </header>

      <main className="dashboard-grid">
        <section className="panel">
          <div className="panel-header">
            <div>
              <h2>设置</h2>
              <p>{savingConfig ? '正在自动保存设置' : '调整后会自动保存并生效'}</p>
            </div>
          </div>

          <div className="form-grid">
            <div className="settings-row settings-row-2">
              <label>
                <span>触发</span>
                <select
                  value={configDraft.trigger_source}
                  onChange={(event) =>
                    onConfigDraftChange({
                      ...configDraft,
                      trigger_source: event.target.value as TriggerSource,
                    })
                  }
                >
                  {TRIGGER_OPTIONS.map((option) => (
                    <option key={option.value} value={option.value}>
                      {option.label}
                    </option>
                  ))}
                </select>
              </label>

              <label>
                <span>快捷键</span>
                <button
                  type="button"
                  className={`hotkey-capture-button ${isRecordingHotkey ? 'recording' : ''}`}
                  onClick={() => {
                    if (configDraft.trigger_source !== 'keyboard') {
                      return
                    }
                    if (isRecordingHotkey) {
                      onStopHotkeyCapture()
                    } else {
                      onStartHotkeyCapture()
                    }
                  }}
                  disabled={configDraft.trigger_source !== 'keyboard'}
                >
                  {configDraft.trigger_source !== 'keyboard'
                    ? '鼠标触发'
                    : isRecordingHotkey
                      ? '按下新快捷键'
                      : configDraft.hotkey}
                </button>
              </label>
            </div>

            <div className="settings-row settings-row-3 dense-number-row">
              <label>
                <span>取词长度</span>
                <input
                  type="number"
                  min={40}
                  max={1000}
                  value={configDraft.max_text_length}
                  onChange={(event) =>
                    onConfigDraftChange({
                      ...configDraft,
                      max_text_length: Number(event.target.value || 280),
                    })
                  }
                />
              </label>

              <label>
                <span>宽度</span>
                <input
                  type="number"
                  min={300}
                  max={520}
                  step={10}
                  value={configDraft.popup_width}
                  onChange={(event) =>
                    onConfigDraftChange({
                      ...configDraft,
                      popup_width: Number(event.target.value || 380),
                    })
                  }
                />
              </label>

              <label>
                <span>高度</span>
                <input
                  type="number"
                  min={120}
                  max={680}
                  step={10}
                  value={configDraft.popup_height}
                  onChange={(event) =>
                    onConfigDraftChange({
                      ...configDraft,
                      popup_height: Number(event.target.value || 400),
                    })
                  }
                />
              </label>
            </div>

            <div className="font-scale-row">
              <label>
                <span>字体</span>
                <input
                  type="range"
                  min={0.8}
                  max={1.15}
                  step={0.05}
                  value={configDraft.popup_font_scale}
                  onChange={(event) =>
                    onConfigDraftChange({
                      ...configDraft,
                      popup_font_scale: Number(event.target.value || 0.92),
                    })
                  }
                />
              </label>
              <strong>{Math.round(configDraft.popup_font_scale * 100)}%</strong>
            </div>

            <div className="settings-toggles">
              <label className="checkbox-row">
                <input
                  type="checkbox"
                  checked={configDraft.close_on_focus_loss}
                  onChange={(event) =>
                    onConfigDraftChange({
                      ...configDraft,
                      close_on_focus_loss: event.target.checked,
                    })
                  }
                />
                <span>失焦关闭</span>
              </label>

              <label className="checkbox-row">
                <input
                  type="checkbox"
                  checked={configDraft.auto_start}
                  onChange={(event) =>
                    onConfigDraftChange({
                      ...configDraft,
                      auto_start: event.target.checked,
                    })
                  }
                />
                <span>开机启动</span>
              </label>
            </div>
          </div>

          <div className="popup-preview-panel">
            <div className="popup-preview-meta">
              <span>弹窗预览</span>
              <strong>
                {configDraft.popup_width} × {configDraft.popup_height} ·{' '}
                {Math.round(configDraft.popup_font_scale * 100)}%
              </strong>
            </div>
            <div
              className="popup-preview"
              style={
                {
                  width: `${Math.round(configDraft.popup_width * 0.58)}px`,
                  height: `${Math.round(configDraft.popup_height * 0.58)}px`,
                  fontSize: `${14 * configDraft.popup_font_scale}px`,
                } as CSSProperties
              }
            >
              <div className="popup-preview-head">
                <span>单词释义</span>
                <strong>vocabulary</strong>
              </div>
              <div className="popup-preview-body">
                <span className="pos-chip">名词</span>
                <p>词汇；词汇量</p>
                <p className="muted">The words a reader understands.</p>
              </div>
            </div>
          </div>
        </section>

        <section className="panel">
          <div className="panel-header">
            <div>
              <h2>手动测试</h2>
              <p>这里不依赖全局触发，适合单独验证联网查词、翻译和星标逻辑。</p>
            </div>
            {latestResult ? (
              <button
                type="button"
                className={`icon-text-button ${latestResult.is_starred ? 'active' : ''}`}
                onClick={() => void onToggleLatestStarred()}
              >
                <Star size={16} fill={latestResult.is_starred ? 'currentColor' : 'none'} />
                {latestResult.is_starred ? '已星标' : '加入星标'}
              </button>
            ) : null}
          </div>

          <div className="manual-lookup">
            <textarea
              value={testInput}
              onChange={(event) => onTestInputChange(event.target.value)}
              placeholder="输入单词、短语或句子"
            />
            <button
              type="button"
              className="primary-button"
              onClick={() => void onManualLookup()}
              disabled={lookupBusy}
            >
              开始查询
            </button>
          </div>
        </section>

        <section className="panel panel-span-2">
          <div className="panel-header split">
            <div>
              <h2>星标词汇</h2>
              <p>支持搜索、按类型筛选、删除，并查看保存下来的释义快照。</p>
            </div>
            <div className="toolbar">
              <label className="search-input">
                <Search size={16} />
                <input
                  value={search}
                  onChange={(event) => onSearchChange(event.target.value)}
                  placeholder="搜索星标词汇"
                />
              </label>
              <select value={filterType} onChange={(event) => onFilterTypeChange(event.target.value)}>
                <option value="">全部</option>
                <option value="word">单词</option>
                <option value="phrase">短语</option>
                <option value="sentence">句子</option>
              </select>
            </div>
          </div>

          <div className="starred-layout">
            <div className="starred-list">
              {starredEntries.length ? (
                starredEntries.map((entry) => (
                  <article
                    key={entry.id}
                    className={`starred-item ${entry.id === selectedEntry?.id ? 'selected' : ''}`}
                    onClick={() => onSelectEntry(entry.id)}
                  >
                    <div>
                      <span className="type-chip">
                        {entry.entry_type === 'word'
                          ? '单词'
                          : entry.entry_type === 'phrase'
                            ? '短语'
                            : '句子'}
                      </span>
                      <strong>{entry.display_title}</strong>
                    </div>
                    <div className="starred-meta">
                      <time>{new Date(entry.updated_at).toLocaleString()}</time>
                      <button
                        type="button"
                        className="icon-button subtle"
                        onClick={(event) => {
                          event.stopPropagation()
                          void onDeleteEntry(entry.id)
                        }}
                        title="删除星标"
                      >
                        <Trash2 size={14} />
                      </button>
                    </div>
                  </article>
                ))
              ) : (
                <div className="empty-state">暂无星标词汇</div>
              )}
            </div>

            <div className="detail-panel">
              {detail ? (
                detail.kind === 'word' ? (
                  <>
                    <div className="detail-head">
                      <p className="eyebrow">单词</p>
                      <h3>{detail.lemma}</h3>
                      {detail.phonetic ? <p className="phonetic">{detail.phonetic}</p> : null}
                    </div>
                    {detail.senses.map((sense) => (
                      <article className="definition-group" key={sense.part_of_speech}>
                        <span className="pos-chip">{sense.part_of_speech}</span>
                        <ol>
                          {sense.definitions.map((definition) => (
                            <li key={definition}>{definition}</li>
                          ))}
                        </ol>
                      </article>
                    ))}
                  </>
                ) : (
                  <>
                    <div className="detail-head">
                      <p className="eyebrow">{detail.kind === 'sentence' ? '句子' : '短语'}</p>
                      <h3>{detail.source_text}</h3>
                    </div>
                    <article className="translation-block">
                      <p className="section-label">翻译</p>
                      <p className="translation-text">{detail.translation}</p>
                    </article>
                  </>
                )
              ) : (
                <div className="empty-state">先执行一次查询，或从左侧选择星标词条</div>
              )}
            </div>
          </div>
        </section>
      </main>
    </div>
  )
}
