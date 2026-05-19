import { useEffect, useMemo, useState } from 'react'
import type { PointerEvent } from 'react'
import { LoaderCircle, Star, X } from 'lucide-react'
import { manualLookup } from '../lib/tauri'
import type { LookupResult, WordLookupPayload } from '../lib/types'

type LookupPopupProps = {
  result: LookupResult | null
  busy: boolean
  queryText: string
  errorMessage: string
  onToggleStarred: () => Promise<void> | void
  onClose: () => Promise<void> | void
  onStartDrag: () => Promise<void> | void
}

type TokenLookupState = {
  loading: boolean
  payload?: WordLookupPayload
  error?: string
}

type TokenChip = {
  id: string
  label: string
  normalized: string
}

function extractWordTokens(text: string): TokenChip[] {
  const matches = [...text.matchAll(/[A-Za-z']+/g)]
  return matches.map((match, index) => ({
    id: `${match.index ?? index}-${index}`,
    label: match[0],
    normalized: match[0].toLowerCase(),
  }))
}

export function LookupPopup({
  result,
  busy,
  queryText,
  errorMessage,
  onToggleStarred,
  onClose,
  onStartDrag,
}: LookupPopupProps) {
  const [expandedWordIds, setExpandedWordIds] = useState<string[]>([])
  const [wordLookups, setWordLookups] = useState<Record<string, TokenLookupState>>({})

  useEffect(() => {
    setExpandedWordIds([])
    setWordLookups({})
  }, [result?.payload.normalized_text])

  const titlebarProps = {
    'data-tauri-drag-region': true,
    onPointerDown: (event: PointerEvent<HTMLDivElement>) => {
      if (event.button === 0) {
        void onStartDrag()
      }
    },
  }

  const sentenceTokens = useMemo(() => {
    if (!result || result.payload.kind === 'word') {
      return []
    }

    return extractWordTokens(result.payload.source_text)
  }, [result])

  const expandedTokens = useMemo(
    () => sentenceTokens.filter((token) => expandedWordIds.includes(token.id)),
    [expandedWordIds, sentenceTokens],
  )

  const toggleWordLookup = async (token: TokenChip) => {
    const isExpanded = expandedWordIds.includes(token.id)

    if (isExpanded) {
      setExpandedWordIds((current) => current.filter((item) => item !== token.id))
      return
    }

    setExpandedWordIds((current) => [...current, token.id])

    if (wordLookups[token.normalized]?.payload || wordLookups[token.normalized]?.loading) {
      return
    }

    setWordLookups((current) => ({
      ...current,
      [token.normalized]: { loading: true },
    }))

    try {
      const lookupResult = await manualLookup(token.normalized)
      if (lookupResult.payload.kind !== 'word') {
        throw new Error('未能取得单词释义')
      }
      const wordPayload: WordLookupPayload = lookupResult.payload

      setWordLookups((current) => ({
        ...current,
        [token.normalized]: {
          loading: false,
          payload: wordPayload,
        },
      }))
    } catch (error) {
      setWordLookups((current) => ({
        ...current,
        [token.normalized]: {
          loading: false,
          error: error instanceof Error ? error.message : String(error),
        },
      }))
    }
  }

  if (busy) {
    return (
      <div className="popup-shell">
        <header className="popup-header">
          <div className="popup-titlebar" {...titlebarProps}>
            <p className="eyebrow">正在查询</p>
            <h1>{queryText || '正在读取选中文本'}</h1>
          </div>
          <div className="popup-actions">
            <button type="button" className="icon-button" onClick={() => void onClose()} title="关闭">
              <X size={16} />
            </button>
          </div>
        </header>
        <div className="popup-loading">
          <LoaderCircle size={18} className="spinning" />
          <span>窗口已弹出，正在联网查询并整理释义。</span>
        </div>
      </div>
    )
  }

  if (!result) {
    return (
      <div className="popup-shell empty">
        <header className="popup-header">
          <div className="popup-titlebar" {...titlebarProps}>
            <p className="eyebrow">{errorMessage ? '查询失败' : '等待查询'}</p>
            <h1>{queryText || 'Reading Assistant Pro'}</h1>
          </div>
          <div className="popup-actions">
            <button type="button" className="icon-button" onClick={() => void onClose()} title="关闭">
              <X size={16} />
            </button>
          </div>
        </header>
        <div className="popup-empty">{errorMessage || '等待划词后触发查询'}</div>
      </div>
    )
  }

  const { payload, is_starred: isStarred } = result

  return (
    <div className="popup-shell">
      <div className="popup-floating-actions">
        <button
          type="button"
          className={`icon-button ${isStarred ? 'active' : ''}`}
          onClick={() => void onToggleStarred()}
          title={isStarred ? '取消星标' : '加入星标'}
        >
          <Star size={16} fill={isStarred ? 'currentColor' : 'none'} />
        </button>
        <button type="button" className="icon-button" onClick={() => void onClose()} title="关闭">
          <X size={16} />
        </button>
      </div>

      <header className="popup-header popup-header-stacked">
        <div className="popup-titlebar popup-titlebar-wide" {...titlebarProps}>
          <p className="eyebrow">
            {payload.kind === 'word' ? '单词释义' : '短语 / 句子 · 逐词查看'}
          </p>
          <h1>{payload.kind === 'word' ? payload.lemma : payload.source_text}</h1>
          {payload.kind === 'word' && payload.phonetic ? (
            <p className="phonetic">{payload.phonetic}</p>
          ) : null}
        </div>
      </header>

      {payload.kind === 'word' ? (
        <section className="definition-stack">
          {payload.senses.map((sense) => (
            <article className="definition-group" key={sense.part_of_speech}>
              <span className="pos-chip">{sense.part_of_speech}</span>
              <ol>
                {sense.definitions.map((definition) => (
                  <li key={definition}>{definition}</li>
                ))}
              </ol>
            </article>
          ))}
        </section>
      ) : (
        <section className="definition-stack">
          <article className="translation-block">
            <p className="section-label">整句释义</p>
            <p className="translation-text">{payload.translation}</p>
          </article>

          <article className="keyword-block">
            <div className="token-chip-list">
              {sentenceTokens.map((token) => {
                const selected = expandedWordIds.includes(token.id)

                return (
                  <button
                    key={token.id}
                    type="button"
                    className={`token-chip ${selected ? 'selected' : ''}`}
                    onClick={() => void toggleWordLookup(token)}
                  >
                    {token.label}
                  </button>
                )
              })}
            </div>
          </article>

          {expandedTokens.length ? (
            <section className="definition-stack">
              {expandedTokens.map((token) => {
                const lookupState = wordLookups[token.normalized]

                if (!lookupState || lookupState.loading) {
                  return (
                    <article className="definition-group" key={token.id}>
                      <div className="inline-loading">
                        <LoaderCircle size={16} className="spinning" />
                        <span>{token.label} 正在查询</span>
                      </div>
                    </article>
                  )
                }

                if (lookupState.error || !lookupState.payload) {
                  return (
                    <article className="definition-group" key={token.id}>
                      <strong>{token.label}</strong>
                      <p className="muted">{lookupState.error || '未能取得单词释义'}</p>
                    </article>
                  )
                }

                return (
                  <article className="definition-group" key={token.id}>
                    <div className="word-detail-head">
                      <div>
                        <strong>{token.label}</strong>
                        {lookupState.payload.phonetic ? (
                          <p className="phonetic">{lookupState.payload.phonetic}</p>
                        ) : null}
                      </div>
                    </div>
                    {lookupState.payload.senses.map((sense) => (
                      <div className="word-sense-block" key={`${token.id}-${sense.part_of_speech}`}>
                        <span className="pos-chip">{sense.part_of_speech}</span>
                        <ol>
                          {sense.definitions.map((definition) => (
                            <li key={definition}>{definition}</li>
                          ))}
                        </ol>
                      </div>
                    ))}
                  </article>
                )
              })}
            </section>
          ) : null}
        </section>
      )}
    </div>
  )
}
