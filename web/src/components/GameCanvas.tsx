"use client";

import { useEffect, useRef, useState } from "react";

type WasmModule = {
  default: (moduleOrPath?: string | URL | Request) => Promise<unknown>;
  start_game: (canvasSelector: string) => void;
};

export function GameCanvas() {
  const startedRef = useRef(false);
  const [status, setStatus] = useState<"loading" | "ready" | "error">("loading");
  const [errorMsg, setErrorMsg] = useState<string | null>(null);

  useEffect(() => {
    const onReady = () => setStatus("ready");
    window.addEventListener("flipthetable:ready", onReady);

    if (!startedRef.current) {
      startedRef.current = true;

      (async () => {
        try {
          const wasmUrl = "/wasm/flipthetable_game.js";
          const mod = (await import(/* webpackIgnore: true */ wasmUrl)) as WasmModule;
          await mod.default("/wasm/flipthetable_game_bg.wasm");
          mod.start_game("#bevy-canvas");
        } catch (err) {
          // Bevy's winit loop throws a control-flow exception in WASM when starting;
          // ignore that expected exception while surfacing real load errors.
          const msg = err instanceof Error ? err.message : String(err);
          if (!msg.includes("Using exceptions for control flow")) {
            setStatus("error");
            setErrorMsg(msg);
          }
        }
      })();
    }

    return () => {
      window.removeEventListener("flipthetable:ready", onReady);
    };
  }, []);

  return (
    <div
      style={{
        position: "relative",
        width: "100%",
        maxWidth: "960px",
        aspectRatio: "16 / 9",
        border: "2px solid #f2bf40",
        background: "#141720",
        boxShadow: "0 0 32px rgba(0, 0, 0, 0.65)",
      }}
    >
      <canvas
        id="bevy-canvas"
        style={{
          width: "100%",
          height: "100%",
          display: "block",
          imageRendering: "pixelated",
        }}
      />

      {status === "loading" && (
        <div
          style={{
            position: "absolute",
            inset: 0,
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            pointerEvents: "none",
            fontSize: "14px",
            letterSpacing: "0.08em",
            color: "#f2bf40",
          }}
        >
          LOADING WASM ENGINE...
        </div>
      )}

      {status === "error" && (
        <div
          style={{
            position: "absolute",
            inset: 0,
            display: "flex",
            flexDirection: "column",
            alignItems: "center",
            justifyContent: "center",
            padding: "24px",
            background: "rgba(13, 15, 20, 0.92)",
            color: "#ff6b6b",
            fontSize: "13px",
            textAlign: "center",
          }}
        >
          <strong>Failed to initialize Bevy WASM</strong>
          <p style={{ marginTop: "8px", opacity: 0.85 }}>{errorMsg}</p>
        </div>
      )}
    </div>
  );
}
