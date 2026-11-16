"use client";

import { useEffect, useRef, useState } from "react";
import { MAX_USERNAME_LENGTH } from "@/lib/scoresApi";

type UsernameModalProps = {
  open: boolean;
  initialValue: string;
  isInitialSetup: boolean;
  busy: boolean;
  error: string | null;
  onSave: (username: string) => void;
  onClose: () => void;
  onSignOut: () => void;
};

export function UsernameModal({
  open,
  initialValue,
  isInitialSetup,
  busy,
  error,
  onSave,
  onClose,
  onSignOut,
}: UsernameModalProps) {
  const [value, setValue] = useState(initialValue);
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (open) {
      setValue(initialValue);
      const timer = setTimeout(() => {
        inputRef.current?.focus();
        inputRef.current?.select();
      }, 30);
      return () => clearTimeout(timer);
    }
  }, [open, initialValue]);

  if (!open) {
    return null;
  }

  const trimmed = value.trim();
  const canSubmit = trimmed.length >= 1 && trimmed.length <= MAX_USERNAME_LENGTH && !busy;

  const stopKeys = (e: React.KeyboardEvent) => {
    // Prevent Bevy's global/canvas key listeners (R, N, 1-4) from firing while typing.
    e.stopPropagation();
    if (e.key === "Escape" && !busy) {
      onClose();
    }
  };

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (canSubmit) {
      onSave(trimmed);
    }
  };

  const actionBtnStyle = (primary = false, disabled = false): React.CSSProperties => ({
    background: primary ? "#f2bf40" : "rgba(13, 15, 20, 0.9)",
    color: primary ? "#0d0f14" : "#f5efe0",
    border: `2px solid ${primary ? "#f2bf40" : "#3a445c"}`,
    boxShadow: "0 3px 0 #06070a",
    padding: "7px 12px",
    fontSize: "9px",
    fontFamily: "inherit",
    fontWeight: primary ? 700 : 500,
    letterSpacing: "0.06em",
    cursor: disabled ? "not-allowed" : "pointer",
    opacity: disabled ? 0.5 : 1,
    textTransform: "uppercase",
  });

  return (
    <div
      onKeyDown={stopKeys}
      onKeyUp={(e) => e.stopPropagation()}
      style={{
        position: "absolute",
        inset: 0,
        zIndex: 40,
        background: "rgba(7, 8, 11, 0.78)",
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        padding: "16px",
      }}
    >
      <form
        onSubmit={handleSubmit}
        style={{
          width: "min(380px, 100%)",
          background: "rgba(13, 15, 20, 0.97)",
          border: "3px solid #f2bf40",
          boxShadow: "0 6px 0 #07080b",
          padding: "16px 18px",
          display: "flex",
          flexDirection: "column",
          gap: "12px",
          fontSize: "10px",
        }}
      >
        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
          <strong style={{ color: "#f2bf40", letterSpacing: "0.08em", fontSize: "11px" }}>
            {isInitialSetup ? "SET ARCADE HANDLE" : "CHANGE ARCADE HANDLE"}
          </strong>
          <button
            type="button"
            onClick={onClose}
            disabled={busy}
            style={{
              background: "transparent",
              border: "none",
              color: "#f5efe0",
              cursor: busy ? "not-allowed" : "pointer",
              fontFamily: "inherit",
              fontSize: "10px",
            }}
          >
            [X]
          </button>
        </div>

        <div style={{ fontSize: "9px", lineHeight: 1.6, opacity: 0.85 }}>
          {isInitialSetup
            ? "Pick any handle for the Top 10 boards. Your local bests will sync right away."
            : "Pick any new handle — your existing Top 10 entries will update automatically."}
        </div>

        <div style={{ display: "flex", flexDirection: "column", gap: "6px" }}>
          <input
            ref={inputRef}
            type="text"
            value={value}
            maxLength={MAX_USERNAME_LENGTH}
            placeholder="E.G. DESK_BREAKER"
            disabled={busy}
            onChange={(e) => setValue(e.target.value)}
            onKeyDown={stopKeys}
            onKeyUp={(e) => e.stopPropagation()}
            style={{
              width: "100%",
              background: "#151924",
              color: "#f5efe0",
              border: "2px solid #f2bf40",
              padding: "9px 10px",
              fontSize: "11px",
              fontFamily: "inherit",
              letterSpacing: "0.06em",
              outline: "none",
              userSelect: "text",
              WebkitUserSelect: "text",
            }}
          />
          <div
            style={{
              display: "flex",
              justifyContent: "space-between",
              fontSize: "8px",
              opacity: 0.65,
              letterSpacing: "0.06em",
            }}
          >
            <span>ARCADE STYLE — PICK ANYTHING</span>
            <span>
              {trimmed.length}/{MAX_USERNAME_LENGTH}
            </span>
          </div>
        </div>

        {error && (
          <div style={{ color: "#ff6b6b", fontSize: "9px", lineHeight: 1.5 }}>{error}</div>
        )}

        <div style={{ display: "flex", justifyContent: "space-between", gap: "8px", marginTop: "2px" }}>
          {isInitialSetup ? (
            <button
              type="button"
              onClick={onSignOut}
              disabled={busy}
              style={actionBtnStyle(false, busy)}
            >
              SIGN OUT
            </button>
          ) : (
            <button
              type="button"
              onClick={onClose}
              disabled={busy}
              style={actionBtnStyle(false, busy)}
            >
              CANCEL
            </button>
          )}

          <button type="submit" disabled={!canSubmit} style={actionBtnStyle(true, !canSubmit)}>
            {busy ? "SAVING..." : "SAVE HANDLE"}
          </button>
        </div>
      </form>
    </div>
  );
}
