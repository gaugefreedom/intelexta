import type { CSSProperties } from "react";

export const buttonBase: CSSProperties = {
  display: "inline-flex",
  alignItems: "center",
  justifyContent: "center",
  gap: "6px",
  padding: "6px 12px",
  borderRadius: "6px",
  border: "1px solid #2d2d2d",
  backgroundColor: "#1b1d27",
  color: "#f3f4f6",
  fontSize: "0.85rem",
  fontFamily: "inherit",
  fontWeight: 500,
  lineHeight: 1.2,
  cursor: "pointer",
  transition:
    "background-color 0.15s ease, border-color 0.15s ease, color 0.15s ease, box-shadow 0.15s ease",
};

export const buttonPrimary: CSSProperties = {
  ...buttonBase,
  backgroundColor: "#2563eb",
  borderColor: "#3b82f6",
  color: "#f8fafc",
};

export const buttonSecondary: CSSProperties = {
  ...buttonBase,
  backgroundColor: "#1f2937",
  borderColor: "#374151",
  color: "#e5e7eb",
};

export const buttonGhost: CSSProperties = {
  ...buttonBase,
  backgroundColor: "transparent",
  borderColor: "#374151",
  color: "#e5e7eb",
};

export const buttonDanger: CSSProperties = {
  ...buttonBase,
  backgroundColor: "#7f1d1d",
  borderColor: "#ef4444",
  color: "#fee2e2",
};

export const buttonDisabled: CSSProperties = {
  opacity: 0.5,
  cursor: "not-allowed",
  boxShadow: "none",
};

export function combineButtonStyles(
  ...styles: Array<CSSProperties | null | undefined | false>
): CSSProperties {
  return styles.reduce<CSSProperties>((acc, style) => {
    if (style) {
      Object.assign(acc, style);
    }
    return acc;
  }, {});
}
