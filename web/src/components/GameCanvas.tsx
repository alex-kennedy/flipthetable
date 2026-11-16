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
            width: "min(520px, 100%)",
            background: "rgba(13, 15, 20, 0.9)",
            border: "3px solid #f2bf40",
            boxShadow: "0 8px 0 #07080b",
            padding: "20px 24px",
            display: "flex",
            flexDirection: "column",
            gap: "12px",
          }}
        >
          <div
            style={{
              display: "flex",
              justifyContent: "space-between",
              alignItems: "baseline",
              gap: "12px",
            }}
          >
            <h1
              style={{
                margin: 0,
                fontSize: "clamp(18px, 3vw, 24px)",
                letterSpacing: "0.12em",
                color: "#f2bf40",
              }}
            >
              FLIP THE TABLE
            </h1>
            <span style={{ fontSize: "11px", color: "#8be9fd", letterSpacing: "0.08em" }}>
              WASM // BEVY
            </span>
          </div>

          {stage === "error" ? (
            <div style={{ color: "#ff6b6b", fontSize: "12px", lineHeight: 1.5 }}>
              <strong>FAILED TO LOAD ENGINE</strong>
              <div style={{ marginTop: "4px", opacity: 0.9 }}>{errorMsg}</div>
            </div>
          ) : (
            <>
              <div
                style={{
                  width: "100%",
                  height: "12px",
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

              <div
                style={{
                  display: "flex",
                  justifyContent: "space-between",
                  fontSize: "11px",
                  letterSpacing: "0.08em",
                  color: "#f5efe0",
                  opacity: 0.85,
                }}
              >
                <span>
                  {stage === "fetching"
                    ? "DOWNLOADING WASM ENGINE..."
                    : "BOOTING PHYSICS & SCENE..."}
                </span>
                <span>HOLD &amp; FLICK UP TO FLIP</span>
              </div>
            </>
          )}
        </div>
      </div>
    </div>
  );
}
