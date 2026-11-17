export type RunSummaryRecord = {
  scenarioId: string;
  scenarioTitle: string;
  totalScore: number;
  itemsDestroyed: number;
  itemsToppled: number;
  maxCombo: number;
  timestamp: number;
};

export type LocalScoreStore = {
  bestByScenario: Record<string, { score: number; combo: number }>;
  recentRuns: RunSummaryRecord[];
};

/**
 * Storage model (v2):
 *   - `flipthetable_guest_scores_v2`: Scores & recent runs achieved while logged OUT.
 *     Uploaded once on sign-in, then cleared so accounts never contaminate each other.
 *   - `flipthetable_pending_sync_v2:<uid>`: ONLY genuinely unsynced personal-best scores
 *     achieved while signed in (e.g. offline or before picking a handle). Cleared per-level
 *     as soon as Firestore confirms the write — never populated from cloud reads, so manual
 *     Firestore edits/deletions are never overwritten by stale browser cache!
 *   - `flipthetable_recent_runs_v2:<uid>`: Recent flips feed for `<uid>`.
 */
const GUEST_STORAGE_KEY = "flipthetable_guest_scores_v2";
const PENDING_SYNC_PREFIX = "flipthetable_pending_sync_v2:";
const USER_RECENT_PREFIX = "flipthetable_recent_runs_v2:";

function emptyStore(): LocalScoreStore {
  return { bestByScenario: {}, recentRuns: [] };
}

function parseStore(raw: string | null): LocalScoreStore {
  if (!raw) {
    return emptyStore();
  }
  try {
    const parsed = JSON.parse(raw) as Partial<LocalScoreStore>;
    const bestByScenario: Record<string, { score: number; combo: number }> = {};
    if (parsed.bestByScenario && typeof parsed.bestByScenario === "object") {
      for (const [levelId, val] of Object.entries(parsed.bestByScenario)) {
        if (val && typeof val === "object" && typeof val.score === "number" && val.score > 0) {
          bestByScenario[levelId] = {
            score: Math.floor(val.score),
            combo: typeof val.combo === "number" && val.combo >= 1 ? Math.floor(val.combo) : 1,
          };
        }
      }
    }
    const recentRuns = Array.isArray(parsed.recentRuns) ? parsed.recentRuns.slice(0, 8) : [];
    return { bestByScenario, recentRuns };
  } catch {
    return emptyStore();
  }
}

function writeJson(key: string, value: unknown): void {
  try {
    localStorage.setItem(key, JSON.stringify(value));
  } catch {
    // Ignore quota or private-browsing storage errors.
  }
}

export function loadGuestScores(): LocalScoreStore {
  try {
    return parseStore(localStorage.getItem(GUEST_STORAGE_KEY));
  } catch {
    return emptyStore();
  }
}

export function saveGuestScores(store: LocalScoreStore): void {
  writeJson(GUEST_STORAGE_KEY, store);
}

export function clearGuestScores(): void {
  try {
    localStorage.removeItem(GUEST_STORAGE_KEY);
  } catch {
    // Ignore storage removal errors.
  }
}

export function loadPendingUserScores(uid: string): Record<string, number> {
  try {
    const raw = localStorage.getItem(`${PENDING_SYNC_PREFIX}${uid}`);
    if (!raw) return {};
    const parsed = JSON.parse(raw) as Record<string, unknown>;
    const out: Record<string, number> = {};
    for (const [levelId, val] of Object.entries(parsed)) {
      if (typeof val === "number" && Number.isFinite(val) && val > 0) {
        out[levelId] = Math.floor(val);
      }
    }
    return out;
  } catch {
    return {};
  }
}

export function recordPendingUserScore(uid: string, levelId: string, score: number): void {
  const prev = loadPendingUserScores(uid);
  const clean = Math.floor(score);
  if (clean > (prev[levelId] ?? 0)) {
    prev[levelId] = clean;
    writeJson(`${PENDING_SYNC_PREFIX}${uid}`, prev);
  }
}

export function clearPendingUserScore(uid: string, levelId: string, upToScore?: number): void {
  const prev = loadPendingUserScores(uid);
  if (!(levelId in prev)) return;
  if (upToScore === undefined || prev[levelId] <= upToScore) {
    delete prev[levelId];
    try {
      if (Object.keys(prev).length === 0) {
        localStorage.removeItem(`${PENDING_SYNC_PREFIX}${uid}`);
      } else {
        writeJson(`${PENDING_SYNC_PREFIX}${uid}`, prev);
      }
    } catch {
      // Ignore storage errors.
    }
  }
}

export function clearAllPendingUserScores(uid: string): void {
  try {
    localStorage.removeItem(`${PENDING_SYNC_PREFIX}${uid}`);
  } catch {
    // Ignore storage errors.
  }
}

export function loadUserRecentRuns(uid: string): RunSummaryRecord[] {
  try {
    const raw = localStorage.getItem(`${USER_RECENT_PREFIX}${uid}`);
    if (!raw) return [];
    const parsed = JSON.parse(raw);
    return Array.isArray(parsed) ? (parsed as RunSummaryRecord[]).slice(0, 8) : [];
  } catch {
    return [];
  }
}

export function saveUserRecentRuns(uid: string, runs: RunSummaryRecord[]): void {
  writeJson(`${USER_RECENT_PREFIX}${uid}`, runs.slice(0, 8));
}

export function extractFlatScores(store: LocalScoreStore): Record<string, number> {
  const flat: Record<string, number> = {};
  for (const [levelId, entry] of Object.entries(store.bestByScenario)) {
    if (entry.score > 0) {
      flat[levelId] = entry.score;
    }
  }
  return flat;
}

export function mergeScoreMaps(
  a: Record<string, number>,
  b: Record<string, number>,
): Record<string, number> {
  const out: Record<string, number> = { ...a };
  for (const [levelId, score] of Object.entries(b)) {
    if (score > (out[levelId] ?? 0)) {
      out[levelId] = score;
    }
  }
  return out;
}

export function mergeRecentRuns(
  a: RunSummaryRecord[],
  b: RunSummaryRecord[],
): RunSummaryRecord[] {
  const seen = new Set<string>();
  const out: RunSummaryRecord[] = [];
  for (const run of [...a, ...b].sort((x, y) => y.timestamp - x.timestamp)) {
    const key = `${run.timestamp}:${run.scenarioId}:${run.totalScore}`;
    if (!seen.has(key)) {
      seen.add(key);
      out.push(run);
      if (out.length >= 8) break;
    }
  }
  return out;
}

export function buildStoreFromScoresAndRuns(
  scoresMap: Record<string, number>,
  recentRuns: RunSummaryRecord[],
): LocalScoreStore {
  const bestByScenario: Record<string, { score: number; combo: number }> = {};
  for (const [levelId, score] of Object.entries(scoresMap)) {
    if (score > 0) {
      bestByScenario[levelId] = { score, combo: 1 };
    }
  }
  return { bestByScenario, recentRuns: recentRuns.slice(0, 8) };
}
