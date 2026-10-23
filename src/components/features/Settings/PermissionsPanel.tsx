import { useCallback, useEffect, useMemo, useState } from 'react'
import { ShieldCheck, Pencil, Trash2, Plus } from 'lucide-react'
import { Textarea } from '../../ui/textarea'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '../../ui/select'
import { Label } from '../../ui/label'
import { Button } from '../../ui/button'
import { DialogRoot, DialogContent, DialogHeader, DialogTitle, DialogBody, DialogFooter } from '../../shared/dialog'
import SectionCard from './SectionCard'
import {
  getPermissions,
  savePermissions,
  getPermissionCapabilities,
  listPermissionWorkspaceRoots,
} from '../../../api/settingsApi'
import type { WorkspaceRootInfo } from '../../../api/settingsApi'

interface PermissionRule {
  capability: string
  effect: string
  match?: string[] | null
  exclude?: string[] | null
}

interface PermissionPolicy {
  rules: PermissionRule[]
  policies?: string[] | null
}

const EFFECTS = ['allow', 'deny', 'ask'] as const

// effect → 色标（allow 绿 / deny 红 / ask 琥珀）。规则行与弹窗共用，便于一眼区分裁决强度。
const EFFECT_BADGE: Record<string, string> = {
  allow: 'bg-emerald-500/10 text-emerald-600 dark:text-emerald-400',
  deny: 'bg-red-500/10 text-red-600 dark:text-red-400',
  ask: 'bg-amber-500/10 text-amber-600 dark:text-amber-400',
}

// 文本域与数组互转：一行一条规则模式
const lines = (v: string) => v.split('\n').map(s => s.trim()).filter(Boolean)

// 归一化：把内存规则收敛成将要写盘的结构，用于脏检查与保存，保证两者口径一致
const normalize = (rules: PermissionRule[], policies: string): PermissionPolicy => ({
  rules: rules.map(r => ({
    capability: r.capability,
    effect: r.effect,
    match: r.match && r.match.length ? r.match : null,
    exclude: r.exclude && r.exclude.length ? r.exclude : null,
  })),
  policies: lines(policies),
})

// Kiro 内置的只读 shell 命令白名单，按工具分组（据 1.1.14 扩展产物的 read-only-shell 预设）。
// 组名保持英文标识符，与 capability / preset id 一致，免得多语言维护。
const SHELL_COMMAND_GROUPS: { id: string; commands: string[] }[] = [
  {
    id: 'node-bun',
    commands: [
      'node --version', 'node -v', 'npm install *', 'npm ci *', 'npm run *', 'npm test *', 'npm pack *',
      'bun install *', 'bun run *', 'bun test *', 'pnpm *', 'yarn *',
    ],
  },
  {
    id: 'rust',
    commands: ['cargo build *', 'cargo test *', 'cargo run *', 'cargo check *', 'cargo clippy *', 'cargo fmt *'],
  },
  {
    id: 'python',
    commands: [
      'python --version', 'python3 --version', 'pip *', 'pip3 *', 'uv pip *', 'uv venv *', 'uv sync *',
      'uv lock *', 'uv add *', 'uv remove *', 'ruff *', 'pyright *', 'pytest *', 'mypy *', 'black *', 'isort *',
    ],
  },
  {
    id: 'go',
    commands: ['go version', 'go env *', 'go build *', 'go test *', 'go mod *', 'go vet *', 'go fmt *'],
  },
  {
    id: 'frontend',
    commands: ['tsc *', 'eslint *', 'jest *', 'vitest *', 'mocha *', 'prettier *'],
  },
  {
    id: 'github-cli',
    commands: [
      'gh pr *', 'gh issue *', 'gh repo view *', 'gh run *', 'gh workflow *', 'gh status',
      'gh auth status', 'gh search *', 'gh label list *',
    ],
  },
  {
    id: 'gitlab-cli',
    commands: ['glab mr *', 'glab issue *', 'glab repo view *', 'glab ci *', 'glab auth status'],
  },
  {
    id: 'file-ops',
    commands: ['mkdir *', 'touch *', 'mv *', 'cp *', 'ln *'],
  },
  {
    id: 'inspect',
    commands: [
      'echo *', 'printf *', 'ls *', 'head *', 'tail *', 'cat *', 'less *', 'grep *', 'rg *', 'diff *',
      'jq *', 'tr *', 'base64 *', 'sort *', 'uniq *', 'cut *', 'comm *', 'column *', 'wc *',
      'sha256sum *', 'md5sum *', 'realpath *', 'readlink *', 'stat *', 'file *',
    ],
  },
]

const READ_ONLY_SHELL_COMMANDS = SHELL_COMMAND_GROUPS.flatMap(g => g.commands)

// Kiro 内置的三个权限预设（`policies: ["<id>"]` 引用的就是这些 id，这里按 1.1.14 实测
// 展开为等价的 rules）。展开而非写 policies：rules 是本面板的编辑对象，展开后用户可直接微调。
const PRESET_IDS = ['read-all', 'read-only-shell', 'read-workspace'] as const
type PresetId = (typeof PRESET_IDS)[number]

const PRESETS: Record<PresetId, PermissionRule[]> = {
  'read-all': [
    { capability: 'fs_read', effect: 'allow', match: [], exclude: [] },
    { capability: 'web_fetch', effect: 'allow', match: [], exclude: [] },
    { capability: 'web_search', effect: 'allow', match: [], exclude: [] },
  ],
  'read-only-shell': [
    { capability: 'shell', effect: 'allow', match: [...READ_ONLY_SHELL_COMMANDS], exclude: [] },
  ],
  // 官方只描述「限工作区内」，未给出对应 match 语法，故留空由 IDE 按工作区自行裁决。
  'read-workspace': [{ capability: 'fs_read', effect: 'allow', match: [], exclude: [] }],
}

const emptyRule = (capability: string): PermissionRule => ({ capability, effect: 'allow', match: [], exclude: [] })

// 规则指纹：用于追加预设时跳过已存在的重复项，避免连点两次堆出一模一样的规则
const ruleSignature = (r: PermissionRule) =>
  `${r.capability}|${r.effect}|${[...(r.match || [])].sort().join('\u0000')}`

// Kiro IDE 1.0 权限面板。
// IDE 1.0 废弃了旧的 trustedCommands / commandDenylist（kiroAgent.* 键只在首次启动
// 迁移一次后即被忽略），改用基于能力(capability)的 permissions.yaml。本面板直接读写
// 全局 ~/.kiro/settings/permissions.yaml，是 1.0 下管理权限的真相源。
//
// 布局：规则列表放在定高（max-h）滚动区内，新增/编辑走弹窗。这样无论规则多少条，
// 卡片高度都恒定，不会随「新增规则」越加越长。
export default function PermissionsPanel({ t }: { t: (key: string) => string }) {
  const [rules, setRules] = useState<PermissionRule[]>([])
  const [policies, setPolicies] = useState('')
  const [capabilities, setCapabilities] = useState<string[]>([])
  const [loading, setLoading] = useState(true)
  const [saving, setSaving] = useState(false)
  const [baseline, setBaseline] = useState('') // 已保存快照，用于脏检查
  // 弹窗草稿：index 为 null 表示新建；草稿独立于 rules，取消即丢弃
  const [draft, setDraft] = useState<{ index: number | null; rule: PermissionRule } | null>(null)
  // 命令组下拉选完即失效（非受控 Select），靠 nonce 重挂载把显示值刷回 placeholder
  const [groupNonce, setGroupNonce] = useState(0)

  // 作用域：global = ~/.kiro/settings/，project = ~/.kiro/workspace-roots/<workspace-id>/
  const [scope, setScope] = useState<'global' | 'project'>('global')
  const [roots, setRoots] = useState<WorkspaceRootInfo[]>([])
  const [projectPath, setProjectPath] = useState('')

  const scopeArg = useMemo(
    () =>
      scope === 'project' ? { scope: 'project' as const, projectPath } : { scope: 'global' as const },
    [scope, projectPath],
  )

  // 能力下拉与 workspace-root 候选只在挂载时拉一次
  useEffect(() => {
    let mounted = true
    Promise.all([
      getPermissionCapabilities<string[]>().catch(() => [] as string[]),
      listPermissionWorkspaceRoots().catch(() => [] as WorkspaceRootInfo[]),
    ]).then(([caps, list]) => {
      if (!mounted) return
      setCapabilities(
        caps && caps.length
          ? caps
          : [
              'shell',
              'fs_read',
              'fs_write',
              'mcp',
              'subagent',
              'web_fetch',
              'web_search',
              'context',
            ],
      )
      setRoots(list || [])
      // 默认选中第一个「已知项目路径」的 root，省得用户再挑一次
      const first = (list || []).find(r => r.projectPath)
      if (first?.projectPath) setProjectPath(first.projectPath)
    })
    return () => {
      mounted = false
    }
  }, [])

  // 按当前作用域加载策略。
  // 项目作用域但还没选路径时**不加载**——后端此时会退化成全局，
  // 直接展示会让人误以为在编辑项目级规则。
  const load = useCallback(async () => {
    if (scope === 'project' && !projectPath) {
      setLoading(false)
      return
    }
    setLoading(true)
    try {
      const policy = await getPermissions<PermissionPolicy>(scopeArg).catch(() => ({
        rules: [] as PermissionRule[],
        policies: [] as string[],
      }))
      const loadedRules = policy.rules || []
      const loadedPolicies = (policy.policies || []).join('\n')
      setRules(loadedRules)
      setPolicies(loadedPolicies)
      setBaseline(JSON.stringify(normalize(loadedRules, loadedPolicies)))
    } finally {
      setLoading(false)
    }
  }, [scope, projectPath, scopeArg, t])

  useEffect(() => {
    void load()
  }, [load])

  const snapshot = useMemo(() => JSON.stringify(normalize(rules, policies)), [rules, policies])
  const dirty = !loading && snapshot !== baseline

  const openNew = () => setDraft({ index: null, rule: emptyRule(capabilities[0] || 'shell') })
  // 深拷贝数组，避免弹窗草稿与列表项共享引用
  const openEdit = (idx: number) =>
    setDraft({
      index: idx,
      rule: {
        ...rules[idx],
        match: [...(rules[idx].match || [])],
        exclude: [...(rules[idx].exclude || [])],
      },
    })
  const closeDraft = () => setDraft(null)
  const commitDraft = () => {
    if (!draft) return
    setRules(rs =>
      draft.index === null ? [...rs, draft.rule] : rs.map((r, i) => (i === draft.index ? draft.rule : r)),
    )
    setDraft(null)
  }
  const patchDraft = (patch: Partial<PermissionRule>) =>
    setDraft(d => (d ? { ...d, rule: { ...d.rule, ...patch } } : d))
  const removeRule = (idx: number) => setRules(rs => rs.filter((_, i) => i !== idx))

  const applyPreset = (id: PresetId) =>
    setRules(rs => {
      const existing = new Set(rs.map(ruleSignature))
      return [
        ...rs,
        ...PRESETS[id]
          .filter(r => !existing.has(ruleSignature(r)))
          .map(r => ({ ...r, match: [...(r.match || [])], exclude: [...(r.exclude || [])] })),
      ]
    })

  // 把一整组只读命令追加进草稿的 match（去重，保持已有顺序）
  const appendShellGroup = (groupId: string) => {
    if (!draft) return
    const commands = SHELL_COMMAND_GROUPS.find(g => g.id === groupId)?.commands ?? []
    patchDraft({ match: Array.from(new Set([...(draft.rule.match || []), ...commands])) })
    setGroupNonce(n => n + 1)
  }

  const handleSave = async () => {
    setSaving(true)
    try {
      await savePermissions(normalize(rules, policies), scopeArg)
      setBaseline(snapshot)
    } catch (err) {
      console.error('[PermissionsPanel] 保存失败:', err)
      await window.alert(t('settings.permissionSaveError'))
    } finally {
      setSaving(false)
    }
  }

  return (
    <SectionCard
      title={t('settings.permissions')}
      accent="red"
      icon={<ShieldCheck size={14} className="text-red-500" />}
      desc={t('settings.permissionsDesc')}
      badge={
        !loading ? (
          <span className="rounded-full bg-muted px-1.5 py-0.5 text-[10px] text-muted-foreground">
            {rules.length}
          </span>
        ) : undefined
      }
    >
      {loading ? (
        <p className="text-[11px] text-muted-foreground">{t('settings.loading')}</p>
      ) : (
        <div className="space-y-3">
          {/* 作用域切换：全局 ~/.kiro/settings  /  项目 ~/.kiro/workspace-roots/<id> */}
          <div className="flex flex-wrap items-center gap-2">
            <div className="inline-flex shrink-0 overflow-hidden rounded-md border border-border">
              {(['global', 'project'] as const).map(s => (
                <button
                  key={s}
                  onClick={() => setScope(s)}
                  className={`px-2.5 py-1 text-[11px] cursor-pointer transition-colors ${
                    scope === s
                      ? 'bg-primary text-primary-foreground'
                      : 'bg-transparent text-muted-foreground hover:bg-muted/50'
                  }`}
                >
                  {t(`settings.permissionScope_${s}`)}
                </button>
              ))}
            </div>

            {scope === 'project' &&
              (roots.length > 0 ? (
                <div className="min-w-0 flex-1">
                  <Select value={projectPath} onValueChange={setProjectPath}>
                    <SelectTrigger className="h-7 w-full text-[11px]">
                      <SelectValue placeholder={t('settings.permissionPickProject')} />
                    </SelectTrigger>
                    <SelectContent>
                      {roots.map(r => (
                        <SelectItem key={r.id} value={r.projectPath || r.id}>
                          <span className="font-mono text-[11px]">{r.projectPath || r.id}</span>
                        </SelectItem>
                      ))}
                    </SelectContent>
                  </Select>
                </div>
              ) : (
                <input
                  value={projectPath}
                  onChange={e => setProjectPath(e.target.value)}
                  placeholder={t('settings.permissionProjectPathPlaceholder')}
                  className="h-7 min-w-0 flex-1 rounded-md border border-border bg-transparent px-2 text-[11px] text-foreground outline-none"
                />
              ))}
          </div>

          {scope === 'project' && roots.length === 0 && (
            <p className="text-[11px] text-muted-foreground">
              {t('settings.permissionNoWorkspaceRoots')}
            </p>
          )}

          {scope === 'project' && projectPath && (
            <p className="font-mono text-[10px] break-all text-muted-foreground">
              {t('settings.permissionTargetPath')}: ~/.kiro/workspace-roots/
              {roots.find(r => r.projectPath === projectPath)?.id ?? '…'}/permissions.yaml
            </p>
          )}

          {/* 定高滚动列表：卡片高度不随规则条数增长 */}
          <div className="rounded-lg border border-border overflow-hidden">
            <div className="max-h-[280px] overflow-y-auto">
              {rules.length === 0 ? (
                <p className="px-3 py-6 text-center text-[11px] text-muted-foreground">
                  {t('settings.permissionEmpty')}
                </p>
              ) : (
                <div className="p-1.5 space-y-1">
                  {rules.map((rule, idx) => (
                    <div
                      key={idx}
                      className="flex items-center gap-2 px-2 py-1.5 rounded-md hover:bg-muted/50"
                    >
                      <span
                        className={`shrink-0 rounded px-1.5 py-0.5 text-[10px] font-medium ${
                          EFFECT_BADGE[rule.effect] || EFFECT_BADGE.ask
                        }`}
                      >
                        {t(`settings.permissionEffect_${rule.effect}`)}
                      </span>
                      <span className="shrink-0 text-xs font-medium text-foreground">{rule.capability}</span>
                      <span className="min-w-0 flex-1 truncate font-mono text-[11px] text-muted-foreground">
                        {(rule.match || []).join('  ') || '—'}
                      </span>
                      {rule.exclude && rule.exclude.length > 0 && (
                        <span className="shrink-0 rounded bg-muted px-1.5 py-0.5 text-[10px] text-muted-foreground">
                          {t('settings.permissionExcludeShort')} {rule.exclude.length}
                        </span>
                      )}
                      <button
                        onClick={() => openEdit(idx)}
                        className="shrink-0 rounded p-1 text-muted-foreground hover:bg-background hover:text-primary cursor-pointer"
                        title={t('settings.permissionEdit')}
                      >
                        <Pencil size={12} />
                      </button>
                      <button
                        onClick={() => removeRule(idx)}
                        className="shrink-0 rounded p-1 text-muted-foreground hover:bg-background hover:text-red-500 cursor-pointer"
                        title={t('settings.permissionRemove')}
                      >
                        <Trash2 size={12} />
                      </button>
                    </div>
                  ))}
                </div>
              )}
            </div>
          </div>

          {/* 工具条：新增 + 脏标记 + 保存 */}
          <div className="flex items-center gap-2">
            <Button variant="outline" size="sm" onClick={openNew}>
              <Plus /> {t('settings.permissionAdd')}
            </Button>
            <div className="flex flex-wrap items-center gap-1.5">
              <span className="text-[11px] text-muted-foreground">{t('settings.permissionPreset')}</span>
              {PRESET_IDS.map(id => (
                <button
                  key={id}
                  onClick={() => applyPreset(id)}
                  title={t(`settings.permissionPreset_${id}`)}
                  className="rounded-md border border-border px-2 py-1 font-mono text-[11px] text-muted-foreground transition-colors hover:bg-muted/50 hover:text-primary cursor-pointer"
                >
                  {id}
                </button>
              ))}
            </div>
            <div className="ml-auto flex items-center gap-2">
              {dirty && (
                <span className="text-[11px] text-amber-600 dark:text-amber-400">
                  {t('settings.permissionUnsaved')}
                </span>
              )}
              <Button size="sm" onClick={handleSave} disabled={!dirty || saving}>
                {saving ? t('settings.saving') : t('settings.save')}
              </Button>
            </div>
          </div>

          {/* 预设策略：单行文本域，保持紧凑 */}
          <div>
            <Label className="block text-[11px] text-muted-foreground mb-1">{t('settings.permissionPolicies')}</Label>
            <Textarea
              value={policies}
              onChange={e => setPolicies(e.target.value)}
              placeholder="preset-id"
              className="font-mono text-xs"
              rows={2}
            />
          </div>

          <p className="text-[11px] text-muted-foreground">{t('settings.permissionNote')}</p>
        </div>
      )}

      {/* 编辑/新建规则弹窗：表单不占区块高度 */}
      <DialogRoot open={draft !== null} onOpenChange={open => { if (!open) closeDraft() }}>
        <DialogContent maxWidth="460px">
          <DialogHeader>
            <DialogTitle>
              {draft?.index === null ? t('settings.permissionNew') : t('settings.permissionEditTitle')}
            </DialogTitle>
          </DialogHeader>

          <DialogBody>
            {draft && (
              <>
                <div className="grid grid-cols-2 gap-3">
                  <div>
                    <Label className="block text-[11px] text-muted-foreground mb-1">
                      {t('settings.permissionCapability')}
                    </Label>
                    <Select value={draft.rule.capability} onValueChange={v => patchDraft({ capability: v })}>
                      <SelectTrigger className="h-8 text-xs w-full"><SelectValue /></SelectTrigger>
                      <SelectContent>
                        {capabilities.map(c => (
                          <SelectItem key={c} value={c}>{c}</SelectItem>
                        ))}
                      </SelectContent>
                    </Select>
                  </div>
                  <div>
                    <Label className="block text-[11px] text-muted-foreground mb-1">
                      {t('settings.permissionEffect')}
                    </Label>
                    <Select value={draft.rule.effect} onValueChange={v => patchDraft({ effect: v })}>
                      <SelectTrigger className="h-8 text-xs w-full"><SelectValue /></SelectTrigger>
                      <SelectContent>
                        {EFFECTS.map(e => (
                          <SelectItem key={e} value={e}>{t(`settings.permissionEffect_${e}`)}</SelectItem>
                        ))}
                      </SelectContent>
                    </Select>
                  </div>
                </div>

                {draft.rule.capability === 'shell' && (
                  <div>
                    <Label className="block text-[11px] text-muted-foreground mb-1">
                      {t('settings.permissionShellGroup')}
                    </Label>
                    <Select key={groupNonce} onValueChange={appendShellGroup}>
                      <SelectTrigger className="h-8 text-xs w-full">
                        <SelectValue placeholder={t('settings.permissionShellGroupPick')} />
                      </SelectTrigger>
                      <SelectContent>
                        {SHELL_COMMAND_GROUPS.map(g => (
                          <SelectItem key={g.id} value={g.id}>{g.id}</SelectItem>
                        ))}
                      </SelectContent>
                    </Select>
                  </div>
                )}

                <div>
                  <Label className="block text-[11px] text-muted-foreground mb-1">
                    {t('settings.permissionMatch')}
                  </Label>
                  <Textarea
                    value={(draft.rule.match || []).join('\n')}
                    onChange={e => patchDraft({ match: lines(e.target.value) })}
                    placeholder={'git *\ngit status'}
                    className="font-mono text-xs"
                    rows={3}
                  />
                </div>

                <div>
                  <Label className="block text-[11px] text-muted-foreground mb-1">
                    {t('settings.permissionExclude')}
                  </Label>
                  <Textarea
                    value={(draft.rule.exclude || []).join('\n')}
                    onChange={e => patchDraft({ exclude: lines(e.target.value) })}
                    placeholder="git push *"
                    className="font-mono text-xs"
                    rows={3}
                  />
                </div>
              </>
            )}
          </DialogBody>

          <DialogFooter>
            <Button variant="outline" onClick={closeDraft}>{t('settings.permissionCancel')}</Button>
            <Button onClick={commitDraft}>{t('settings.permissionConfirm')}</Button>
          </DialogFooter>
        </DialogContent>
      </DialogRoot>
    </SectionCard>
  )
}
