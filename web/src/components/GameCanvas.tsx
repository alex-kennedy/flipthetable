"use client";

import { useCallback, useEffect, useRef, useState } from "react";
import { ScoresDrawer } from "@/components/ScoresDrawer";
import { UsernameModal } from "@/components/UsernameModal";
import {
  buildStoreFromScoresAndRuns,
  clearAllPendingUserScores,
  clearGuestScores,
  clearPendingUserScore,
  extractFlatScores,
  loadGuestScores,
  loadPendingUserScores,
  loadUserRecentRuns,
  type LocalScoreStore,
  mergeRecentRuns,
  mergeScoreMaps,
  recordPendingUserScore,
  saveGuestScores,
  saveUserRecentRuns,
} from "@/lib/localScores";
import { trackAnalyticsEvent } from "@/lib/firebase";
import {
  DEFAULT_LEADERBOARD_LIMIT,
  formatAuthError,
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

export function GameCanvas() {
  const containerRef = useRef<HTMLDivElement>(null);
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const wasmRef = useRef<WasmModule | null>(null);
  const startedRef = useRef(false);
  const authSeqRef = useRef<number>(0);
  const bannerTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

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
  const [authSyncing, setAuthSyncing] = useState<boolean>(false);
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

  const focusCanvas = useCallback(() => {
    const canvas = canvasRef.current;
    if (canvas && document.activeElement !== canvas) {
      canvas.focus({ preventScroll: true });
    }
  }, []);

  const showBanner = useCallback((message: string, durationMs = 3200) => {
    if (bannerTimerRef.current) {
      clearTimeout(bannerTimerRef.current);
    }
    setNewRecordBanner(message);
    bannerTimerRef.current = setTimeout(() => {
      setNewRecordBanner(null);
      bannerTimerRef.current = null;
    }, durationMs);
  }, []);

  useEffect(() => {
    authRef.current = { uid: currentUid, username, cloudScores };
  }, [currentUid, username, cloudScores]);

  useEffect(() => {
    setScores(loadGuestScores());
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

    return () => {
      if (bannerTimerRef.current) {
        clearTimeout(bannerTimerRef.current);
      }
    };
  }, []);

  // Subscribe to Firebase Google Auth state & safely sync guest/account localStorage scores.
  useEffect(() => {
    const unsub = subscribeAuthState(async (user) => {
      const seq = ++authSeqRef.current;
      setAuthError(null);

      if (!user) {
        authRef.current = { uid: null, username: null, cloudScores: {} };
        setCurrentUid(null);
        setUsername(null);
        setSuggestedHandle("");
        setCloudScores({});
        setAuthSyncing(false);
        setShowUsernameModal(false);
        setUsernameError(null);
        setScores(loadGuestScores());
        return;
      }

      const uid = user.uid;
      const fallbackHandle = (user.displayName ?? "").trim().slice(0, MAX_USERNAME_LENGTH);
      setCurrentUid(uid);
      setSuggestedHandle(fallbackHandle);
      setAuthSyncing(true);

      // Combine ONLY genuinely unsynced runs (pending offline runs + unclaimed guest runs).
      const pendingMap = loadPendingUserScores(uid);
      const guestLocal = loadGuestScores();
      const unsyncedScores = mergeScoreMaps(pendingMap, extractFlatScores(guestLocal));
      const mergedRuns = mergeRecentRuns(loadUserRecentRuns(uid), guestLocal.recentRuns);
      const hadGuestData =
        Object.keys(guestLocal.bestByScenario).length > 0 || guestLocal.recentRuns.length > 0;
      setScores(buildStoreFromScoresAndRuns(unsyncedScores, mergedRuns));

      try {
        const synced = await syncLocalScores(unsyncedScores, uid);
        if (authSeqRef.current !== seq) {
          return;
        }

        if (synced) {
          clearAllPendingUserScores(uid);
          if (hadGuestData) {
            clearGuestScores();
          }
          saveUserRecentRuns(uid, mergedRuns);
          authRef.current = {
            uid,
            username: synced.profile.username,
            cloudScores: synced.profile.scores,
          };
          setUsername(synced.profile.username);
          setCloudScores(synced.profile.scores);
          setScores(buildStoreFromScoresAndRuns(synced.profile.scores, mergedRuns));
          setBoardRefreshToken((n) => n + 1);
          if (synced.uploadedCount > 0) {
            showBanner(
              `SYNCED ${synced.uploadedCount} LOCAL RECORD${synced.uploadedCount > 1 ? "S" : ""} TO ${synced.profile.username}!`,
            );
          }
        } else {
          // First-time Google user — keep guest store intact until handle is saved.
          authRef.current = { uid, username: null, cloudScores: {} };
          setUsername(null);
          setCloudScores({});
          setUsernameError(null);
          setShowUsernameModal(true);
        }
      } catch (err) {
        if (authSeqRef.current !== seq) {
          return;
        }
        setAuthError(formatAuthError(err, "Failed to sync cloud profile."));
      } finally {
        if (authSeqRef.current === seq) {
          setAuthSyncing(false);
        }
      }
    });

    return () => unsub();
  }, [showBanner]);

  // Auto-resync any offline signed-in runs when network connectivity returns.
  useEffect(() => {
    const onOnline = () => {
      const { uid, username: activeName } = authRef.current;
      if (!uid || !activeName) return;
      const pendingMap = loadPendingUserScores(uid);
      if (Object.keys(pendingMap).length === 0) return;
      void syncLocalScores(pendingMap, uid)
        .then((res) => {
          if (!res || authRef.current.uid !== uid) return;
          clearAllPendingUserScores(uid);
          setCloudScores(res.profile.scores);
          setScores((prev) => buildStoreFromScoresAndRuns(res.profile.scores, prev.recentRuns));
          if (res.uploadedCount > 0) {
            setBoardRefreshToken((n) => n + 1);
          }
        })
        .catch(() => {
          // Ignore transient reconnect errors.
        });
    };

    window.addEventListener("online", onOnline);
    return () => window.removeEventListener("online", onOnline);
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
    const allRows = hudBoardRows[scenario] ?? [];
    const myBoardRow = currentUid ? allRows.find((r) => r.uid === currentUid) : undefined;
    const myBest = currentUid
      ? Math.max(
          cloudScores[scenario] ?? 0,
          myBoardRow?.score ?? 0,
          scores.bestByScenario[scenario]?.score ?? 0,
        )
      : (scores.bestByScenario[scenario]?.score ?? 0);
    const rows = allRows.slice(0, 5).map((r) => ({
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
    const onReady = () => {
      setStage("ready");
      focusCanvas();
    };
    const onFullscreenChange = () => setIsFullscreen(Boolean(document.fullscreenElement));
    const canvasEl = canvasRef.current;
    const onWebglContextLost = (e: Event) => {
      e.preventDefault();
      setStage("error");
      setErrorMsg("Graphics context lost — please reload the page.");
    };

    const onScenarioChanged = (e: Event) => {
      const detail = (e as CustomEvent<string>).detail;
      if (detail) {
        setScenario(detail);
      }
    };

    const onRunComplete = (e: Event) => {
      try {
        const payload = JSON.parse((e as CustomEvent<string>).detail) as RunCompletePayload;
        trackAnalyticsEvent("table_flip", {
          room_id: payload.scenarioId,
          room_title: payload.scenarioTitle,
          score: payload.totalScore,
          items_destroyed: payload.itemsDestroyed,
          items_toppled: payload.itemsToppled,
          max_combo: payload.maxCombo,
        });
        const activeUid = authRef.current.uid;
        const activeUsername = authRef.current.username;
        const prevCloudBest = authRef.current.cloudScores[payload.scenarioId] ?? 0;

        setScores((prev) => {
          const prevLocalBest = prev.bestByScenario[payload.scenarioId]?.score ?? 0;
          const prevOverallBest = Math.max(prevLocalBest, prevCloudBest);
          const isNewBest = payload.totalScore > prevOverallBest && payload.totalScore > 0;
          if (isNewBest) {
            const suffix = activeUid && !activeUsername ? " — SET HANDLE TO POST!" : "!";
            showBanner(`NEW ${payload.scenarioTitle} RECORD: ${payload.totalScore}${suffix}`);
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

          if (activeUid) {
            saveUserRecentRuns(activeUid, next.recentRuns);
            if (payload.totalScore > prevCloudBest && payload.totalScore > 0) {
              recordPendingUserScore(activeUid, payload.scenarioId, payload.totalScore);
            }
          } else {
            saveGuestScores(next);
          }
          return next;
        });

        if (activeUid && activeUsername && payload.totalScore > prevCloudBest && payload.totalScore > 0) {
          void submitLevelScore(payload.scenarioId, payload.totalScore, activeUid)
            .then((res) => {
              clearPendingUserScore(activeUid, payload.scenarioId, res.bestScore);
              if (res.updated && authRef.current.uid === activeUid) {
                setCloudScores((prev) => ({
                  ...prev,
                  [payload.scenarioId]: res.bestScore,
                }));
                setBoardRefreshToken((n) => n + 1);
              }
            })
            .catch(() => {
              // Remains in `pending` queue; online reconnect or next session will sync.
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
    canvasEl?.addEventListener("webglcontextlost", onWebglContextLost);

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
      canvasEl?.removeEventListener("webglcontextlost", onWebglContextLost);
    };
  }, [showBanner, focusCanvas]);

  const handleSelectScenario = useCallback(
    (id: string) => {
      setScenario(id);
      wasmRef.current?.bridge_load_scenario(id);
      focusCanvas();
    },
    [focusCanvas],
  );

  const handleReset = useCallback(() => {
    wasmRef.current?.bridge_reset_round();
    focusCanvas();
  }, [focusCanvas]);

  const handleToggleMute = useCallback(() => {
    setMuted((prev) => {
      const next = !prev;
      wasmRef.current?.bridge_set_muted(next);
      return next;
    });
    focusCanvas();
  }, [focusCanvas]);

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
    } finally {
      focusCanvas();
    }
  }, [focusCanvas]);

  const handleSignIn = useCallback(async () => {
    setAuthBusy(true);
    setAuthError(null);
    try {
      await signInWithGoogle();
    } catch (err) {
      const code = (err as { code?: string })?.code ?? "";
      if (code !== "auth/popup-closed-by-user" && code !== "auth/cancelled-popup-request") {
        const msg = formatAuthError(err, "Google sign-in failed.");
        setAuthError(msg);
        showBanner(msg, 4200);
      }
    } finally {
      setAuthBusy(false);
    }
  }, [showBanner]);

  const handleSignOut = useCallback(async () => {
    authSeqRef.current += 1;
    setAuthError(null);
    setUsernameError(null);
    setShowUsernameModal(false);
    try {
      await signOutUser();
      authRef.current = { uid: null, username: null, cloudScores: {} };
      setCurrentUid(null);
      setUsername(null);
      setCloudScores({});
      setScores(loadGuestScores());
    } catch (err) {
      setAuthError(formatAuthError(err, "Failed to sign out."));
    }
  }, []);

  const handleSaveUsername = useCallback(
    async (chosenHandle: string) => {
      const uid = authRef.current.uid;
      if (!uid) {
        setUsernameError("You must sign in with Google first.");
        return;
      }
      setUsernameBusy(true);
      setUsernameError(null);
      try {
        const guestLocal = loadGuestScores();
        const unsyncedScores = mergeScoreMaps(
          loadPendingUserScores(uid),
          extractFlatScores(guestLocal),
        );
        const mergedRuns = mergeRecentRuns(loadUserRecentRuns(uid), guestLocal.recentRuns);
        const hadGuestData =
          Object.keys(guestLocal.bestByScenario).length > 0 || guestLocal.recentRuns.length > 0;
        const updated = await saveCloudUsername(chosenHandle, unsyncedScores);
        if (authRef.current.uid !== uid) {
          return;
        }
        clearAllPendingUserScores(uid);
        if (hadGuestData) {
          clearGuestScores();
        }
        saveUserRecentRuns(uid, mergedRuns);
        authRef.current = {
          uid,
          username: updated.username,
          cloudScores: updated.scores,
        };
        setUsername(updated.username);
        setCloudScores(updated.scores);
        setScores(buildStoreFromScoresAndRuns(updated.scores, mergedRuns));
        setShowUsernameModal(false);
        setBoardRefreshToken((n) => n + 1);
        showBanner(`HANDLE SAVED: ${updated.username}!`);
        focusCanvas();
      } catch (err) {
        setUsernameError(formatAuthError(err, "Failed to save handle."));
      } finally {
        setUsernameBusy(false);
      }
    },
    [showBanner, focusCanvas],
  );

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
        ref={canvasRef}
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
            onClick={(e) => {
              e.currentTarget.blur();
              handleSelectScenario(entry.id);
            }}
            style={pillBtnStyle(scenario === entry.id)}
          >
            {entry.shortLabel}
          </button>
        ))}
        <button
          type="button"
          onClick={(e) => {
            e.currentTarget.blur();
            handleReset();
          }}
          style={pillBtnStyle(false)}
        >
          RETRY
        </button>
        <button
          type="button"
          onClick={(e) => {
            e.currentTarget.blur();
            handleToggleMute();
          }}
          style={pillBtnStyle(muted)}
        >
          {muted ? "MUTED" : "SFX ON"}
        </button>
        {!currentUid ? (
          <button
            type="button"
            onClick={(e) => {
              e.currentTarget.blur();
              void handleSignIn();
            }}
            disabled={authBusy}
            style={pillBtnStyle(false)}
          >
            {authBusy ? "SIGNING IN..." : "GOOGLE LOGIN"}
          </button>
        ) : (
          <>
            <button
              type="button"
              onClick={(e) => {
                e.currentTarget.blur();
                setUsernameError(null);
                setShowUsernameModal(true);
              }}
              disabled={authSyncing}
              style={pillBtnStyle(!username && !authSyncing)}
            >
              {authSyncing ? "SYNCING..." : username ? `ID: ${username}` : "SET HANDLE"}
            </button>
            <button
              type="button"
              onClick={(e) => {
                e.currentTarget.blur();
                void handleSignOut();
              }}
              style={pillBtnStyle(false)}
            >
              LOGOUT
            </button>
          </>
        )}
        <button
          type="button"
          onClick={(e) => {
            e.currentTarget.blur();
            const next = !showScores;
            setShowScores(next);
            if (!next) {
              focusCanvas();
            }
          }}
          style={pillBtnStyle(showScores)}
        >
          SCORES
        </button>
        <button
          type="button"
          onClick={(e) => {
            e.currentTarget.blur();
            void toggleFullscreen();
          }}
          style={pillBtnStyle(isFullscreen)}
        >
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
          authBusy={authBusy || authSyncing}
          authError={authError}
          refreshToken={boardRefreshToken}
          onClose={() => {
            setShowScores(false);
            focusCanvas();
          }}
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
        onClose={() => {
          setShowUsernameModal(false);
          focusCanvas();
        }}
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
