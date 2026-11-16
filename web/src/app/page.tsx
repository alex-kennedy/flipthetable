import { GameCanvas } from "@/components/GameCanvas";

export default function HomePage() {
  return (
    <main
      style={{
        position: "fixed",
        inset: 0,
        width: "100vw",
        height: "100dvh",
        overflow: "hidden",
        background: "#0d0f14",
      }}
    >
      <GameCanvas />
    </main>
  );
}
