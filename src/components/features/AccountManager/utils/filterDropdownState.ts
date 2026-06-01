const STATUS_LABELS = {
  normal: 'Normal',
  capped: 'Capped',
  banned: 'Banned',
  invalid: 'Invalid',
  expired: 'Expired'}

const SPECIAL_GROUP_LABELS = {
  __none__: 'No group',
  __has__: 'Has group'}

const SPECIAL_TAG_LABELS = {
  __none__: 'No tags',
  __has__: 'Has tags'}

const translate = (t, key, fallback, options = undefined) => {
  if (typeof t !== 'function') return fallback
  return t(key, options) || fallback
}

function pickFirst(values) {
  return Array.isArray(values) ? values[0] || '' : values || ''
}

export function resolveGroupFilterLabel(selectedGroup, allGroups = [], t = undefined) {
  if (!selectedGroup) return ''

  const groupMap = new Map((Array.isArray(allGroups) ? allGroups : []).map(group => [group.id, group]))
  const specialLabels = {
    __none__: translate(t, 'groups.noGroup', SPECIAL_GROUP_LABELS.__none__),
    __has__: translate(t, 'groups.hasGroup', SPECIAL_GROUP_LABELS.__has__)}
  return specialLabels[selectedGroup] || groupMap.get(selectedGroup)?.name || selectedGroup
}

export function countActiveFilters({ filters, selectedGroup, selectedTag }) {
  return [
    filters?.subscriptions?.length || 0,
    filters?.statuses?.length || 0,
    filters?.providers?.length || 0,
    filters?.usageRange ? 1 : 0,
    selectedGroup ? 1 : 0,
    selectedTag ? 1 : 0,
  ].reduce((total, count) => total + count, 0)
}

export function buildFilterSummaryItems({
  filters,
  selectedGroup,
  selectedTag,
  allGroups = [],
  allTags = [],
  t = undefined}) {
  const items = []
  const tagMap = new Map((Array.isArray(allTags) ? allTags : []).map(tag => [tag.id, tag]))
  const statusLabels = {
    normal: translate(t, 'accounts.active', STATUS_LABELS.normal),
    capped: translate(t, 'accounts.capped', STATUS_LABELS.capped),
    banned: translate(t, 'accounts.banned', STATUS_LABELS.banned),
    invalid: translate(t, 'accounts.invalid', STATUS_LABELS.invalid),
    expired: translate(t, 'accounts.expired', STATUS_LABELS.expired)}
  const tagSpecialLabels = {
    __none__: translate(t, 'tags.noTags', SPECIAL_TAG_LABELS.__none__),
    __has__: translate(t, 'tags.hasTags', SPECIAL_TAG_LABELS.__has__)}

  if (selectedGroup) {
    items.push({
      key: 'group',
      label: translate(t, 'groups.title', 'Groups'),
      value: resolveGroupFilterLabel(selectedGroup, allGroups, t)})
  }

  if (selectedTag) {
    items.push({
      key: 'tag',
      label: translate(t, 'tags.title', 'Tags'),
      value: tagSpecialLabels[selectedTag] || tagMap.get(selectedTag)?.name || selectedTag})
  }

  const subscription = pickFirst(filters?.subscriptions)
  if (subscription) {
    items.push({ key: 'subscription', label: translate(t, 'accounts.subscription', 'Subscription'), value: subscription })
  }

  const status = pickFirst(filters?.statuses)
  if (status) {
    items.push({ key: 'status', label: translate(t, 'accounts.status', 'Status'), value: statusLabels[status] || status })
  }

  const provider = pickFirst(filters?.providers)
  if (provider) {
    items.push({ key: 'provider', label: translate(t, 'accounts.provider', 'Provider'), value: provider })
  }

  if (filters?.usageRange) {
    items.push({ key: 'usageRange', label: translate(t, 'filter.usage', 'Usage'), value: filters.usageRange })
  }

  return items
}
