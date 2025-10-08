import React from "react";
import {
  listApiKeysStatus,
  setApiKey,
  deleteApiKey,
  type ApiKeyStatus,
  type ApiKeyProvider,
} from "../lib/api.js";
import { buttonPrimary, buttonSecondary } from "../styles/common.js";

interface ApiKeySettingsModalProps {
  onClose: () => void;
}

export default function ApiKeySettingsModal({
  onClose,
}: ApiKeySettingsModalProps) {
  const [apiKeys, setApiKeys] = React.useState<ApiKeyStatus[]>([]);
  const [loading, setLoading] = React.useState(true);
  const [error, setError] = React.useState<string | null>(null);
  const [editingProvider, setEditingProvider] =
    React.useState<ApiKeyProvider | null>(null);
  const [newKey, setNewKey] = React.useState("");
  const [saving, setSaving] = React.useState(false);

  React.useEffect(() => {
    loadApiKeys();
  }, []);

  const loadApiKeys = async () => {
    try {
      setLoading(true);
      setError(null);
      const keys = await listApiKeysStatus();
      setApiKeys(keys);
    } catch (err) {
      setError(`Failed to load API keys: ${err}`);
    } finally {
      setLoading(false);
    }
  };

  const handleSaveKey = async () => {
    if (!editingProvider || !newKey.trim()) return;

    try {
      setSaving(true);
      setError(null);
      await setApiKey(editingProvider, newKey.trim());
      await loadApiKeys();
      setEditingProvider(null);
      setNewKey("");
    } catch (err) {
      setError(`Failed to save API key: ${err}`);
    } finally {
      setSaving(false);
    }
  };

  const handleDeleteKey = async (provider: ApiKeyProvider) => {
    if (!confirm(`Delete API key for ${provider}?`)) return;

    try {
      setSaving(true);
      setError(null);
      await deleteApiKey(provider);
      await loadApiKeys();
    } catch (err) {
      setError(`Failed to delete API key: ${err}`);
    } finally {
      setSaving(false);
    }
  };

  const handleCancel = () => {
    setEditingProvider(null);
    setNewKey("");
  };

  return (
    <div
      style={{
        position: "fixed",
        top: 0,
        left: 0,
        right: 0,
        bottom: 0,
        background: "rgba(0,0,0,0.7)",
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        zIndex: 1000,
      }}
      onClick={onClose}
    >
      <div
        style={{
          background: "#2d2d2d",
          padding: "24px",
          borderRadius: "8px",
          minWidth: "600px",
          maxWidth: "800px",
          maxHeight: "80vh",
          overflow: "auto",
          border: "1px solid #444",
        }}
        onClick={(e) => e.stopPropagation()}
      >
        <div
          style={{
            display: "flex",
            justifyContent: "space-between",
            alignItems: "center",
            marginBottom: "20px",
          }}
        >
          <h2 style={{ margin: 0, fontSize: "20px", color: "#e0e0e0" }}>
            API Key Settings
          </h2>
          <button
            onClick={onClose}
            style={{
              background: "transparent",
              border: "none",
              color: "#999",
              fontSize: "24px",
              cursor: "pointer",
              padding: "0 8px",
            }}
          >
            ×
          </button>
        </div>

        {error && (
          <div
            style={{
              background: "#3d1f1f",
              border: "1px solid #6d2828",
              color: "#ff6b6b",
              padding: "12px",
              borderRadius: "4px",
              marginBottom: "16px",
            }}
          >
            {error}
          </div>
        )}

        {loading ? (
          <div style={{ textAlign: "center", padding: "40px", color: "#999" }}>
            Loading API keys...
          </div>
        ) : (
          <div>
            <p style={{ color: "#999", marginBottom: "20px" }}>
              Configure API keys for external LLM providers. Keys are stored
              securely in your system keychain.
            </p>

            <div style={{ display: "flex", flexDirection: "column", gap: "16px" }}>
              {apiKeys.map((keyStatus) => (
                <div
                  key={keyStatus.provider}
                  style={{
                    background: "#1e1e1e",
                    padding: "16px",
                    borderRadius: "4px",
                    border: "1px solid #333",
                  }}
                >
                  <div
                    style={{
                      display: "flex",
                      justifyContent: "space-between",
                      alignItems: "flex-start",
                      marginBottom: "8px",
                    }}
                  >
                    <div>
                      <h3
                        style={{
                          margin: "0 0 4px 0",
                          fontSize: "16px",
                          color: "#e0e0e0",
                        }}
                      >
                        {keyStatus.display_name}
                      </h3>
                      <div
                        style={{
                          fontSize: "12px",
                          color: keyStatus.is_configured ? "#4caf50" : "#999",
                        }}
                      >
                        {keyStatus.is_configured ? "✓ Configured" : "Not configured"}
                      </div>
                    </div>
                    <div style={{ display: "flex", gap: "8px" }}>
                      {keyStatus.is_configured && (
                        <button
                          onClick={() => handleDeleteKey(keyStatus.provider)}
                          disabled={saving}
                          style={{
                            ...buttonSecondary,
                            fontSize: "12px",
                            padding: "4px 12px",
                          }}
                        >
                          Delete
                        </button>
                      )}
                      <button
                        onClick={() => {
                          setEditingProvider(keyStatus.provider);
                          setNewKey("");
                        }}
                        disabled={saving}
                        style={{
                          ...buttonPrimary,
                          fontSize: "12px",
                          padding: "4px 12px",
                        }}
                      >
                        {keyStatus.is_configured ? "Update" : "Add"}
                      </button>
                    </div>
                  </div>

                  {editingProvider === keyStatus.provider && (
                    <div style={{ marginTop: "12px" }}>
                      <div style={{ marginBottom: "8px" }}>
                        <label
                          style={{
                            display: "block",
                            fontSize: "12px",
                            color: "#999",
                            marginBottom: "4px",
                          }}
                        >
                          API Key (e.g., {keyStatus.example_format})
                        </label>
                        <input
                          type="password"
                          value={newKey}
                          onChange={(e) => setNewKey(e.target.value)}
                          placeholder={keyStatus.example_format}
                          style={{
                            width: "100%",
                            padding: "8px",
                            background: "#1e1e1e",
                            border: "1px solid #444",
                            borderRadius: "4px",
                            color: "#e0e0e0",
                            fontSize: "14px",
                            fontFamily: "monospace",
                          }}
                          onKeyDown={(e) => {
                            if (e.key === "Enter") {
                              handleSaveKey();
                            } else if (e.key === "Escape") {
                              handleCancel();
                            }
                          }}
                          autoFocus
                        />
                      </div>
                      <div style={{ display: "flex", gap: "8px" }}>
                        <button
                          onClick={handleSaveKey}
                          disabled={saving || !newKey.trim()}
                          style={{
                            ...buttonPrimary,
                            fontSize: "12px",
                            padding: "6px 16px",
                          }}
                        >
                          {saving ? "Saving..." : "Save"}
                        </button>
                        <button
                          onClick={handleCancel}
                          disabled={saving}
                          style={{
                            ...buttonSecondary,
                            fontSize: "12px",
                            padding: "6px 16px",
                          }}
                        >
                          Cancel
                        </button>
                      </div>
                    </div>
                  )}
                </div>
              ))}
            </div>
          </div>
        )}

        <div
          style={{
            marginTop: "24px",
            paddingTop: "16px",
            borderTop: "1px solid #333",
          }}
        >
          <p style={{ fontSize: "12px", color: "#666", margin: 0 }}>
            💡 Tip: API keys are stored in your system's secure keychain
            (macOS Keychain, Windows Credential Manager, or Linux Secret Service).
            If that's unavailable, they're stored encrypted on disk.
          </p>
        </div>
      </div>
    </div>
  );
}
