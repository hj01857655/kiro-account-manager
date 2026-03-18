import { invoke } from '@tauri-apps/api/core'

export async function applyMachineGuid(account, settings = {}) {
  const autoChangeMachineId = settings.autoChangeMachineId !== false
  const bindMachineIdToAccount = settings.bindMachineIdToAccount !== false

  if (!autoChangeMachineId) return account

  try {
    if (bindMachineIdToAccount) {
      let machineId = account.machineId

      if (!machineId) {
        machineId = await invoke('generate_machine_guid')
        await invoke('update_account', { id: account.id, machineId })
        return await setCustomMachineGuid(account, machineId)
      }

      return await setCustomMachineGuid(account, machineId)
    }

    const newMachineId = await invoke('generate_machine_guid')
    await invoke('set_custom_machine_guid', { newGuid: newMachineId })
  } catch {
    // 机器码操作失败不阻断切换流程
  }

  return account
}

async function setCustomMachineGuid(account, machineId) {
  await invoke('set_custom_machine_guid', { newGuid: machineId })
  return { ...account, machineId }
}

export function buildSwitchParams(account) {
  const isIdC = account.provider === 'BuilderId' || account.provider === 'Enterprise' || account.clientIdHash
  const authMethod = isIdC ? 'IdC' : 'social'

  const params = {
    accessToken: account.accessToken,
    refreshToken: account.refreshToken,
    provider: account.provider || 'Google',
    authMethod
  }

  if (isIdC) {
    params.region = account.region || 'us-east-1'
    params.clientId = account.clientId
    params.clientSecret = account.clientSecret

    if (account.provider === 'Enterprise') {
      params.startUrl = account.startUrl
    }
  } else {
    params.profileArn = account.profileArn || 'arn:aws:codewhisperer:us-east-1:699475941385:profile/EHGA3GRVQMUK'
  }

  return params
}
