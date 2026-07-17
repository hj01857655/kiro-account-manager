export interface ApiEnvelope<T> {
  success: boolean;
  data?: T;
  error?: { code: string; message: string };
}

let csrfToken = "";
type ApiOptions = RequestInit & { suppressUnauthorizedEvent?: boolean };

export function setCsrfToken(value?: string) {
  csrfToken = value ?? "";
}

export async function api<T>(path: string, options: ApiOptions = {}): Promise<T> {
  const { suppressUnauthorizedEvent = false, ...requestOptions } = options;
  const method = (requestOptions.method ?? "GET").toUpperCase();
  const headers = new Headers(requestOptions.headers);
  if (requestOptions.body && !(requestOptions.body instanceof FormData) && !headers.has("content-type")) {
    headers.set("content-type", "application/json");
  }
  if (!["GET", "HEAD", "OPTIONS"].includes(method) && csrfToken) {
    headers.set("x-csrf-token", csrfToken);
  }
  const response = await fetch(path, { ...requestOptions, headers, credentials: "include" });
  const payload = (await response.json().catch(() => null)) as ApiEnvelope<T> | null;
  if (!response.ok || !payload?.success) {
    if (response.status === 401 && !suppressUnauthorizedEvent) {
      window.dispatchEvent(new Event("kiro:unauthorized"));
    }
    throw new Error(payload?.error?.message ?? `请求失败（HTTP ${response.status}）`);
  }
  return payload.data as T;
}

export async function login(username: string, password: string) {
  const data = await api<{ username: string; csrfToken: string }>("/api/auth/login", {
    method: "POST",
    body: JSON.stringify({ username, password })
  });
  setCsrfToken(data.csrfToken);
  return data;
}

export async function session() {
  const data = await api<{ username: string; csrfToken: string }>("/api/auth/me", {
    suppressUnauthorizedEvent: true
  });
  setCsrfToken(data.csrfToken);
  return data;
}

export async function logout() {
  await api("/api/auth/logout", { method: "POST" });
  setCsrfToken();
}

export async function downloadExport() {
  const headers = new Headers();
  if (csrfToken) headers.set("x-csrf-token", csrfToken);
  const response = await fetch("/api/accounts/export", { credentials: "include", headers });
  if (!response.ok) throw new Error("导出失败");
  const blob = await response.blob();
  const url = URL.createObjectURL(blob);
  const link = document.createElement("a");
  link.href = url;
  link.download = "kiro-accounts-redacted.json";
  link.click();
  URL.revokeObjectURL(url);
}
