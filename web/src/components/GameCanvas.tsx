"use client";

import { useCallback, useEffect, useRef, useState } from "react";

type WasmModule = {
  default: (moduleOrPath?: string | URL | Request) => Promise<unknown>;
  start_game: (canvasSelector: string) => void;
};

type LoadStage = "fetching" | "booting" | "ready" | "error";

export function GameCanvas() {
  const containerRef = useRef<HTMLDivElement>(null);
  const startedRef = useRef(false);
  const [stage, setStage] = useState<LoadStage>("fetching");
  const [errorMsg, setErrorMsg] = useState<string | null>(null);
  const [isFullscreen, setIsFullscreen] = useState(false);

  useEffect(() => {
    const onReady = () => setStage("ready");
    const onFullscreenChange = () => setIsFullscreen(Boolean(document.fullscreenElement));

    window.addEventListener("flipthetable:ready", onReady);
    document.addEventListener("fullscreenchange", onFullscreenChange);

    if (!startedRef.current) {
      startedRef.current = true;

      (async () => {
        try {
          const wasmUrl = "/wasm/flipthetable_game.js";
          const mod = (await import(/* webpackIgnore: true */ wasmUrl)) as WasmModule;
          await mod.default("/wasm/flipthetable_game_bg.wasm");
          setStage("booting");
          mod.start_game("#bevy-canvas");
        } catch (err) {
          // Bevy's winit loop throws a control-flow exception in WASM when starting;
          // ignore that expected exception while surfacing real load errors.
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
      document.removeEventListener("fullscreenchange", onFullscreenChange);
    };
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
      // Ignore fullscreen rejections (e.g., unsupported in embedded iframe).
    }
  }, []);

  const isOverlayVisible = stage !== "ready";

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

      {/* Minimal floating top-right shell controls */}
      <div
        style={{
          position: "absolute",
          top: "14px",
          right: "14px",
          zIndex: 20,
          display: "flex",
          gap: "8px",
        }}
      >
        <button
          type="button"
          onClick={toggleFullscreen}
          title={isFullscreen ? "Exit Fullscreen" : "Enter Fullscreen"}
          style={{
            background: "rgba(13, 15, 20, 0.8)",
            color: "#f2bf40",
            border: "2px solid #f2bf40",
            padding: "6px 10px",
            fontSize: "11px",
            fontFamily: "inherit",
            letterSpacing: "0.08em",
            cursor: "pointer",
            textTransform: "uppercase",
          }}
        >
          {isFullscreen ? "EXIT FULL" : "FULLSCREEN"}
        </button>
      </div>

      {/* Retro pixel-art loading overlay */}
      <div
        aria-hidden={!isOverlayVisible}
        style={{
          position: "absolute",
          inset: 0,
          zIndex: 10,
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
        {/* Pixel-art backdrop */}
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

        {/* Vignette + CRT scanlines */}
        <div
          className="scanlines"
          style={{
            position: "absolute",
            inset: 0,
            backgroundColor: "rgba(13, 15, 20, 0.42)",
            boxShadow: "inset 0 0 120px rgba(0, 0, 0, 0.85)",
          }}
        />

        {/* Foreground loading card */}
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
          <div
            style={{
              display: "flex",
              alignItems: "center",
              gap: "14px",
            }}
          >
            {/* Cute animated pixel-art steaming coffee mug SVG */}
            <svg
              width="36"
              height="36"
              viewBox="0 0 18 18"
              shapeRendering="crispEdges"
              aria-hidden="true"
              style={{ flexShrink: 0 }}
            >
              {/* Rising pixel steam */}
              <g fill="#f5efe0">
                <rect className="steam-1" x="6" y="4" width="1" height="2" />
                <rect className="steam-2" x="8" y="3" width="1" height="2" />
                <rect className="steam-3" x="10" y="4" width="1" height="2" />
              </g>
              {/* Mug body & handle */}
              <rect x="4" y="7" width="8" height="8" fill="#f5efe0" />
              <rect x="5" y="7" width="6" height="1" fill="#6b3e26" />
              <rect x="12" y="9" width="2" height="1" fill="#d9cfc0" />
              <rect x="13" y="10" width="1" height="2" fill="#d9cfc0" />
              <rect x="12" y="12" width="2" height="1" fill="#d9cfc0" />
              {/* Cute face on mug */}
              <rect x="6" y="10" width="1" height="1" fill="#1a1f2b" />
              <rect x="9" y="10" width="1" height="1" fill="#1a1f2b" />
              <rect x="7" y="12" width="2" height="1" fill="#f2bf40" />
              {/* Subtle base shadow */}
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
                {stage === "fetching"
                  ? "BREWING WASM ENGINE..."
                  : "SETTING UP THE DESK..."}
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
