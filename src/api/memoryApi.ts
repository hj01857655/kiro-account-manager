// Agent Memory 管理 API（~/.kiro/memories/memories.db）
import { invoke } from '@tauri-apps/api/core'

export interface MemoryItem {
  id: string
  memoryType: string
  scopes: string[]
  title: string
  summary: string
  content: string
  why: string
  howToApply: string
  sessionId: string
  createdAt: string
  updatedAt: string
}

export interface MemoryModeConfig {
  mode: string
  reflection: boolean
}

export interface MemoryDraft {
  /** 为空 = 新建，有值 = 更新 */
  id?: string
  memoryType: string
  scopes: string[]
  title: string
  summary: string
  content: string
  why?: string
  howToApply?: string
}

export interface MemoryPage {
  memories: MemoryItem[]
  nextCursor?: string
  /** 当前生效的 scope 过滤（IDE 的 scopeLabel） */
  scopeLabel?: string
}

export function listMemories() {
  return invoke<MemoryItem[]>('list_memories')
}

/** 分页读取，口径对齐 IDE：limit 默认 100、上限 1000，cursor 不透明；scopes 为空则不过滤 */
export function listMemoriesPage(limit?: number, cursor?: string, scopes?: string[]) {
  return invoke<MemoryPage>('list_memories_page', { limit, cursor, scopes })
}

export function deleteMemory(id: string) {
  return invoke<void>('delete_memory', { id })
}

export function saveMemory(draft: MemoryDraft) {
  return invoke<string>('save_memory', { draft })
}

export function getMemoryMode() {
  return invoke<MemoryModeConfig>('get_memory_mode')
}

export interface MemoryPreviewRow {
  title: string
  memoryType: string
  chars: number
  line: string
}

// 注入预览（模拟 Kiro buildMemoryBlock 的 resident/manifest 分层与预算）
export interface MemoryInjectionPreview {
  enabled: boolean
  disabledReason: string | null
  totalBudget: number
  singleMax: number
  recordLimit: number
  prefQuota: number
  resident: MemoryPreviewRow[]
  manifest: MemoryPreviewRow[]
  droppedTooLong: string[]
  renderedChars: number
}

export function previewMemoryInjection() {
  return invoke<MemoryInjectionPreview>('preview_memory_injection')
}
