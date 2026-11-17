"use client";

import { useCallback, useEffect, useState } from "react";
import {
  DEFAULT_LEADERBOARD_LIMIT,
  getLevelHighScores,
  type LeaderboardEntry,
} from "@/lib/scoresApi";

export type DrawerScenarioEntry = {
  id: string;
  title: string;
  shortLabel: string;
};

export type DrawerRecentRun = {
  scenarioId: string;
  scenarioTitle: string;
  totalScore: number;
  itemsDestroyed: number;
  timestamp: number;
};

type ScoresDrawerProps = {
  scenarios: DrawerScenarioEntry[];
  activeScenario: string;
  localBestByScenario: Record<string, { score: number; combo: number }>;
  cloudScores: Record<string, number>;
  recentRuns: DrawerRecentRun[];
  currentUid: string | null;
  username: string | null;
  authBusy: boolean;
  authError: string | null;
  refreshToken: number;
  onClose: () => void;
  onSignIn: () => void;
  onSignOut: () => void;
  onOpenUsernameModal: () => void;
};

export function ScoresDrawer({
  scenarios,
  activeScenario,
  localBestByScenario,
  cloudScores,
  recentRuns,
  currentUid,
  username,
  authBusy,
  authError,
  refreshToken,
  onClose,
  onSignIn,
  onSignOut,
  onOpenUsernameModal,
}: ScoresDrawerProps) {
  const [tab, setTab] = useState<"top10" | "personal">("top10");
  const [boardLevel, setBoardLevel] = useState<string>(activeScenario);
  const [entries, setEntries] = useState<LeaderboardEntry[]>([]);
  const [loadingBoard, setLoadingBoard] = useState<boolean>(false);
  const [boardError, setBoardError] = useState<string | null>(null);

  useEffect(() => {
    setBoardLevel(activeScenario);
  }, [activeScenario]);

  const fetchBoard = useCallback(async (levelId: string) => {
    setLoadingBoard(true);
    setBoardError(null);
    try {
      const rows = await getLevelHighScores(levelId, DEFAULT_LEADERBOARD_LIMIT);
      setEntries(rows);
    } catch (err) {
      const msg = err instanceof Error ? err.message : "Failed to load high scores.";
      setBoardError(msg);
    } finally {
      setLoadingBoard(false);
    }
  }, []);

  useEffect(() => {
    if (tab === "top10") {
      void fetchBoard(boardLevel);
    }
  }, [tab, boardLevel, refreshToken, fetchBoard]);

  const smallBtnStyle = (active = false): React.CSSProperties => ({
    background: active ? "#f2bf40" : "rgba(21, 25, 36, 0.92)",
    color: active ? "#0d0f14" : "#f2bf40",
    border: `1px solid ${active ? "#f2bf40" : "#3a445c"}`,
    padding: "4px 7px",
    fontSize: "8px",
    fontFamily: "inherit",
    fontWeight: active ? 700 : 500,
    letterSpacing: "0.06em",
    cursor: "pointer",
    textTransform: "uppercase",
  });

  const selectedScenarioTitle =
    scenarios.find((s) => s.id === boardLevel)?.title ?? boardLevel.toUpperCase();

  return (
    <div
      style={{
        position: "absolute",
        top: "54px",
        right: "14px",
        zIndex: 25,
        width: "min(390px, calc(100vw - 28px))",
        maxHeight: "calc(100dvh - 70px)",
        overflowY: "auto",
        background: "rgba(13, 15, 20, 0.95)",
        border: "2px solid #f2bf40",
        boxShadow: "0 6px 0 #07080b",
        padding: "14px 16px",
        display: "flex",
        flexDirection: "column",
        gap: "10px",
        fontSize: "10px",
      }}
    >
      {/* Header */}
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
        <strong style={{ color: "#f2bf40", letterSpacing: "0.08em" }}>HIGH SCORES</strong>
        <button
          type="button"
          onClick={onClose}
          style={{
            background: "transparent",
            border: "none",
            color: "#f5efe0",
            cursor: "pointer",
            fontFamily: "inherit",
          }}
        >
          [X]
        </button>
      </div>

      {/* Account Bar */}
      <div
        style={{
          background: "#151924",
          border: "1px solid #2b3345",
          padding: "8px 10px",
          display: "flex",
          flexDirection: "column",
          gap: "6px",
        }}
      >
        {!currentUid ? (
          <div
            style={{
              display: "flex",
              justifyContent: "space-between",
              alignItems: "center",
              gap: "8px",
            }}
          >
            <span style={{ fontSize: "8px", opacity: 0.8, lineHeight: 1.4 }}>
              SIGN IN WITH GOOGLE TO SYNC & POST SCORES
            </span>
            <button
              type="button"
              onClick={onSignIn}
              disabled={authBusy}
              style={{
                ...smallBtnStyle(true),
                flexShrink: 0,
                opacity: authBusy ? 0.6 : 1,
              }}
            >
              {authBusy ? "WAIT..." : "GOOGLE LOGIN"}
            </button>
          </div>
        ) : (
          <div
            style={{
              display: "flex",
              justifyContent: "space-between",
              alignItems: "center",
              gap: "6px",
              flexWrap: "wrap",
            }}
          >
            <span style={{ fontSize: "9px", color: "#8be9fd" }}>
              {authBusy
                ? "SYNCING CLOUD PROFILE..."
                : username
                  ? `HANDLE: ${username}`
                  : "NO HANDLE SET — PICK ONE TO SYNC"}
            </span>
            <div style={{ display: "flex", gap: "5px" }}>
              <button
                type="button"
                onClick={onOpenUsernameModal}
                disabled={authBusy}
                style={smallBtnStyle(!username && !authBusy)}
              >
                {username ? "RENAME" : "SET HANDLE"}
              </button>
              <button type="button" onClick={onSignOut} style={smallBtnStyle(false)}>
                LOGOUT
              </button>
            </div>
          </div>
        )}

        {authError && (
          <div style={{ color: "#ff6b6b", fontSize: "8px", lineHeight: 1.4 }}>{authError}</div>
        )}
      </div>

      {/* Mode Tabs */}
      <div style={{ display: "flex", gap: "6px" }}>
        <button
          type="button"
          onClick={() => setTab("top10")}
          style={{ ...smallBtnStyle(tab === "top10"), flex: 1, padding: "6px 8px" }}
        >
          GLOBAL TOP 10
        </button>
        <button
          type="button"
          onClick={() => setTab("personal")}
          style={{ ...smallBtnStyle(tab === "personal"), flex: 1, padding: "6px 8px" }}
        >
          MY BESTS
        </button>
      </div>

      {tab === "top10" ? (
        <div style={{ display: "flex", flexDirection: "column", gap: "8px" }}>
          {/* Level Selector */}
          <div style={{ display: "flex", flexWrap: "wrap", gap: "4px" }}>
            {scenarios.map((s) => (
              <button
                key={s.id}
                type="button"
                onClick={() => setBoardLevel(s.id)}
                style={smallBtnStyle(boardLevel === s.id)}
              >
                {s.shortLabel}
              </button>
            ))}
          </div>

          <div
            style={{
              display: "flex",
              justifyContent: "space-between",
              alignItems: "center",
              borderBottom: "1px solid #2b3345",
              paddingBottom: "5px",
            }}
          >
            <span style={{ fontSize: "9px", color: "#f2bf40", letterSpacing: "0.06em" }}>
              {selectedScenarioTitle}
            </span>
            <button
              type="button"
              onClick={() => void fetchBoard(boardLevel)}
              disabled={loadingBoard}
              style={{
                background: "transparent",
                border: "none",
                color: "#8be9fd",
                cursor: loadingBoard ? "default" : "pointer",
                fontFamily: "inherit",
                fontSize: "8px",
              }}
            >
              {loadingBoard ? "LOADING..." : "[REFRESH]"}
            </button>
          </div>

          {boardError ? (
            <div style={{ color: "#ff6b6b", fontSize: "9px", lineHeight: 1.5 }}>{boardError}</div>
          ) : loadingBoard && entries.length === 0 ? (
            <div style={{ opacity: 0.7, fontSize: "9px", padding: "8px 0" }}>
              FETCHING TOP 10...
            </div>
          ) : entries.length === 0 ? (
            <div style={{ opacity: 0.7, fontSize: "9px", padding: "8px 0", lineHeight: 1.5 }}>
              NO SCORES LOGGED FOR THIS LEVEL YET — FLIP THE TABLE!
            </div>
          ) : (
            <div style={{ display: "flex", flexDirection: "column", gap: "5px" }}>
              {entries.map((row, idx) => {
                const isMe = Boolean(currentUid && row.uid === currentUid);
                return (
                  <div
                    key={row.uid}
                    style={{
                      display: "flex",
                      justifyContent: "space-between",
                      alignItems: "center",
                      padding: "3px 5px",
                      background: isMe ? "rgba(242, 191, 64, 0.14)" : "transparent",
                      borderLeft: isMe ? "2px solid #f2bf40" : "2px solid transparent",
                    }}
                  >
                    <span
                      style={{
                        color: isMe ? "#8be9fd" : "#f5efe0",
                        overflow: "hidden",
                        textOverflow: "ellipsis",
                        whiteSpace: "nowrap",
                        paddingRight: "8px",
                      }}
                    >
                      #{String(idx + 1).padStart(2, "0")} {row.username}
                      {isMe ? " (YOU)" : ""}
                    </span>
                    <span style={{ color: "#f2bf40", flexShrink: 0 }}>{row.score} PTS</span>
                  </div>
                );
              })}
            </div>
          )}
        </div>
      ) : (
        <>
          <div style={{ display: "flex", flexDirection: "column", gap: "6px" }}>
            {scenarios.map((entry) => {
              const best = Math.max(
                localBestByScenario[entry.id]?.score ?? 0,
                cloudScores[entry.id] ?? 0,
              );
              return (
                <div key={entry.id} style={{ display: "flex", justifyContent: "space-between" }}>
                  <span>{entry.title}</span>
                  <span style={{ color: "#f2bf40" }}>{best} PTS</span>
                </div>
              );
            })}
          </div>

          {recentRuns.length > 0 && (
            <div
              style={{
                borderTop: "1px solid #2b3345",
                paddingTop: "8px",
                display: "flex",
                flexDirection: "column",
                gap: "5px",
              }}
            >
              <span style={{ fontSize: "9px", opacity: 0.65, letterSpacing: "0.08em" }}>
                RECENT FLIPS
              </span>
              {recentRuns.slice(0, 4).map((run, idx) => (
                <div
                  key={`${run.timestamp}-${idx}`}
                  style={{
                    display: "flex",
                    justifyContent: "space-between",
                    fontSize: "9px",
                    opacity: 0.88,
                  }}
                >
                  <span>
                    {run.scenarioTitle} ({run.itemsDestroyed}X)
                  </span>
                  <span style={{ color: "#8be9fd" }}>{run.totalScore}</span>
                </div>
              ))}
            </div>
          )}
        </>
      )}
    </div>
  );
}
