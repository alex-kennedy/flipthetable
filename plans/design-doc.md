# Flip the Table — Design & Implementation Plan

## 1. High-Level Concept

**Flip the Table** is a retro-styled, physics-driven arcade destruction game.  
You sit at a desk in a series of themed office scenarios (a cramped corporate cubicle, a chaotic building site trailer, an executive boardroom, etc.). Using a single upward mouse/touch thrust, you flip the table—launching it and everything on it through the room. The more chaos and destruction you cause, the higher your score.

### Core Loop
1. **Load Scenario**: You're seated in first-person PoV at a desk cluttered with scenario-specific props, facing a room full of breakable/knockable objects.
2. **The Flip**: Perform a single upward + directional swipe/thrust. We sample pointer velocity, acceleration curve, and direction to translate your physical gesture into an authentic, weighty table flip.
3. **Mayhem**: The table launches into the room, sending desk items flying, triggering chain-reaction collisions, shattering props, and racking up combo multipliers.
4. **Tally & Retry**: Once physics settle (or a post-flip timer expires), your destruction breakdown and total score are shown. Instant retry or submit to the leaderboard.

---

## 2. Visual & Audio Direction ("Retro / Pixely / Super Satisfying")

### Visuals
- **Low-Poly 3D + Pixelation Pipeline**:
  - Clean low-poly 3D meshes (procedurally composed in Bevy / `.glb` later if desired) with crisp retro color palettes.
  - Render the 3D Bevy scene to an offscreen low-resolution texture (e.g., `320x180` or `480x270`) and upscale to the canvas with nearest-neighbor filtering.
  - Optional lightweight post-processing pass: subtle color quantization / ordered dithering for a crisp PS1 / *A Short Hike* aesthetic.
- **Particles & Debris**:
  - High-impact 2D/3D particle bursts: flying papers (tumbling quads), ceramic shards, sparks from electronics, water splashes, and dust clouds.

### "Juice" & Game Feel
- **Subtle Follow Camera**: Starts seated at the desk in 1st-person PoV; on flip, smoothly eases/tracks toward the table's trajectory and primary impact zone (with subtle FOV punch and trauma-based camera shake) so you always have the best view of the carnage.
- **Hit-Stop & Micro Slow-Mo**: Brief 25–45ms freeze frames on heavy impacts (e.g., smashing a monitor or glass partition), plus optional subtle time-dilation when a tall prop (like a filing cabinet or water cooler) tips and slowly crashes down.
- **Animated Point Popups**: Crisp pixel-art floating score numbers (`+250`, `+500 MONITOR!`, `x3 CHAIN!`) that pop, arc, scale with combo multiplier, and tally into the HUD counter as items break, tip over, or topple.
- **Audio**: Crunchy, punchy retro SFX (table wood thud, ceramic shatter, keyboard clatter, glass crash, paper whoosh) with pitch randomization so nothing sounds repetitive.

---

## 3. Core Mechanics & Physics

### Physics Engine
- **`avian3d`** (recommended over `rapier` for native Bevy ECS integration, clean WASM compilation, and straightforward collision/impulse events).

### The Flip Gesture (Click-and-Flip)
- **Input Sampling**:
  - **Pointer Down**: Player clicks/touches to grab the front edge of the desk (optional subtle visual/audio "brace" cue).
  - **Drag / Thrust**: Record a rolling ring buffer of `(x, y, timestamp)` samples over the last ~100ms while dragging upward/directionally.
  - **Release (or Peak Thrust)**: Compute **Peak Acceleration & Release Velocity** $\rightarrow$ flip force magnitude, and **Direction Vector** $(\Delta x, \Delta y)$ $\rightarrow$ lateral aim (left/center/right) + loft angle.
- **Physics Application**:
  - Apply an upward + forward linear impulse *and* an angular impulse (pitching the table up and over) offset toward the user-facing edge of the desk so it genuinely *flips* end-over-end rather than sliding like an air-hockey puck.
  - Tune table mass, friction, and center-of-mass so weak flicks barely tip the desk (funny/pathetic), while a fast, committed thrust sends it airborne across the room.

### Multi-Stage Destructible Props & Scoring
- Each prop has `Mass`, `Restitution`, `PointValue`, and staged damage thresholds:
  - **Stage 1 (Tipped / Damaged)**:
    - Tall/heavy props (filing cabinets, cubicle walls, water coolers) emit a `TOPPLED!` point popup + subtle slow-mo accent as soon as they tilt past a critical angle and begin falling.
    - Electronics (monitors, coffee pots) spark/smoke/crack on a medium hit before breaking.
  - **Stage 2 (Shattered / Crashed)**:
    - On high-impulse impact, swap the mesh for 2–4 smaller physics debris chunks + spawns particle burst + emits `ScoreEvent`.
- **Chain Reactions & Bonus Targets**:
  - Moving props that knock over or shatter *other* props escalate the combo multiplier (`Chain x2`, `Chain x3`, etc.).
  - Room-specific bonus callouts (e.g., *"BOSS'S MUG!"*, *"TPS REPORTS SCATTERED!"*, *"OSHA VIOLATION!"*).
- **Run End Detection**: Wait until all dynamic bodies drop below a kinetic energy threshold for ~1.0s (with a hard cap like 8–10 seconds post-flip so a spinning paperclip doesn't stall the round).

---

## 4. Technical Architecture

```
┌───────────────────────────────────────────────────────────┐
│         React + Next.js (Static Export -> CF Pages)       │
│  • Landing / Auth / Profile / Global & Scenario Boards    │
│  • Scenario Picker & Settings                             │
│  • Hosts <canvas> + JS <-> WASM Bridge                    │
└───────────────────────────┬───────────────────────────────┘
                            │ wasm-bindgen / CustomEvents
┌───────────────────────────▼───────────────────────────────┐
│                Bevy Engine (Rust -> WASM)                 │
│  • 3D Scene, Low-Res Pixel Render Target, Follow Camera   │
│  • Avian3D Physics, Click-and-Flip, Destruction System    │
│  • Rust UI (bevy_ui): In-Game HUD, Animated Score Popups, │
│    Combo Meter, Post-Flip Breakdown & Quick-Retry         │
└───────────────────────────┬───────────────────────────────┘
                            │ (Phase 5: Score & Auth Token)
┌───────────────────────────▼───────────────────────────────┐
│                   Firebase Backend                        │
│  • Firebase Auth (Google / Email / Anonymous upgrade)     │
│  • Firestore (Leaderboards per scenario, User Profiles)   │
│  • Cloud Functions (Optional score-breakdown validation)  │
└───────────────────────────────────────────────────────────┘
```

### Responsibility Split
- **Bevy + Rust UI (`crates/game`)**:
  - Owns the entire gameplay loop, rendering, audio, and *in-game* HUD (score counter, combo meter, animated world-to-screen point popups, pause/retry overlay) using `bevy_ui` with a crisp pixel font.
  - Exposes a minimal, clean WASM API:
    - Commands from React $\rightarrow$ Bevy: `load_scenario(id)`, `reset_round()`, `set_muted(bool)`, `pause(bool)`.
    - Events from Bevy $\rightarrow$ React: `on_ready()`, `on_run_complete({ scenarioId, totalScore, itemsDestroyed, maxCombo, breakdown })`.
- **React + Next.js (`web/`)**:
  - Configured with `output: 'export'` for **Cloudflare Pages** static hosting (no Node server runtime required; WASM + assets served from `public/wasm/`).
  - Handles everything *outside* the graphics canvas: routing, Firebase authentication UI, leaderboards, player stats, and responsive arcade framing around the game canvas.
- **Firebase**:
  - Added in a dedicated phase once the core game is fun and playable, so we don't block gameplay iteration on auth/backend plumbing.

---

## 5. Planned Scenarios (Initial Set)

1. **Corporate Cubicle**
   - *On the desk*: CRT/LCD monitor, mechanical keyboard, mouse, ceramic coffee mug, stapler, stacks of "TPS Reports", desk plant.
   - *In the room*: Swivel chair, cubicle partitions (knockable!), filing cabinet, water cooler, coworker's desk + monitor across the aisle, fluorescent ceiling lights.
2. **Building Site Office (Portable Trailer)**
   - *On the desk*: Rolled blueprints, hard hat, dented metal thermos, walkie-talkie, clipboard, coffee pot.
   - *In the room*: Folding chairs, whiteboard on wheels, shelving with paint cans / toolboxes, mini-fridge, window overlooking scaffolding.
3. **Executive Boardroom** *(stretch/unlockable)*
   - *On the desk*: Heavy mahogany table (requires a big thrust!), laptops, crystal water glasses, speakerphone.
   - *In the room*: Floor-to-ceiling glass wall, wall-mounted TV, leather chairs, trophy shelf.

---

## 6. Phased Implementation Plan

### Phase 0: Workspace & Toolchain Setup
- Initialize Cargo workspace (`crates/game`) targeting `wasm32-unknown-unknown` via `wasm-pack`.
- Initialize Next.js (`output: 'export'`) + TypeScript app (`web/`) using `npm`, configured for Cloudflare Pages static deployment and loading the `wasm-pack` output from `public/wasm/`.
- Verify a minimal Bevy canvas renders inside the Next.js React shell.

### Phase 1: "The Flip" Vertical Slice
- Greybox 1 office room + 1 desk + ~10 physics props.
- Implement mouse/touch gesture tracking (acceleration + direction) $\rightarrow$ table flip impulse.
- Basic collision-based destruction, score counting, and a minimal `bevy_ui` HUD + instant retry (`R` key or button).
- *Goal*: Make the flip itself feel hilarious and satisfying before adding any polish.

### Phase 2: Retro Pixel Style, Juice & Audio
- Add low-res offscreen render target + nearest-neighbor upscaling.
- Add hit-stop, camera trauma shake, flying paper/shard particles, and floating score popups.
- Add retro SFX with pitch variation.

### Phase 3: Scenarios & Data-Driven Rooms
- Define scenario layouts in RON/JSON so rooms, desk props, and point values are easy to author and tune.
- Build out the **Corporate Cubicle** and **Building Site Office** scenarios with distinct low-poly/pixel-textured props.

### Phase 4: React Shell & WASM Bridge
- Build the React UI (scenario selector, mute/settings, local high scores, clean arcade framing around the canvas).
- Wire up bidirectional JS $\leftrightarrow$ WASM communication.

### Phase 5: Firebase Auth & Leaderboards
- Integrate Firebase Auth (Google Sign-In + guest/anonymous option).
- Store and query per-scenario & overall leaderboards in Firestore.
- Add basic score-breakdown sanity checks.

---

## 7. Locked-In Decisions & Next Step
- **Stack**: Bevy + `avian3d` + `bevy_ui` (Rust $\rightarrow$ WASM, web-only target) inside Next.js (`output: 'export'`, `npm`) for Cloudflare Pages, with Firebase Auth + Firestore added in Phase 5.
- **Look & Feel**: Low-poly 3D through a low-res pixelated render target, click-and-flip gesture, subtle follow camera, micro slow-mo on big topples, multi-stage destruction (tipping gives fun minor popups, shattering/destroying yields the big points), and animated floating pixel point popups.
- **Next Step**: Begin **Phase 0** (Cargo workspace + Next.js static export + `wasm-pack` bridge verification).
