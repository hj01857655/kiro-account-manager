import { FormEvent, ReactNode, useCallback, useEffect, useMemo, useState } from "react";
import {
  Activity, ArrowLeft, Boxes, CheckCircle2, ChevronRight, CircleGauge, CloudCog,
  Download, FileClock, Network, Import, LayoutDashboard, LoaderCircle, LogOut,
  Menu, Pencil, Plus, RefreshCw, Search, ServerCog, Settings, ShieldCheck,
  Sparkles, Trash2, UploadCloud, UserRoundCheck, UsersRound, X, XCircle
} from "lucide-react";
import { api, downloadExport, login, logout, session } from "./api";
import type { Account, AccountInput, AuditLog, DashboardStats, Group, Tag } from "./types";

type Page = "dashboard" | "accounts" | "detail" | "import" | "groups" | "gateway" | "settings" | "logs";
type User = { username: string };

export default function App() {
  const [user, setUser] = useState<User | null | undefined>();
  const [page, setPage] = useState<Page>("dashboard");
  const [detailId, setDetailId] = useState<string>();
  const [toast, setToast] = useState<string>();
  const notify = useCallback((message: string) => {
    setToast(message);
    window.setTimeout(() => setToast(undefined), 3200);
  }, []);

  useEffect(() => {
    session().then(data => setUser({ username: data.username })).catch(() => setUser(null));
    const unauthorized = () => setUser(null);
    window.addEventListener("kiro:unauthorized", unauthorized);
    return () => window.removeEventListener("kiro:unauthorized", unauthorized);
  }, []);

  if (user === undefined) return <Splash />;
  if (!user) return <Login onSuccess={name => setUser({ username: name })} />;

  const openDetail = (id: string) => { setDetailId(id); setPage("detail"); };
  return (
    <Shell user={user} page={page} navigate={setPage} onLogout={async () => { await logout(); setUser(null); }}>
      {page === "dashboard" && <Dashboard navigate={setPage} />}
      {page === "accounts" && <Accounts onDetail={openDetail} notify={notify} />}
      {page === "detail" && detailId && <AccountDetail id={detailId} back={() => setPage("accounts")} notify={notify} />}
      {page === "import" && <ImportAccounts notify={notify} done={() => setPage("accounts")} />}
      {page === "groups" && <Groups notify={notify} />}
      {page === "gateway" && <GatewaySettings notify={notify} />}
      {page === "settings" && <SystemSettings notify={notify} />}
      {page === "logs" && <Logs />}
      {toast && <div className="toast"><CheckCircle2 size={17} />{toast}</div>}
    </Shell>
  );
}

function Splash() {
  return <div className="splash"><div className="brand-mark"><Sparkles /></div><LoaderCircle className="spin" /></div>;
}

function Login({ onSuccess }: { onSuccess: (username: string) => void }) {
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const submit = async (event: FormEvent) => {
    event.preventDefault(); setBusy(true); setError("");
    try { const result = await login(username, password); onSuccess(result.username); }
    catch (reason) { setError(reason instanceof Error ? reason.message : "登录失败"); }
    finally { setBusy(false); }
  };
  return (
    <main className="login-page">
      <div className="login-glow glow-one" /><div className="login-glow glow-two" />
      <section className="login-card">
        <div className="login-brand"><div className="brand-mark"><Sparkles /></div><span>Kiro Manager</span></div>
        <div className="login-copy"><span className="eyebrow">SECURE ADMIN CONSOLE</span><h1>欢迎回来</h1><p>登录后管理账号、配额和 API Gateway。</p></div>
        <form onSubmit={submit}>
          <label>管理员账号<input autoComplete="username" value={username} onChange={e => setUsername(e.target.value)} placeholder="ADMIN_USERNAME" required autoFocus /></label>
          <label>密码<input type="password" autoComplete="current-password" value={password} onChange={e => setPassword(e.target.value)} placeholder="输入管理员密码" required /></label>
          {error && <div className="form-error"><XCircle size={16} />{error}</div>}
          <button className="primary wide" disabled={busy}>{busy ? <LoaderCircle className="spin" size={18} /> : <ShieldCheck size={18} />}{busy ? "正在验证…" : "安全登录"}</button>
        </form>
        <div className="login-security"><ShieldCheck size={14} />凭据通过安全 Cookie 保存，不写入浏览器存储</div>
      </section>
    </main>
  );
}

function Shell({ user, page, navigate, onLogout, children }: { user: User; page: Page; navigate: (page: Page) => void; onLogout: () => void; children: ReactNode }) {
  const [open, setOpen] = useState(false);
  const items: { key: Page; label: string; icon: typeof Activity }[] = [
    { key: "dashboard", label: "仪表盘", icon: LayoutDashboard },
    { key: "accounts", label: "账号管理", icon: UsersRound },
    { key: "import", label: "导入账号", icon: Import },
    { key: "groups", label: "分组与标签", icon: Boxes },
    { key: "gateway", label: "API Gateway", icon: Network },
    { key: "settings", label: "系统设置", icon: Settings },
    { key: "logs", label: "操作日志", icon: FileClock }
  ];
  return (
    <div className="app-shell">
      <aside className={`sidebar ${open ? "open" : ""}`}>
        <div className="sidebar-brand"><div className="brand-mark small"><Sparkles /></div><div><strong>Kiro Manager</strong><span>Web Console</span></div><button className="icon-button close-nav" onClick={() => setOpen(false)}><X /></button></div>
        <nav>{items.map(item => <button key={item.key} className={(page === item.key || (page === "detail" && item.key === "accounts")) ? "active" : ""} onClick={() => { navigate(item.key); setOpen(false); }}><item.icon /><span>{item.label}</span>{item.key === "gateway" && <i className="live-dot" />}</button>)}</nav>
        <div className="sidebar-bottom"><div className="user-chip"><div className="avatar">{user.username.slice(0, 1).toUpperCase()}</div><div><strong>{user.username}</strong><span>Administrator</span></div></div><button className="icon-button" title="退出登录" onClick={onLogout}><LogOut /></button></div>
      </aside>
      {open && <button className="nav-backdrop" aria-label="关闭菜单" onClick={() => setOpen(false)} />}
      <div className="workspace"><header className="mobile-header"><button className="icon-button" onClick={() => setOpen(true)}><Menu /></button><strong>Kiro Manager</strong></header><main className="content">{children}</main></div>
    </div>
  );
}

function PageHead({ eyebrow, title, description, actions }: { eyebrow: string; title: string; description: string; actions?: ReactNode }) {
  return <div className="page-head"><div><span className="eyebrow">{eyebrow}</span><h1>{title}</h1><p>{description}</p></div>{actions && <div className="head-actions">{actions}</div>}</div>;
}

function Dashboard({ navigate }: { navigate: (page: Page) => void }) {
  const [stats, setStats] = useState<DashboardStats>();
  const [accounts, setAccounts] = useState<Account[]>([]);
  useEffect(() => { api<DashboardStats>("/api/dashboard").then(setStats); api<Account[]>("/api/accounts").then(setAccounts); }, []);
  const cards = [
    ["账号总数", stats?.totalAccounts ?? "—", UsersRound, "neutral"], ["可用账号", stats?.availableAccounts ?? "—", CheckCircle2, "good"],
    ["异常账号", stats?.abnormalAccounts ?? "—", XCircle, "bad"], ["Gateway 请求", stats?.gatewayRequestCount ?? "—", Activity, "purple"]
  ] as const;
  const quota = stats?.totalQuota ? Math.min(100, (stats.usedQuota / stats.totalQuota) * 100) : 0;
  return <>
    <PageHead eyebrow="OVERVIEW" title="运行概览" description="集中查看 Kiro 账号健康状态、配额与网关流量。" actions={<button className="secondary" onClick={() => navigate("accounts")}><UsersRound size={17} />管理账号</button>} />
    <section className="stat-grid">{cards.map(([label, value, Icon, tone]) => <div className={`stat-card ${tone}`} key={label}><div className="stat-icon"><Icon /></div><div><span>{label}</span><strong>{value}</strong></div></div>)}</section>
    <section className="dashboard-grid">
      <div className="panel quota-panel"><PanelTitle icon={<CircleGauge />} title="整体配额" subtitle="基于最近一次账号查询" /><div className="quota-numbers"><strong>{formatNumber(stats?.usedQuota)}</strong><span>/ {formatNumber(stats?.totalQuota)}</span></div><div className="progress"><i style={{ width: `${quota}%` }} /></div><div className="panel-foot"><span>{quota.toFixed(1)}% 已使用</span><span>刷新：{formatTime(stats?.lastRefreshAt)}</span></div></div>
      <div className="panel"><PanelTitle icon={<Activity />} title="最近账号状态" subtitle="最新更新的账号" /><div className="account-mini-list">{accounts.slice(0, 5).map(account => <div key={account.id}><StatusDot status={account.status} /><div><strong>{account.label}</strong><span>{account.email || account.provider || "未设置邮箱"}</span></div><span className="mini-time">{formatTime(account.updatedAt)}</span></div>)}{!accounts.length && <Empty compact title="还没有账号" />}</div></div>
    </section>
  </>;
}

function Accounts({ onDetail, notify }: { onDetail: (id: string) => void; notify: (message: string) => void }) {
  const [accounts, setAccounts] = useState<Account[]>([]); const [groups, setGroups] = useState<Group[]>([]); const [tags, setTags] = useState<Tag[]>([]);
  const [query, setQuery] = useState(""); const [editing, setEditing] = useState<Account | null | undefined>(); const [busyId, setBusyId] = useState("");
  const load = useCallback(() => { api<Account[]>("/api/accounts").then(setAccounts); api<Group[]>("/api/groups").then(setGroups); api<Tag[]>("/api/tags").then(setTags); }, []);
  useEffect(load, [load]);
  const filtered = useMemo(() => accounts.filter(account => `${account.label} ${account.email ?? ""} ${account.provider ?? ""}`.toLowerCase().includes(query.toLowerCase())), [accounts, query]);
  const action = async (id: string, name: "refresh" | "check") => { setBusyId(`${id}:${name}`); try { await api(`/api/accounts/${id}/${name}`, { method: "POST" }); notify(name === "refresh" ? "Token 刷新完成" : "可用性检测完成"); load(); } catch (e) { notify(e instanceof Error ? e.message : "操作失败"); } finally { setBusyId(""); } };
  const remove = async (account: Account) => { if (!confirm(`确定删除账号“${account.label}”吗？`)) return; await api(`/api/accounts/${account.id}`, { method: "DELETE" }); notify("账号已删除"); load(); };
  return <>
    <PageHead eyebrow="ACCOUNTS" title="账号管理" description="账号凭据仅在服务端加密保存，列表不会返回完整 Token。" actions={<><button className="secondary" onClick={downloadExport}><Download size={17} />安全导出</button><button className="primary" onClick={() => setEditing(null)}><Plus size={17} />添加账号</button></>} />
    <div className="toolbar"><div className="search"><Search /><input value={query} onChange={e => setQuery(e.target.value)} placeholder="搜索账号、邮箱或提供商" /></div><div className="count-chip">{filtered.length} 个账号</div></div>
    <section className="account-grid">{filtered.map(account => <article className="account-card" key={account.id}>
      <button className="card-main" onClick={() => onDetail(account.id)}><div className="account-card-head"><div className="account-avatar">{account.label.slice(0, 1).toUpperCase()}</div><div><h3>{account.label}</h3><p>{account.email || "未设置邮箱"}</p></div><StatusPill status={account.status} /></div><div className="account-meta"><span><CloudCog />{account.provider || account.authMethod || "Kiro"}</span><span><ServerCog />{account.region || "us-east-1"}</span></div><div className="token-flags"><span className={account.hasAccessToken ? "ok" : ""}>{account.hasAccessToken ? "Access Token 已配置" : "缺少 Access Token"}</span><span className={account.hasRefreshToken ? "ok" : ""}>{account.hasRefreshToken ? "可刷新" : "不可刷新"}</span></div><div className="card-link">查看账号详情<ChevronRight /></div></button>
      <div className="card-actions"><button onClick={() => action(account.id, "check")} disabled={!!busyId}>{busyId === `${account.id}:check` ? <LoaderCircle className="spin" /> : <UserRoundCheck />}检测</button><button onClick={() => action(account.id, "refresh")} disabled={!account.hasRefreshToken || !!busyId}>{busyId === `${account.id}:refresh` ? <LoaderCircle className="spin" /> : <RefreshCw />}刷新</button><button onClick={() => setEditing(account)}><Pencil />编辑</button><button className="danger-text" onClick={() => remove(account)}><Trash2 /></button></div>
    </article>)}{!filtered.length && <Empty title="没有匹配的账号" description="添加一个账号，或调整搜索条件。" />}</section>
    {editing !== undefined && <AccountEditor account={editing} groups={groups} tags={tags} close={() => setEditing(undefined)} saved={() => { setEditing(undefined); notify(editing ? "账号已更新" : "账号已添加"); load(); }} />}
  </>;
}

function AccountEditor({ account, groups, tags, close, saved }: { account: Account | null; groups: Group[]; tags: Tag[]; close: () => void; saved: () => void }) {
  const [form, setForm] = useState<AccountInput>({ label: account?.label ?? "", email: account?.email ?? "", status: account?.status ?? "active", provider: account?.provider ?? "social", authMethod: account?.authMethod ?? "social", region: account?.region ?? "us-east-1", profileArn: account?.profileArn ?? "", groupId: account?.groupId ?? "", machineId: account?.machineId ?? "", enabled: account?.enabled ?? true, accessToken: "", refreshToken: "", clientId: "", clientSecret: "", tagIds: account?.tags.map(tag => tag.id) ?? [] });
  const [busy, setBusy] = useState(false); const [error, setError] = useState("");
  const field = (name: keyof AccountInput, value: string | boolean) => setForm(previous => ({ ...previous, [name]: value }));
  const submit = async (event: FormEvent) => { event.preventDefault(); setBusy(true); setError(""); try { await api(account ? `/api/accounts/${account.id}` : "/api/accounts", { method: account ? "PUT" : "POST", body: JSON.stringify(form) }); saved(); } catch (e) { setError(e instanceof Error ? e.message : "保存失败"); } finally { setBusy(false); } };
  return <Modal title={account ? "编辑账号" : "添加 Kiro 账号"} close={close}><form className="form-grid" onSubmit={submit}>
    <label className="span-2">显示名称<input value={form.label} onChange={e => field("label", e.target.value)} required /></label><label>邮箱<input type="email" value={form.email} onChange={e => field("email", e.target.value)} /></label><label>认证方式<select value={form.authMethod} onChange={e => field("authMethod", e.target.value)}><option value="social">Social</option><option value="idc">AWS IdC</option><option value="external_idp">External IdP</option></select></label>
    <label>区域<input value={form.region} onChange={e => field("region", e.target.value)} /></label><label>分组<select value={form.groupId} onChange={e => field("groupId", e.target.value)}><option value="">未分组</option>{groups.map(group => <option key={group.id} value={group.id}>{group.name}</option>)}</select></label><label className="span-2">Profile ARN<input value={form.profileArn} onChange={e => field("profileArn", e.target.value)} /></label>
    <label className="span-2">标签<div className="tag-picker">{tags.map(tag => <button type="button" key={tag.id} className={form.tagIds.includes(tag.id) ? "selected" : ""} onClick={() => setForm(previous => ({ ...previous, tagIds: previous.tagIds.includes(tag.id) ? previous.tagIds.filter(id => id !== tag.id) : [...previous.tagIds, tag.id] }))}><i style={{ background: tag.color }} />{tag.name}</button>)}{!tags.length && <span>可在“分组与标签”页面创建标签</span>}</div></label><div className="form-divider span-2">敏感凭据（{account ? "留空则保留原值" : "写入后使用 AES-256-GCM 加密"}）</div><label className="span-2">Access Token<textarea rows={2} value={form.accessToken} onChange={e => field("accessToken", e.target.value)} required={!account} /></label><label className="span-2">Refresh Token<textarea rows={2} value={form.refreshToken} onChange={e => field("refreshToken", e.target.value)} /></label><label>Client ID<input value={form.clientId} onChange={e => field("clientId", e.target.value)} /></label><label>Client Secret<input type="password" value={form.clientSecret} onChange={e => field("clientSecret", e.target.value)} /></label><label className="switch-row span-2"><input type="checkbox" checked={form.enabled} onChange={e => field("enabled", e.target.checked)} /><span>允许此账号参与 Gateway 选择</span></label>
    {error && <div className="form-error span-2"><XCircle />{error}</div>}<div className="modal-actions span-2"><button type="button" className="secondary" onClick={close}>取消</button><button className="primary" disabled={busy}>{busy && <LoaderCircle className="spin" />}保存账号</button></div>
  </form></Modal>;
}

function AccountDetail({ id, back, notify }: { id: string; back: () => void; notify: (message: string) => void }) {
  const [account, setAccount] = useState<Account>(); const [busy, setBusy] = useState(false);
  const load = useCallback(() => api<Account>(`/api/accounts/${id}`).then(setAccount), [id]); useEffect(() => { void load(); }, [load]);
  if (!account) return <Loading />;
  const run = async (operation: "usage" | "models" | "check" | "refresh") => { setBusy(true); try { await api(`/api/accounts/${id}/${operation}`, { method: ["usage", "models"].includes(operation) ? "GET" : "POST" }); notify("操作完成"); load(); } catch (e) { notify(e instanceof Error ? e.message : "操作失败"); } finally { setBusy(false); } };
  return <><button className="back-button" onClick={back}><ArrowLeft />返回账号列表</button><PageHead eyebrow="ACCOUNT DETAIL" title={account.label} description={account.email || "Kiro 账号详情"} actions={<><button className="secondary" onClick={() => run("check")} disabled={busy}><UserRoundCheck />检测</button><button className="primary" onClick={() => run("refresh")} disabled={busy || !account.hasRefreshToken}><RefreshCw />刷新 Token</button></>} />
    <section className="detail-grid"><div className="panel"><PanelTitle icon={<ShieldCheck />} title="账号状态" subtitle="服务端安全摘要" /><dl className="detail-list"><Detail name="状态" value={<StatusPill status={account.status} />} /><Detail name="认证方式" value={account.authMethod || account.provider || "—"} /><Detail name="区域" value={account.region || "us-east-1"} /><Detail name="Machine ID" value={<code>{account.machineId}</code>} /><Detail name="Access Token" value={account.hasAccessToken ? "已加密保存" : "未配置"} /><Detail name="Refresh Token" value={account.hasRefreshToken ? "已加密保存" : "未配置"} /><Detail name="最近检测" value={formatTime(account.lastCheckedAt)} /></dl></div><div className="panel"><PanelTitle icon={<CircleGauge />} title="用量与模型" subtitle="从 Kiro Management API 查询" /><div className="inline-actions"><button className="secondary" onClick={() => run("usage")} disabled={busy}>查询配额</button><button className="secondary" onClick={() => run("models")} disabled={busy}>查询模型</button></div><JsonPreview value={{ usage: account.usage ?? "尚未查询", models: account.models ?? "尚未查询" }} /></div></section>
  </>;
}

function ImportAccounts({ notify, done }: { notify: (message: string) => void; done: () => void }) {
  const [content, setContent] = useState(""); const [busy, setBusy] = useState(false); const [result, setResult] = useState<{ createdIds: string[]; errors: unknown[] }>();
  const choose = async (file?: File) => { if (!file) return; if (file.size > 10 * 1024 * 1024) return notify("文件不能超过 10MB"); setContent(await file.text()); };
  const submit = async () => { setBusy(true); try { const parsed = JSON.parse(content); const data = await api<{ createdIds: string[]; errors: unknown[] }>("/api/accounts/import", { method: "POST", body: JSON.stringify(parsed) }); setResult(data); notify(`成功导入 ${data.createdIds.length} 个账号`); } catch (e) { notify(e instanceof Error ? e.message : "导入失败"); } finally { setBusy(false); } };
  return <><PageHead eyebrow="IMPORT" title="导入账号" description="上传 JSON 文件或粘贴账号数组；服务端会再次检查大小与结构。" /><section className="panel import-panel"><label className="drop-zone"><UploadCloud /><strong>选择 JSON 文件</strong><span>最大 10MB，最多 500 个账号</span><input type="file" accept="application/json,.json" onChange={e => choose(e.target.files?.[0])} /></label><div className="or"><span>或粘贴 JSON</span></div><textarea className="json-input" rows={14} value={content} onChange={e => setContent(e.target.value)} placeholder={'[{\n  "label": "Account 01",\n  "accessToken": "...",\n  "refreshToken": "..."\n}]'} /><div className="import-actions"><span>{content ? `${new Blob([content]).size} bytes` : "等待输入"}</span><div>{result?.createdIds.length ? <button className="secondary" onClick={done}>查看账号</button> : null}<button className="primary" disabled={!content || busy} onClick={submit}>{busy ? <LoaderCircle className="spin" /> : <Import />}开始导入</button></div></div></section></>;
}

function Groups({ notify }: { notify: (message: string) => void }) {
  const [groups, setGroups] = useState<Group[]>([]); const [tags, setTags] = useState<Tag[]>([]); const [name, setName] = useState(""); const [color, setColor] = useState("#8b5cf6"); const [tagName, setTagName] = useState(""); const [tagColor, setTagColor] = useState("#55d6a8");
  const load = useCallback(() => { api<Group[]>("/api/groups").then(setGroups); api<Tag[]>("/api/tags").then(setTags); }, []); useEffect(() => { load(); }, [load]);
  const create = async (e: FormEvent) => { e.preventDefault(); await api("/api/groups", { method: "POST", body: JSON.stringify({ name, color, sortOrder: groups.length }) }); setName(""); notify("分组已创建"); load(); };
  const remove = async (group: Group) => { if (!confirm(`删除分组“${group.name}”？账号不会被删除。`)) return; await api(`/api/groups/${group.id}`, { method: "DELETE" }); notify("分组已删除"); load(); };
  const createTag = async (e: FormEvent) => { e.preventDefault(); await api("/api/tags", { method: "POST", body: JSON.stringify({ name: tagName, color: tagColor }) }); setTagName(""); notify("标签已创建"); load(); };
  const removeTag = async (tag: Tag) => { if (!confirm(`删除标签“${tag.name}”？`)) return; await api(`/api/tags/${tag.id}`, { method: "DELETE" }); notify("标签已删除"); load(); };
  return <><PageHead eyebrow="ORGANIZATION" title="分组与标签" description="按用途组织账号，删除分组或标签不会删除账号。" /><section className="groups-layout"><div className="stack"><form className="panel group-form" onSubmit={create}><PanelTitle icon={<Plus />} title="新建分组" subtitle="为账号建立逻辑集合" /><label>名称<input value={name} onChange={e => setName(e.target.value)} placeholder="例如：生产账号" required /></label><label>标识颜色<input type="color" value={color} onChange={e => setColor(e.target.value)} /></label><button className="primary wide">创建分组</button></form><form className="panel group-form" onSubmit={createTag}><PanelTitle icon={<Plus />} title="新建标签" subtitle="为账号添加多维标记" /><label>名称<input value={tagName} onChange={e => setTagName(e.target.value)} placeholder="例如：高配额" required /></label><label>标识颜色<input type="color" value={tagColor} onChange={e => setTagColor(e.target.value)} /></label><button className="primary wide">创建标签</button></form></div><div className="stack"><div className="panel"><PanelTitle icon={<Boxes />} title="现有分组" subtitle={`${groups.length} 个分组`} /><div className="group-list">{groups.map(group => <div key={group.id}><i style={{ background: group.color || "#64748b" }} /><div><strong>{group.name}</strong><span>排序 {group.sortOrder}</span></div><button className="icon-button danger-text" onClick={() => remove(group)}><Trash2 /></button></div>)}{!groups.length && <Empty compact title="还没有分组" />}</div></div><div className="panel"><PanelTitle icon={<Boxes />} title="现有标签" subtitle={`${tags.length} 个标签`} /><div className="group-list">{tags.map(tag => <div key={tag.id}><i style={{ background: tag.color }} /><div><strong>{tag.name}</strong><span>可关联多个账号</span></div><button className="icon-button danger-text" onClick={() => removeTag(tag)}><Trash2 /></button></div>)}{!tags.length && <Empty compact title="还没有标签" />}</div></div></div></section></>;
}

function GatewaySettings({ notify }: { notify: (message: string) => void }) {
  const [settings, setSettings] = useState<Record<string, unknown>>(); const runtime = settings?.gatewayRuntime as { enabled?: boolean; hasApiKey?: boolean; defaultAccount?: string; autoSwitch?: boolean } | undefined;
  useEffect(() => { api<Record<string, unknown>>("/api/settings").then(setSettings); }, []);
  if (!settings) return <Loading />;
  const save = async () => { await api("/api/settings", { method: "PUT", body: JSON.stringify({ gatewayPolicy: { updatedFromWeb: true } }) }); notify("Gateway 面板设置已保存；运行凭据由环境变量控制"); };
  return <><PageHead eyebrow="KIRO API GATEWAY" title="Gateway 设置" description="提供 Anthropic、OpenAI Chat Completions 与 Responses 兼容路由。" actions={<button className="primary" onClick={save}><ShieldCheck />保存策略</button>} /><section className="detail-grid"><div className="panel"><PanelTitle icon={<Network />} title="运行状态" subtitle="敏感值只从服务端环境变量读取" /><dl className="detail-list"><Detail name="Gateway" value={<StatusPill status={runtime?.enabled ? "active" : "disabled"} />} /><Detail name="独立 API Key" value={runtime?.hasApiKey ? "已配置（不显示）" : "未配置"} /><Detail name="自动切换账号" value={runtime?.autoSwitch ? "启用" : "关闭"} /><Detail name="默认账号" value={runtime?.defaultAccount || "自动选择"} /></dl></div><div className="panel"><PanelTitle icon={<CloudCog />} title="兼容端点" subtitle="Authorization: Bearer GATEWAY_API_KEY" /><div className="endpoint-list"><code>POST /v1/messages</code><code>POST /v1/chat/completions</code><code>POST /v1/responses</code><code>GET /v1/models</code></div><div className="notice"><ShieldCheck />管理员 Cookie 不可用于 Gateway，Gateway Key 也不可登录管理面板。</div></div></section></>;
}

function SystemSettings({ notify }: { notify: (message: string) => void }) {
  const [value, setValue] = useState("{}"); const [busy, setBusy] = useState(false);
  useEffect(() => { api<Record<string, unknown>>("/api/settings").then(data => { const { gatewayRuntime: _, ...editable } = data; setValue(JSON.stringify(editable, null, 2)); }); }, []);
  const save = async () => { setBusy(true); try { await api("/api/settings", { method: "PUT", body: JSON.stringify(JSON.parse(value)) }); notify("系统设置已保存"); } catch (e) { notify(e instanceof Error ? e.message : "保存失败"); } finally { setBusy(false); } };
  return <><PageHead eyebrow="SYSTEM" title="系统设置" description="普通运行策略保存在 SQLite；密码、密钥和 Token 只能通过环境变量或账号接口设置。" actions={<button className="primary" onClick={save} disabled={busy}><Settings />保存设置</button>} /><section className="panel"><PanelTitle icon={<ServerCog />} title="高级 JSON 设置" subtitle="敏感字段会被服务端拒绝" /><textarea className="json-input" rows={20} value={value} onChange={e => setValue(e.target.value)} /><div className="notice"><ShieldCheck />ADMIN_PASSWORD、JWT_SECRET、DATA_ENCRYPTION_KEY 与 GATEWAY_API_KEY 不会出现在此页面。</div></section></>;
}

function Logs() {
  const [logs, setLogs] = useState<AuditLog[]>([]); const [loading, setLoading] = useState(true);
  const load = useCallback(() => { setLoading(true); api<AuditLog[]>("/api/logs?limit=300").then(setLogs).finally(() => setLoading(false)); }, []); useEffect(load, [load]);
  return <><PageHead eyebrow="AUDIT" title="操作日志" description="记录登录失败与管理操作；不会保存密码、Cookie、Token 或 Authorization。" actions={<button className="secondary" onClick={load}><RefreshCw />刷新</button>} /><section className="panel log-panel">{loading ? <Loading /> : <div className="log-table"><div className="log-row log-head"><span>时间</span><span>级别</span><span>操作</span><span>说明</span><span>客户端 IP</span></div>{logs.map(log => <div className="log-row" key={log.id}><span>{formatTime(log.createdAt)}</span><span><i className={`level ${log.level}`}>{log.level}</i></span><code>{log.action}</code><span>{log.message}</span><span>{log.clientIp || "—"}</span></div>)}{!logs.length && <Empty title="暂无操作日志" />}</div>}</section></>;
}

function Modal({ title, close, children }: { title: string; close: () => void; children: ReactNode }) { return <div className="modal-backdrop" onMouseDown={e => e.currentTarget === e.target && close()}><div className="modal"><div className="modal-head"><div><span className="eyebrow">ACCOUNT</span><h2>{title}</h2></div><button className="icon-button" onClick={close}><X /></button></div>{children}</div></div>; }
function PanelTitle({ icon, title, subtitle }: { icon: ReactNode; title: string; subtitle: string }) { return <div className="panel-title"><div className="panel-icon">{icon}</div><div><h2>{title}</h2><p>{subtitle}</p></div></div>; }
function Empty({ title, description, compact = false }: { title: string; description?: string; compact?: boolean }) { return <div className={`empty ${compact ? "compact" : ""}`}><Boxes /><strong>{title}</strong>{description && <span>{description}</span>}</div>; }
function Loading() { return <div className="loading"><LoaderCircle className="spin" />加载中…</div>; }
function StatusDot({ status }: { status: string }) { return <i className={`status-dot ${status}`} />; }
function StatusPill({ status }: { status: string | boolean }) { const active = status === "active" || status === true; const label = active ? "正常" : status === "disabled" || status === false ? "已停用" : status === "banned" ? "已封禁" : "异常"; return <span className={`status-pill ${active ? "active" : "error"}`}><StatusDot status={active ? "active" : "error"} />{label}</span>; }
function Detail({ name, value }: { name: string; value: ReactNode }) { return <div><dt>{name}</dt><dd>{value}</dd></div>; }
function JsonPreview({ value }: { value: unknown }) { return <pre className="json-preview">{JSON.stringify(value, null, 2)}</pre>; }
function formatTime(value?: string) { if (!value) return "暂无"; const date = new Date(value); return Number.isNaN(date.getTime()) ? value : date.toLocaleString("zh-CN", { hour12: false }); }
function formatNumber(value?: number) { return (value ?? 0).toLocaleString("zh-CN", { maximumFractionDigits: 1 }); }
