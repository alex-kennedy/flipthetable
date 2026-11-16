import { GameCanvas } from "@/components/GameCanvas";

export default function HomePage() {
  return (
    <main
      style={{
        minHeight: "100vh",
        display: "flex",
        flexDirection: "column",
        alignItems: "center",
        justifyContent: "center",
        padding: "24px",
        gap: "16px",
      }}
    >
      <header
        style={{
          width: "100%",
          maxWidth: "960px",
          display: "flex",
          justifyContent: "space-between",
          alignItems: "baseline",
        }}
      >
        <h1 style={{ margin: 0, fontSize: "20px", letterSpacing: "0.08em", color: "#f2bf40" }}>
          FLIP THE TABLE
        </h1>
        <span style={{ fontSize: "12px", opacity: 0.7 }}>
          BEVY WASM + NEXT.JS STATIC EXPORT
        </span>
      </header>

      <GameCanvas />

      <footer style={{ fontSize: "12px", opacity: 0.6 }}>
        Phase 0: Engine &amp; React Shell Verification
      </footer>
    </main>
  );
}
