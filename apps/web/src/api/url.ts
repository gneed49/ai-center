const LOCAL_API_URL = "http://127.0.0.1:4317";
const ANDROID_EMULATOR_API_URL = "http://10.0.2.2:4317";

export type ApiUrlOptions = {
  configuredUrl?: string;
  userAgent?: string;
};

export function isAndroid(userAgent: string): boolean {
  return /android/i.test(userAgent);
}

export function resolveApiUrl(options: ApiUrlOptions = {}): string {
  const configuredUrl = options.configuredUrl?.trim();
  if (configuredUrl) return configuredUrl.replace(/\/+$/, "");

  const userAgent = options.userAgent ?? globalThis.navigator?.userAgent ?? "";
  return isAndroid(userAgent) ? ANDROID_EMULATOR_API_URL : LOCAL_API_URL;
}
