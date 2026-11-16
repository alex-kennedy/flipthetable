"use client";

import { useCallback, useEffect, useRef, useState } from "react";
import { ScoresDrawer } from "@/components/ScoresDrawer";
import { UsernameModal } from "@/components/UsernameModal";
import {
  DEFAULT_LEADERBOARD_LIMIT,
  getLevelHighScores,
  type LeaderboardEntry,
  MAX_USERNAME_LENGTH,
  signInWithGoogle,
  signOutUser,
  submitLevelScore,
  subscribeAuthState,
  syncLocalScores,
  setUsername as saveCloudUsername,
} from "@/lib/scoresApi";

type ScenarioEntry = {
  id: string;
  title: string;
  shortLabel: string;
  file: string;
};

type ScenarioManifest = {
  scenarios: ScenarioEntry[];
};

const DEFAULT_SCENARIOS: ScenarioEntry[] = [
  {
    id: "corporate",
    title: "CORPORATE CUBICLE",
    shortLabel: "1: OFFICE",
    file: "scenarios/corporate.json",
  },
  {
    id: "building_site",
    title: "BUILDING SITE TRAILER",
    shortLabel: "2: SITE",
    file: "scenarios/building_site.json",
  },
  {
    id: "boardroom",
    title: "EXECUTIVE BOARDROOM",
    shortLabel: "3: BOARDROOM",
    file: "scenarios/boardroom.json",
  },
  {
    id: "all_hands",
    title: "ALL-HANDS LAYOFF STAGE",
    shortLabel: "4: ALL-HANDS",
    file: "scenarios/all_hands.json",
  },
  {
    id: "breakroom",
    title: "SAD CORPORATE BREAKROOM",
    shortLabel: "5: BREAKROOM",
    file: "scenarios/breakroom.json",
  },
  {
    id: "warehouse",
    title: "FULFILLMENT SUPERVISOR CAGE",
    shortLabel: "6: WAREHOUSE",
    file: "scenarios/warehouse.json",
  },
];

type RunCompletePayload = {
  scenarioId: string;
  scenarioTitle: string;
  totalScore: number;
  itemsDestroyed: number;
  itemsToppled: number;
  maxCombo: number;
};

type LocalScoreStore = {
  bestByScenario: Record<string, { score: number; combo: number }>;
  recentRuns: Array<RunCompletePayload & { timestamp: number }>;
};

type WasmModule = {
  default: (
    moduleOrPath?: string | URL | Request | Response | Promise<Response>,
  ) => Promise<unknown>;
  start_game: (canvasSelector: string) => void;
  bridge_load_scenario: (id: string) => void;
  bridge_reset_round: () => void;
  bridge_set_muted: (muted: boolean) => void;
  bridge_set_leaderboard?: (json: string) => void;
};

type LoadStage = "fetching" | "booting" | "ready" | "error";

const STORAGE_KEY = "flipthetable_scores_v1";

function loadLocalScores(): LocalScoreStore {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (raw) {
      return JSON.parse(raw) as LocalScoreStore;
    }
  } catch {
    // Ignore storage read errors.
  }
  return { bestByScenario: {}, recentRuns: [] };
}

function saveLocalScores(store: LocalScoreStore) {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(store));
  } catch {
    // Ignore storage write errors.
  }
}

function extractFlatLocalScores(
  bestByScenario: Record<string, { score: number; combo: number }>,
): Record<string, number> {
  const flat: Record<string, number> = {};
  for (const [id, entry] of Object.entries(bestByScenario)) {
    if (entry.score > 0) {
      flat[id] = entry.score;
    }
  }
  return flat;
}

function mergeCloudIntoLocalStore(
  prev: LocalScoreStore,
  cloudScores: Record<string, number>,
): LocalScoreStore {
  let changed = false;
  const nextBest = { ...prev.bestByScenario };
  for (const [levelId, cloudScore] of Object.entries(cloudScores)) {
    const localBest = nextBest[levelId]?.score ?? 0;
    if (cloudScore > localBest) {
      nextBest[levelId] = {
        score: cloudScore,
        combo: nextBest[levelId]?.combo ?? 1,
      };
      changed = true;
    }
  }
  if (!changed) {
    return prev;
  }
  const next: LocalScoreStore = {
    ...prev,
    bestByScenario: nextBest,
  };
  saveLocalScores(next);
  return next;
}

export function GameCanvas() {
  const containerRef = useRef<HTMLDivElement>(null);
  const wasmRef = useRef<WasmModule | null>(null);
  const startedRef = useRef(false);

  const [stage, setStage] = useState<LoadStage>("fetching");
  const [errorMsg, setErrorMsg] = useState<string | null>(null);
  const [isFullscreen, setIsFullscreen] = useState(false);
  const [scenarios, setScenarios] = useState<ScenarioEntry[]>(DEFAULT_SCENARIOS);
  const [scenario, setScenario] = useState<string>("corporate");
  const [muted, setMuted] = useState(false);
  const [showScores, setShowScores] = useState(false);
  const [scores, setScores] = useState<LocalScoreStore>({ bestByScenario: {}, recentRuns: [] });
  const [newRecordBanner, setNewRecordBanner] = useState<string | null>(null);

  const [currentUid, setCurrentUid] = useState<string | null>(null);
  const [username, setUsername] = useState<string | null>(null);
  const [suggestedHandle, setSuggestedHandle] = useState<string>("");
  const [cloudScores, setCloudScores] = useState<Record<string, number>>({});
  const [authBusy, setAuthBusy] = useState<boolean>(false);
  const [authError, setAuthError] = useState<string | null>(null);
  const [showUsernameModal, setShowUsernameModal] = useState<boolean>(false);
  const [usernameBusy, setUsernameBusy] = useState<boolean>(false);
  const [usernameError, setUsernameError] = useState<string | null>(null);
  const [boardRefreshToken, setBoardRefreshToken] = useState<number>(0);
  const [hudBoardRows, setHudBoardRows] = useState<Record<string, LeaderboardEntry[]>>({});
  const [hudBoardLoading, setHudBoardLoading] = useState<boolean>(true);

  const authRef = useRef<{
    uid: string | null;
    username: string | null;
    cloudScores: Record<string, number>;
  }>({
    uid: null,
    username: null,
    cloudScores: {},
  });

  useEffect(() => {
    authRef.current = { uid: currentUid, username, cloudScores };
  }, [currentUid, username, cloudScores]);

  useEffect(() => {
    setScores(loadLocalScores());
    fetch("/assets/scenarios/manifest.json")
      .then((res) => (res.ok ? (res.json() as Promise<ScenarioManifest>) : null))
      .then((manifest) => {
        if (manifest?.scenarios?.length) {
          setScenarios(manifest.scenarios);
        }
      })
      .catch(() => {
        // Fall back to built-in scenario list.
      });
  }, []);

  // Subscribe to Firebase Google Auth state & sync localStorage scores on sign-in.
  useEffect(() => {
    const unsub = subscribeAuthState(async (user) => {
      setAuthError(null);
      if (!user) {
        setCurrentUid(null);
        setUsername(null);
        setCloudScores({});
        setShowUsernameModal(false);
        return;
      }

      setCurrentUid(user.uid);
      const fallbackHandle = (user.displayName ?? "").trim().slice(0, MAX_USERNAME_LENGTH);
      setSuggestedHandle(fallbackHandle);

      try {
        const localNow = loadLocalScores();
        const synced = await syncLocalScores(extractFlatLocalScores(localNow.bestByScenario));
        if (synced) {
          setUsername(synced.username);
          setCloudScores(synced.scores);
          setScores((prev) => mergeCloudIntoLocalStore(prev, synced.scores));
          setBoardRefreshToken((n) => n + 1);
        } else {
          // Signed in for the first time — prompt player to choose any arcade handle.
          setUsername(null);
          setCloudScores({});
          setUsernameError(null);
          setShowUsernameModal(true);
        }
      } catch (err) {
        const msg = err instanceof Error ? err.message : "Failed to sync profile.";
        setAuthError(msg);
      }
    });

    return () => unsub();
  }, []);

  // Fetch active scenario's leaderboard for the top-left Top 5 HUD box.
  useEffect(() => {
    let cancelled = false;
    setHudBoardLoading(true);
    getLevelHighScores(scenario, DEFAULT_LEADERBOARD_LIMIT)
      .then((rows) => {
        if (cancelled) return;
        setHudBoardRows((prev) => ({ ...prev, [scenario]: rows }));
      })
      .catch(() => {
        // Leave empty/cached rows if offline.
      })
      .finally(() => {
        if (!cancelled) {
          setHudBoardLoading(false);
        }
      });

    return () => {
      cancelled = true;
    };
  }, [scenario, boardRefreshToken]);

  // Push Top 5 + My High Score into Bevy's top-left HUD whenever data or scenario changes.
  useEffect(() => {
    if (stage !== "ready") return;
    const myBest = Math.max(
      scores.bestByScenario[scenario]?.score ?? 0,
      cloudScores[scenario] ?? 0,
    );
    const rows = (hudBoardRows[scenario] ?? []).slice(0, 5).map((r) => ({
      username: r.username,
      score: r.score,
      isMe: Boolean(currentUid && r.uid === currentUid),
    }));
    wasmRef.current?.bridge_set_leaderboard?.(
      JSON.stringify({
        myHighScore: myBest,
        topScores: rows,
        loading: hudBoardLoading && rows.length === 0,
      }),
    );
  }, [stage, scenario, scores.bestByScenario, cloudScores, hudBoardRows, hudBoardLoading, currentUid]);

  useEffect(() => {
    const onReady = () => setStage("ready");
    const onFullscreenChange = () => setIsFullscreen(Boolean(document.fullscreenElement));

    const onScenarioChanged = (e: Event) => {
      const detail = (e as CustomEvent<string>).detail;
      if (detail) {
        setScenario(detail);
      }
    };

    const onRunComplete = (e: Event) => {
      try {
        const payload = JSON.parse((e as CustomEvent<string>).detail) as RunCompletePayload;
        const prevCloudBest = authRef.current.cloudScores[payload.scenarioId] ?? 0;

        setScores((prev) => {
          const prevLocalBest = prev.bestByScenario[payload.scenarioId]?.score ?? 0;
          const prevOverallBest = Math.max(prevLocalBest, prevCloudBest);
          const isNewBest = payload.totalScore > prevOverallBest && payload.totalScore > 0;
          if (isNewBest) {
            setNewRecordBanner(`NEW ${payload.scenarioTitle} RECORD: ${payload.totalScore}!`);
            setTimeout(() => setNewRecordBanner(null), 3200);
          }

          const next: LocalScoreStore = {
            bestByScenario: {
              ...prev.bestByScenario,
              [payload.scenarioId]: {
                score: Math.max(prevOverallBest, payload.totalScore),
                combo: Math.max(
                  prev.bestByScenario[payload.scenarioId]?.combo ?? 1,
                  payload.maxCombo,
                ),
              },
            },
            recentRuns: [{ ...payload, timestamp: Date.now() }, ...prev.recentRuns].slice(0, 8),
          };
          saveLocalScores(next);
          return next;
        });

        if (
          authRef.current.uid &&
          authRef.current.username &&
          payload.totalScore > prevCloudBest &&
          payload.totalScore > 0
        ) {
          void submitLevelScore(payload.scenarioId, payload.totalScore)
            .then((res) => {
              if (res.updated) {
                setCloudScores((prev) => ({
                  ...prev,
                  [payload.scenarioId]: res.bestScore,
                }));
                setBoardRefreshToken((n) => n + 1);
              }
            })
            .catch(() => {
              // Local score remains stored; next session sign-in will retry sync.
            });
        }
      } catch {
        // Ignore malformed event payloads.
      }
    };

    window.addEventListener("flipthetable:ready", onReady);
    window.addEventListener("flipthetable:scenario_changed", onScenarioChanged);
    window.addEventListener("flipthetable:run_complete", onRunComplete);
    document.addEventListener("fullscreenchange", onFullscreenChange);

    if (!startedRef.current) {
      startedRef.current = true;

      (async () => {
        try {
          const wasmUrl = "/wasm/flipthetable_game.js";
          const mod = (await import(/* webpackIgnore: true */ wasmUrl)) as WasmModule;
          const res = await fetch("/wasm/flipthetable_game_bg.wasm.gz");
          if (!res.ok) {
            throw new Error(`Failed to fetch WASM (${res.status} ${res.statusText})`);
          }
          const contentEncoding = res.headers.get("content-encoding") ?? "";
          let wasmResponse: Response;
          if (contentEncoding.includes("gzip")) {
            wasmResponse = new Response(res.body, {
              headers: { "Content-Type": "application/wasm" },
            });
          } else {
            if (!res.body || typeof DecompressionStream === "undefined") {
              throw new Error("Browser does not support DecompressionStream");
            }
            const decompressed = res.body.pipeThrough(new DecompressionStream("gzip"));
            wasmResponse = new Response(decompressed, {
              headers: { "Content-Type": "application/wasm" },
            });
          }
          await mod.default(wasmResponse);
          wasmRef.current = mod;
          setStage("booting");
          mod.start_game("#bevy-canvas");
        } catch (err) {
          const msg = err instanceof Error ? err.message : String(err);
          if (!msg.includes("Using exceptions for control flow")) {
            setStage("error");
            setErrorMsg(msg);
          }
        }
      })();
    }

    return () => {
      window.removeEventListener("flipthetable:ready", onReady);
      window.removeEventListener("flipthetable:scenario_changed", onScenarioChanged);
      window.removeEventListener("flipthetable:run_complete", onRunComplete);
      document.removeEventListener("fullscreenchange", onFullscreenChange);
    };
  }, []);

  const handleSelectScenario = useCallback((id: string) => {
    setScenario(id);
    wasmRef.current?.bridge_load_scenario(id);
  }, []);

  const handleReset = useCallback(() => {
    wasmRef.current?.bridge_reset_round();
  }, []);

  const handleToggleMute = useCallback(() => {
    setMuted((prev) => {
      const next = !prev;
      wasmRef.current?.bridge_set_muted(next);
      return next;
    });
  }, []);

  const toggleFullscreen = useCallback(async () => {
    const el = containerRef.current;
    if (!el) return;
    try {
      if (!document.fullscreenElement) {
        await el.requestFullscreen();
      } else {
        await document.exitFullscreen();
      }
    } catch {
      // Ignore fullscreen rejections.
    }
  }, []);

  const handleSignIn = useCallback(async () => {
    setAuthBusy(true);
    setAuthError(null);
    try {
      await signInWithGoogle();
    } catch (err) {
      const code = (err as { code?: string })?.code ?? "";
      if (code !== "auth/popup-closed-by-user" && code !== "auth/cancelled-popup-request") {
        setAuthError(err instanceof Error ? err.message : "Google sign-in failed.");
      }
    } finally {
      setAuthBusy(false);
    }
  }, []);

  const handleSignOut = useCallback(async () => {
    setAuthError(null);
    try {
      await signOutUser();
      setShowUsernameModal(false);
    } catch (err) {
      setAuthError(err instanceof Error ? err.message : "Failed to sign out.");
    }
  }, []);

  const handleSaveUsername = useCallback(async (chosenHandle: string) => {
    setUsernameBusy(true);
    setUsernameError(null);
    try {
      const localNow = loadLocalScores();
      const updated = await saveCloudUsername(
        chosenHandle,
        extractFlatLocalScores(localNow.bestByScenario),
      );
      setUsername(updated.username);
      setCloudScores(updated.scores);
      setScores((prev) => mergeCloudIntoLocalStore(prev, updated.scores));
      setShowUsernameModal(false);
      setBoardRefreshToken((n) => n + 1);
    } catch (err) {
      setUsernameError(err instanceof Error ? err.message : "Failed to save handle.");
    } finally {
      setUsernameBusy(false);
    }
  }, []);

  const isOverlayVisible = stage !== "ready";

  const pillBtnStyle = (active = false): React.CSSProperties => ({
    background: active ? "#f2bf40" : "rgba(13, 15, 20, 0.86)",
    color: active ? "#0d0f14" : "#f2bf40",
    border: "2px solid #f2bf40",
    boxShadow: "0 3px 0 #06070a",
    padding: "6px 9px",
    fontSize: "9px",
    fontFamily: "inherit",
    fontWeight: active ? 700 : 500,
    letterSpacing: "0.06em",
    cursor: "pointer",
    textTransform: "uppercase",
  });

  return (
    <div
      ref={containerRef}
      style={{
        position: "relative",
        width: "100%",
        height: "100%",
        overflow: "hidden",
        background: "#0d0f14",
      }}
    >
      <canvas
        id="bevy-canvas"
        tabIndex={0}
        style={{
          width: "100%",
          height: "100%",
          display: "block",
          outline: "none",
          imageRendering: "pixelated",
        }}
      />

      {/* Subtle retro CRT scanlines & vignette over the 3D canvas */}
      <div
        className="scanlines"
        style={{
          position: "absolute",
          inset: 0,
          pointerEvents: "none",
          opacity: 0.28,
          boxShadow: "inset 0 0 85px rgba(0, 0, 0, 0.48)",
          zIndex: 5,
        }}
      />

      {/* New High Score Banner */}
      {newRecordBanner && (
        <div
          style={{
            position: "absolute",
            top: "16px",
            left: "50%",
            transform: "translateX(-50%)",
            zIndex: 25,
            background: "#f2bf40",
            color: "#0d0f14",
            border: "2px solid #f5efe0",
            padding: "6px 14px",
            fontSize: "10px",
            fontWeight: 700,
            letterSpacing: "0.08em",
            boxShadow: "0 4px 0 #07080b",
            pointerEvents: "none",
          }}
        >
          {newRecordBanner}
        </div>
      )}

      {/* Data-driven top-right shell controls */}
      <div
        style={{
          position: "absolute",
          top: "14px",
          right: "14px",
          maxWidth: "calc(100vw - 265px)",
          zIndex: 20,
          display: "flex",
          flexWrap: "wrap",
          justifyContent: "flex-end",
          gap: "6px",
        }}
      >
        {scenarios.map((entry) => (
          <button
            key={entry.id}
            type="button"
            onClick={() => handleSelectScenario(entry.id)}
            style={pillBtnStyle(scenario === entry.id)}
          >
            {entry.shortLabel}
          </button>
        ))}
        <button type="button" onClick={handleReset} style={pillBtnStyle(false)}>
          RETRY
        </button>
        <button type="button" onClick={handleToggleMute} style={pillBtnStyle(muted)}>
          {muted ? "MUTED" : "SFX ON"}
        </button>
        {!currentUid ? (
          <button
            type="button"
            onClick={handleSignIn}
            disabled={authBusy}
            style={pillBtnStyle(false)}
          >
            {authBusy ? "WAIT..." : "GOOGLE LOGIN"}
          </button>
        ) : (
          <button
            type="button"
            onClick={() => {
              setUsernameError(null);
              setShowUsernameModal(true);
            }}
            style={pillBtnStyle(!username)}
          >
            {username ? `ID: ${username}` : "SET HANDLE"}
          </button>
        )}
        <button type="button" onClick={() => setShowScores((s) => !s)} style={pillBtnStyle(showScores)}>
          SCORES
        </button>
        <button type="button" onClick={toggleFullscreen} style={pillBtnStyle(isFullscreen)}>
          {isFullscreen ? "EXIT FULL" : "FULLSCREEN"}
        </button>
      </div>

      {/* Global Top 10 & Personal High Scores Drawer */}
      {showScores && (
        <ScoresDrawer
          scenarios={scenarios}
          activeScenario={scenario}
          localBestByScenario={scores.bestByScenario}
          cloudScores={cloudScores}
          recentRuns={scores.recentRuns}
          currentUid={currentUid}
          username={username}
          authBusy={authBusy}
          authError={authError}
          refreshToken={boardRefreshToken}
          onClose={() => setShowScores(false)}
          onSignIn={handleSignIn}
          onSignOut={handleSignOut}
          onOpenUsernameModal={() => {
            setUsernameError(null);
            setShowUsernameModal(true);
          }}
        />
      )}

      {/* Arcade Handle Picker / Editor Modal */}
      <UsernameModal
        open={showUsernameModal}
        initialValue={username ?? suggestedHandle}
        isInitialSetup={!username}
        busy={usernameBusy}
        error={usernameError}
        onSave={handleSaveUsername}
        onClose={() => setShowUsernameModal(false)}
        onSignOut={handleSignOut}
      />

      {/* Retro pixel-art loading overlay */}
      <div
        aria-hidden={!isOverlayVisible}
        style={{
          position: "absolute",
          inset: 0,
          zIndex: 30,
          display: "flex",
          flexDirection: "column",
          alignItems: "center",
          justifyContent: "flex-end",
          padding: "clamp(24px, 5vw, 56px)",
          opacity: isOverlayVisible ? 1 : 0,
          pointerEvents: isOverlayVisible ? "auto" : "none",
          transition: "opacity 450ms ease-out",
        }}
      >
        <img
          src="/images/loading-desk.jpg"
          alt="Retro pixel-art office desk"
          style={{
            position: "absolute",
            inset: 0,
            width: "100%",
            height: "100%",
            objectFit: "cover",
            objectPosition: "center",
            imageRendering: "pixelated",
          }}
        />

        <div
          className="scanlines"
          style={{
            position: "absolute",
            inset: 0,
            backgroundColor: "rgba(13, 15, 20, 0.42)",
            boxShadow: "inset 0 0 120px rgba(0, 0, 0, 0.85)",
          }}
        />

        <div
          style={{
            position: "relative",
            zIndex: 2,
            width: "min(440px, 100%)",
            background: "rgba(13, 15, 20, 0.88)",
            border: "3px solid #f2bf40",
            boxShadow: "0 6px 0 #07080b",
            padding: "16px 20px",
            display: "flex",
            flexDirection: "column",
            gap: "12px",
          }}
        >
          <div style={{ display: "flex", alignItems: "center", gap: "14px" }}>
            <svg
              width="36"
              height="36"
              viewBox="0 0 18 18"
              shapeRendering="crispEdges"
              aria-hidden="true"
              style={{ flexShrink: 0 }}
            >
              <g fill="#f5efe0">
                <rect className="steam-1" x="6" y="4" width="1" height="2" />
                <rect className="steam-2" x="8" y="3" width="1" height="2" />
                <rect className="steam-3" x="10" y="4" width="1" height="2" />
              </g>
              <rect x="4" y="7" width="8" height="8" fill="#f5efe0" />
              <rect x="5" y="7" width="6" height="1" fill="#6b3e26" />
              <rect x="12" y="9" width="2" height="1" fill="#d9cfc0" />
              <rect x="13" y="10" width="1" height="2" fill="#d9cfc0" />
              <rect x="12" y="12" width="2" height="1" fill="#d9cfc0" />
              <rect x="6" y="10" width="1" height="1" fill="#1a1f2b" />
              <rect x="9" y="10" width="1" height="1" fill="#1a1f2b" />
              <rect x="7" y="12" width="2" height="1" fill="#f2bf40" />
              <rect x="4" y="15" width="8" height="1" fill="#b8ae9e" />
            </svg>

            <div style={{ flex: 1, minWidth: 0 }}>
              <h1
                style={{
                  margin: 0,
                  fontSize: "clamp(13px, 2.2vw, 16px)",
                  letterSpacing: "0.08em",
                  color: "#f2bf40",
                }}
              >
                FLIP THE TABLE
              </h1>
              <div
                style={{
                  marginTop: "4px",
                  fontSize: "9px",
                  letterSpacing: "0.06em",
                  color: "#f5efe0",
                  opacity: 0.78,
                }}
              >
                {stage === "fetching" ? "BREWING WASM ENGINE..." : "SETTING UP THE DESK..."}
              </div>
            </div>
          </div>

          {stage === "error" ? (
            <div style={{ color: "#ff6b6b", fontSize: "10px", lineHeight: 1.5 }}>
              <strong>FAILED TO LOAD ENGINE</strong>
              <div style={{ marginTop: "4px", opacity: 0.9 }}>{errorMsg}</div>
            </div>
          ) : (
            <div
              style={{
                width: "100%",
                height: "8px",
                background: "#1a1f2b",
                border: "2px solid #3a445c",
                overflow: "hidden",
              }}
            >
              <div
                className="loading-bar-indeterminate"
                style={{
                  width: "35%",
                  height: "100%",
                  background: "#f2bf40",
                }}
              />
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
