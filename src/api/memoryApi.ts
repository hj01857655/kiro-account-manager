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

export function listMemories() {
  return invoke<MemoryItem[]>('list_memories')
}

export function deleteMemory(id: string) {
  return invoke<void>('delete_memory', { id })
}

export function getMemoryMode() {
  return invoke<MemoryModeConfig>('get_memory_mode')
}
