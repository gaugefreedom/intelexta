function parseBooleanFlag(value: unknown): boolean {
  if (typeof value === "boolean") {
    return value;
  }
  if (typeof value === "string") {
    const normalized = value.trim().toLowerCase();
    return normalized === "1" || normalized === "true" || normalized === "yes";
  }
  return false;
}

const metaEnv: Record<string, unknown> =
  typeof import.meta !== "undefined" && (import.meta as { env?: Record<string, unknown> }).env
    ? (import.meta as { env?: Record<string, unknown> }).env!
    : {};

const target = (metaEnv.VITE_BUILD_TARGET ?? "").toString().trim().toLowerCase();
const explicitV1 = parseBooleanFlag(metaEnv.VITE_V1_BUILD);

export const isV1Build = explicitV1 || target === "v1";
export const interactiveFeatureEnabled = !isV1Build;
