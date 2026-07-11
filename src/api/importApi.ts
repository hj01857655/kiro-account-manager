// 账号导入相关 API 调用
import { invoke } from '@tauri-apps/api/core'

// 读取本机 Kiro IDE 已登录账号
export function readKiroAccounts() {
  return invoke<any[]>('read_kiro_accounts')
}

// social 账号导入
export function addAccountBySocial(args: {
  refreshToken: string
  provider: string
  machineId?: string | null
  accessToken?: string | null
}) {
  return invoke('add_account_by_social', args)
}

// IdC（BuilderId / Enterprise）账号导入（params 为已构建好的完整参数）
export function addAccountByIdc(params: Record<string, any>) {
  return invoke('add_account_by_idc', params)
}

// external_idp（微软 / Azure AD）账号导入（params 为已构建好的完整参数）
export function addAccountByExternalIdp(params: Record<string, any>) {
  return invoke('add_account_by_external_idp', params)
}

// 从 Kiro CLI 数据库导入账号
export function importFromKiroCli(dbPath: string) {
  return invoke<any>('import_from_kiro_cli', { dbPath })
}
