"use client";

import { useCallback, useEffect, useRef, useState } from "react";

type ScenarioId = "corporate" | "building_site";

type RunCompletePayload = {
  scenarioId: ScenarioId;
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
  default: (moduleOrPath?: string | URL | Request) => Promise<unknown>;
  start_game: (canvasSelector: string) => void;
  bridge_load_scenario: (id: string) => void;
  bridge_reset_round: () => void;
  bridge_set_muted: (muted: boolean) => void;
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

export function GameCanvas() {
  const containerRef = useRef<HTMLDivElement>(null);
  const wasmRef = useRef<WasmModule | null>(null);
  const startedRef = useRef(false);

  const [stage, setStage] = useState<LoadStage>("fetching");
  const [errorMsg, setErrorMsg] = useState<string | null>(null);
  const [isFullscreen, setIsFullscreen] = useState(false);
  const [scenario, setScenario] = useState<ScenarioId>("corporate");
  const [muted, setMuted] = useState(false);
  const [showScores, setShowScores] = useState(false);
  const [scores, setScores] = useState<LocalScoreStore>({ bestByScenario: {}, recentRuns: [] });
  const [newRecordBanner, setNewRecordBanner] = useState<string | null>(null);

  useEffect(() => {
    setScores(loadLocalScores());
  }, []);

  useEffect(() => {
    const onReady = () => setStage("ready");
    const onFullscreenChange = () => setIsFullscreen(Boolean(document.fullscreenElement));

    const onScenarioChanged = (e: Event) => {
      const detail = (e as CustomEvent<string>).detail;
      if (detail === "corporate" || detail === "building_site") {
        setScenario(detail);
      }
    };

    const onRunComplete = (e: Event) => {
      try {
        const payload = JSON.parse((e as CustomEvent<string>).detail) as RunCompletePayload;
        setScores((prev) => {
          const prevBest = prev.bestByScenario[payload.scenarioId]?.score ?? 0;
          const isNewBest = payload.totalScore > prevBest && payload.totalScore > 0;
          if (isNewBest) {
            setNewRecordBanner(`NEW ${payload.scenarioTitle} RECORD: ${payload.totalScore}!`);
            setTimeout(() => setNewRecordBanner(null), 3200);
          }

          const next: LocalScoreStore = {
            bestByScenario: {
              ...prev.bestByScenario,
              [payload.scenarioId]: {
                score: Math.max(prevBest, payload.totalScore),
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
          await mod.default("/wasm/flipthetable_game_bg.wasm");
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

  const handleSelectScenario = useCallback((id: ScenarioId) => {
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

  const isOverlayVisible = stage !== "ready";

  const pillBtnStyle = (active = false): React.CSSProperties => ({
    background: active ? "#f2bf40" : "rgba(13, 15, 20, 0.84)",
    color: active ? "#0d0f14" : "#f2bf40",
    border: "2px solid #f2bf40",
    padding: "5px 9px",
    fontSize: "11px",
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
          opacity: 0.32,
          boxShadow: "inset 0 0 90px rgba(0, 0, 0, 0.55)",
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
            fontSize: "12px",
            fontWeight: 700,
            letterSpacing: "0.08em",
            boxShadow: "0 4px 0 #07080b",
            pointerEvents: "none",
          }}
        >
          {newRecordBanner}
        </div>
      )}

      {/* Minimal floating top-right shell controls */}
      <div
        style={{
          position: "absolute",
          top: "14px",
          right: "14px",
          zIndex: 20,
          display: "flex",
          flexWrap: "wrap",
          justifyContent: "flex-end",
          gap: "6px",
        }}
      >
        <button
          type="button"
          onClick={() => handleSelectScenario("corporate")}
          style={pillBtnStyle(scenario === "corporate")}
        >
          1: OFFICE
        </button>
        <button
          type="button"
          onClick={() => handleSelectScenario("building_site")}
          style={pillBtnStyle(scenario === "building_site")}
        >
          2: SITE
        </button>
        <button type="button" onClick={handleReset} style={pillBtnStyle(false)}>
          RETRY
        </button>
        <button type="button" onClick={handleToggleMute} style={pillBtnStyle(muted)}>
          {muted ? "MUTED" : "SFX ON"}
        </button>
        <button type="button" onClick={() => setShowScores((s) => !s)} style={pillBtnStyle(showScores)}>
          SCORES
        </button>
        <button type="button" onClick={toggleFullscreen} style={pillBtnStyle(isFullscreen)}>
          {isFullscreen ? "EXIT FULL" : "FULLSCREEN"}
        </button>
      </div>

      {/* Local High Scores Drawer */}
      {showScores && (
        <div
          style={{
            position: "absolute",
            top: "52px",
            right: "14px",
            zIndex: 25,
            width: "min(320px, calc(100vw - 28px))",
            background: "rgba(13, 15, 20, 0.94)",
            border: "2px solid #f2bf40",
            boxShadow: "0 6px 0 #07080b",
            padding: "14px 16px",
            display: "flex",
            flexDirection: "column",
            gap: "10px",
            fontSize: "12px",
          }}
        >
          <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
            <strong style={{ color: "#f2bf40", letterSpacing: "0.08em" }}>LOCAL HIGH SCORES</strong>
            <button
              type="button"
              onClick={() => setShowScores(false)}
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

          <div style={{ display: "flex", flexDirection: "column", gap: "6px" }}>
            <div style={{ display: "flex", justifyContent: "space-between" }}>
              <span>CORPORATE CUBICLE</span>
              <span style={{ color: "#f2bf40" }}>
                {scores.bestByScenario.corporate?.score ?? 0} pts
              </span>
            </div>
            <div style={{ display: "flex", justifyContent: "space-between" }}>
              <span>BUILDING SITE TRAILER</span>
              <span style={{ color: "#f2bf40" }}>
                {scores.bestByScenario.building_site?.score ?? 0} pts
              </span>
            </div>
          </div>

          {scores.recentRuns.length > 0 && (
            <div
              style={{
                borderTop: "1px solid #2b3345",
                paddingTop: "8px",
                display: "flex",
                flexDirection: "column",
                gap: "4px",
              }}
            >
              <span style={{ fontSize: "10px", opacity: 0.65, letterSpacing: "0.08em" }}>
                RECENT FLIPS
              </span>
              {scores.recentRuns.slice(0, 4).map((run, idx) => (
                <div
                  key={`${run.timestamp}-${idx}`}
                  style={{
                    display: "flex",
                    justifyContent: "space-between",
                    fontSize: "11px",
                    opacity: 0.85,
                  }}
                >
                  <span>
                    {run.scenarioId === "building_site" ? "SITE" : "OFFICE"} ({run.itemsDestroyed}x)
                  </span>
                  <span style={{ color: "#8be9fd" }}>{run.totalScore}</span>
                </div>
              ))}
            </div>
          )}
        </div>
      )}

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
                  fontSize: "clamp(16px, 2.6vw, 20px)",
                  letterSpacing: "0.1em",
                  color: "#f2bf40",
                }}
              >
                FLIP THE TABLE
              </h1>
              <div
                style={{
                  marginTop: "2px",
                  fontSize: "11px",
                  letterSpacing: "0.08em",
                  color: "#f5efe0",
                  opacity: 0.78,
                }}
              >
                {stage === "fetching" ? "BREWING WASM ENGINE..." : "SETTING UP THE DESK..."}
              </div>
            </div>
          </div>

          {stage === "error" ? (
            <div style={{ color: "#ff6b6b", fontSize: "12px", lineHeight: 1.5 }}>
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
