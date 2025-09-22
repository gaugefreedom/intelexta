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

const target = (import.meta.env.VITE_BUILD_TARGET ?? "").toString().trim().toLowerCase();
const explicitV1 = parseBooleanFlag(import.meta.env.VITE_V1_BUILD);

export const isV1Build = explicitV1 || target === "v1";
export const interactiveFeatureEnabled = !isV1Build;
