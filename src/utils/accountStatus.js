export function isBannedStatus(status) {
  return status === 'banned' || status === '封禁' || status === '已封禁'
}

export function isUnavailableStatus(status) {
  return isBannedStatus(status) || status === '已过期'
}
