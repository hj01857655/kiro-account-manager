import test from 'node:test'
import assert from 'node:assert/strict'
import { buildFilterSummaryItems, countActiveFilters, resolveGroupFilterLabel } from './filterDropdownState.js'

test('countActiveFilters counts basic and relation filters', () => {
  const count = countActiveFilters({
    filters: {
      subscriptions: ['KIRO PRO'],
      statuses: ['normal'],
      providers: [],
      usageRange: '500-1000'},
    selectedGroup: 'group-1',
    selectedTag: '__has__'})

  assert.equal(count, 5)
})

test('buildFilterSummaryItems resolves readable labels', () => {
  const items = buildFilterSummaryItems({
    filters: {
      subscriptions: ['KIRO PRO'],
      statuses: ['normal'],
      providers: ['Google'],
      usageRange: '500-1000'},
    selectedGroup: 'group-1',
    selectedTag: 'tag-1',
    allGroups: [{ id: 'group-1', name: '主力组' }],
    allTags: [{ id: 'tag-1', name: '高频' }]})

  assert.deepEqual(items, [
    { key: 'group', label: 'Groups', value: '主力组' },
    { key: 'tag', label: 'Tags', value: '高频' },
    { key: 'subscription', label: 'Subscription', value: 'KIRO PRO' },
    { key: 'status', label: 'Status', value: 'Active' },
    { key: 'provider', label: 'Provider', value: 'Google' },
    { key: 'usageRange', label: 'Usage', value: '500-1000' },
  ])
})

test('buildFilterSummaryItems handles special pseudo values', () => {
  const items = buildFilterSummaryItems({
    filters: {
      subscriptions: [],
      statuses: [],
      providers: [],
      usageRange: null},
    selectedGroup: '__none__',
    selectedTag: '__has__',
    allGroups: [],
    allTags: []})

  assert.deepEqual(items, [
    { key: 'group', label: 'Groups', value: 'No group' },
    { key: 'tag', label: 'Tags', value: 'Has tags' },
  ])
})

test('resolveGroupFilterLabel resolves normal and pseudo group values', () => {
  assert.equal(
    resolveGroupFilterLabel('group-1', [{ id: 'group-1', name: '主力组' }]),
    '主力组'
  )
  assert.equal(resolveGroupFilterLabel('__has__', []), 'Has group')
  assert.equal(resolveGroupFilterLabel(null, []), '')
})
