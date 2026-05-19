export type DictionarySense = {
  part_of_speech: string
  definitions: string[]
}

export type KeywordDefinition = {
  word: string
  part_of_speech?: string | null
  brief_definition: string
}

export type WordLookupPayload = {
  kind: 'word'
  source_text: string
  normalized_text: string
  lemma: string
  phonetic?: string | null
  senses: DictionarySense[]
}

export type PhraseLookupPayload = {
  kind: 'phrase' | 'sentence'
  source_text: string
  normalized_text: string
  translation: string
  keywords: KeywordDefinition[]
}

export type LookupPayload = WordLookupPayload | PhraseLookupPayload

export type LookupResult = {
  payload: LookupPayload
  is_starred: boolean
}

export type TriggerSource = 'keyboard' | 'mouse_middle' | 'mouse_x1' | 'mouse_x2'

export type AppConfig = {
  hotkey: string
  trigger_source: TriggerSource
  auto_start: boolean
  max_text_length: number
  close_on_focus_loss: boolean
  popup_width: number
  popup_height: number
  popup_font_scale: number
}

export type LookupEvent = {
  anchor: {
    x: number
    y: number
  }
  result: LookupResult
}

export type LookupLoadingEvent = {
  anchor: {
    x: number
    y: number
  }
  text: string
}

export type LookupErrorEvent = {
  anchor?: {
    x: number
    y: number
  } | null
  text?: string | null
  message: string
}

export type StarredEntry = {
  id: string
  entry_type: 'word' | 'phrase' | 'sentence' | string
  source_text: string
  normalized_text: string
  display_title: string
  payload: LookupPayload
  created_at: string
  updated_at: string
}

export type StarredQuery = {
  search?: string
  entry_type?: string
}

export type ToggleStarredResponse = {
  is_starred: boolean
  entry?: StarredEntry | null
}
