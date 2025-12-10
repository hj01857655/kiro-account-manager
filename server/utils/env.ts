export function isDebugMode(): boolean {
  const debug = Deno.env.get("DEBUG");
  if (debug === "true" || debug === "1") return true;

  const logLevel = Deno.env.get("LOG_LEVEL");
  if (logLevel?.toLowerCase() === "debug") return true;

  const ginMode = Deno.env.get("GIN_MODE");
  if (ginMode === "debug") return true;

  return false;
}

export function getEnvWithDefault(key: string, defaultValue: string): string {
  return Deno.env.get(key) || defaultValue;
}

export function getEnvBool(key: string): boolean {
  const value = Deno.env.get(key)?.toLowerCase().trim() || "";
  return ["true", "1", "yes", "on"].includes(value);
}

export function getEnvBoolWithDefault(key: string, defaultValue: boolean): boolean {
  const value = Deno.env.get(key);
  return value ? getEnvBool(key) : defaultValue;
}
