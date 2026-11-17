import {
  GoogleAuthProvider,
  onAuthStateChanged,
  signInWithPopup,
  signOut,
  type User,
} from "firebase/auth";
import {
  collection,
  collectionGroup,
  doc,
  getDoc,
  getDocs,
  limit,
  orderBy,
  query,
  serverTimestamp,
  setDoc,
  Timestamp,
  where,
  writeBatch,
} from "firebase/firestore";

import { getFirebaseServices } from "./firebase";

export const MAX_USERNAME_LENGTH = 24;
export const MAX_SCORE_VALUE = 100_000_000;
export const DEFAULT_LEADERBOARD_LIMIT = 10;

const KNOWN_LEVEL_IDS = [
  "corporate",
  "building_site",
  "boardroom",
  "all_hands",
  "breakroom",
  "warehouse",
];

export type UserScoreDoc = {
  username: string;
  scores: Record<string, number>;
  updatedAt: Date | null;
};

export type SyncLocalScoresResult = {
  profile: UserScoreDoc;
  uploadedCount: number;
};

export type LeaderboardEntry = {
  uid: string;
  username: string;
  score: number;
  updatedAt: Date | null;
};

function toDateOrNull(value: unknown): Date | null {
  if (value instanceof Timestamp) {
    return value.toDate();
  }
  return null;
}

export function formatAuthError(err: unknown, fallback: string): string {
  const code = (err as { code?: string })?.code ?? "";
  switch (code) {
    case "auth/popup-blocked":
      return "Sign-in popup was blocked by your browser. Allow popups for this site and try again.";
    case "auth/unauthorized-domain":
      return "This domain is not yet listed under Firebase Auth Authorized Domains.";
    case "auth/network-request-failed":
      return "Network error contacting Google Sign-In. Check your connection and retry.";
    default:
      return err instanceof Error && err.message ? err.message : fallback;
  }
}

export function sanitizeUsername(raw: string): string {
  const cleaned = raw.trim().slice(0, MAX_USERNAME_LENGTH).trim();
  if (!cleaned) {
    throw new Error("Pick any non-empty handle (up to 24 characters).");
  }
  return cleaned;
}

export function normalizeScore(raw: number): number {
  if (!Number.isFinite(raw)) {
    return 0;
  }
  return Math.min(MAX_SCORE_VALUE, Math.max(0, Math.floor(raw)));
}

function isValidLevelId(levelId: string): boolean {
  return typeof levelId === "string" && levelId.length >= 1 && levelId.length <= 64;
}

function sanitizeScoresMap(raw: Record<string, unknown>): Record<string, number> {
  const out: Record<string, number> = {};
  for (const [levelId, val] of Object.entries(raw)) {
    if (!isValidLevelId(levelId)) continue;
    const score = normalizeScore(typeof val === "number" ? val : Number(val));
    if (score > 0) {
      out[levelId] = score;
    }
    if (Object.keys(out).length >= 50) {
      break;
    }
  }
  return out;
}

/**
 * Subscribe to Firebase Authentication state changes on the client.
 */
export function subscribeAuthState(onChange: (user: User | null) => void): () => void {
  const services = getFirebaseServices();
  if (!services) {
    return () => {};
  }
  return onAuthStateChanged(services.auth, onChange);
}

/**
 * Sign in exclusively with Google via popup (works cross-origin on Cloudflare Pages).
 */
export async function signInWithGoogle(): Promise<User> {
  const services = getFirebaseServices();
  if (!services) {
    throw new Error("Firebase Auth is only available in the browser.");
  }
  const provider = new GoogleAuthProvider();
  provider.setCustomParameters({ prompt: "select_account" });
  const result = await signInWithPopup(services.auth, provider);
  return result.user;
}

/**
 * Sign out the current Firebase user.
 */
export async function signOutUser(): Promise<void> {
  const services = getFirebaseServices();
  if (!services) return;
  await signOut(services.auth);
}

/**
 * Read all per-level personal bests for `uid` directly from the authoritative
 * `levels/{levelId}/scores/{uid}` leaderboard documents (single source of truth).
 */
export async function getUserScores(
  uid: string,
  extraLevelIds: string[] = [],
): Promise<Record<string, number>> {
  const services = getFirebaseServices();
  if (!services || !uid) return {};

  try {
    const q = query(collectionGroup(services.db, "scores"), where("uid", "==", uid));
    const snap = await getDocs(q);
    const out: Record<string, number> = {};
    for (const docSnap of snap.docs) {
      const levelDoc = docSnap.ref.parent.parent;
      const levelId = levelDoc?.id ?? "";
      if (!isValidLevelId(levelId)) continue;
      const score = normalizeScore(typeof docSnap.data().score === "number" ? docSnap.data().score : 0);
      if (score > 0) {
        out[levelId] = score;
      }
    }
    return out;
  } catch {
    // Fallback to direct primary-key reads if the collectionGroup index is still building.
    const candidates = [...new Set([...KNOWN_LEVEL_IDS, ...extraLevelIds])].filter(isValidLevelId);
    const pairs = await Promise.all(
      candidates.map(async (levelId) => {
        const snap = await getDoc(doc(services.db, "levels", levelId, "scores", uid));
        if (!snap.exists()) return [levelId, 0] as const;
        const score = normalizeScore(typeof snap.data().score === "number" ? snap.data().score : 0);
        return [levelId, score] as const;
      }),
    );
    const out: Record<string, number> = {};
    for (const [levelId, score] of pairs) {
      if (score > 0) {
        out[levelId] = score;
      }
    }
    return out;
  }
}

/**
 * Read a player's profile (`username` from `users/{uid}` + authoritative `scores` from
 * `levels/{levelId}/scores/{uid}`).
 */
export async function getUserProfile(
  uid: string,
  extraLevelIds: string[] = [],
): Promise<UserScoreDoc | null> {
  const services = getFirebaseServices();
  if (!services || !uid) return null;

  const [userSnap, scores] = await Promise.all([
    getDoc(doc(services.db, "users", uid)),
    getUserScores(uid, extraLevelIds),
  ]);

  if (!userSnap.exists()) {
    return null;
  }

  const data = userSnap.data();
  const username = typeof data.username === "string" ? data.username.trim() : "";
  if (!username) {
    return null;
  }

  return {
    username,
    scores,
    updatedAt: toDateOrNull(data.updatedAt),
  };
}

/**
 * Create or change the signed-in player's arcade username (non-unique, any 1–24 char string),
 * merging any unsynced `localScores` and atomically updating `users/{uid}` plus all
 * `levels/{levelId}/scores/{uid}` leaderboard entries.
 */
export async function setUsername(
  rawUsername: string,
  localScores: Record<string, number> = {},
): Promise<UserScoreDoc> {
  const services = getFirebaseServices();
  const user = services?.auth.currentUser;
  if (!services || !user) {
    throw new Error("You must sign in with Google first.");
  }
  const uid = user.uid;

  const username = sanitizeUsername(rawUsername);
  const cleanLocal = sanitizeScoresMap(localScores);
  const existingScores = await getUserScores(uid, Object.keys(cleanLocal));

  const mergedScores: Record<string, number> = { ...existingScores };
  for (const [levelId, localScore] of Object.entries(cleanLocal)) {
    const prev = mergedScores[levelId] ?? 0;
    if (localScore > prev && (levelId in mergedScores || Object.keys(mergedScores).length < 50)) {
      mergedScores[levelId] = localScore;
    }
  }

  if (services.auth.currentUser?.uid !== uid) {
    throw new Error("Signed-in account changed before saving.");
  }

  const batch = writeBatch(services.db);
  batch.set(doc(services.db, "users", uid), {
    username,
    updatedAt: serverTimestamp(),
  });

  for (const [levelId, score] of Object.entries(mergedScores)) {
    if (score > 0) {
      batch.set(doc(services.db, "levels", levelId, "scores", uid), {
        uid,
        username,
        score,
        updatedAt: serverTimestamp(),
      });
    }
  }

  await batch.commit();

  return {
    username,
    scores: mergedScores,
    updatedAt: new Date(),
  };
}

/**
 * Sync genuinely unsynced local scores (`guest` + `pending`) up to `levels/{levelId}/scores/{uid}`
 * and return the authoritative profile (returns `null` if the user has not chosen a handle yet).
 */
export async function syncLocalScores(
  unsyncedLocalScores: Record<string, number>,
  expectedUid?: string,
): Promise<SyncLocalScoresResult | null> {
  const services = getFirebaseServices();
  const user = services?.auth.currentUser;
  if (!services || !user) {
    return null;
  }
  if (expectedUid && user.uid !== expectedUid) {
    return null;
  }
  const uid = user.uid;

  const cleanLocal = sanitizeScoresMap(unsyncedLocalScores);
  const profile = await getUserProfile(uid, Object.keys(cleanLocal));
  if (!profile) {
    return null;
  }

  const mergedScores: Record<string, number> = { ...profile.scores };
  const improvedLevels: string[] = [];

  for (const [levelId, localScore] of Object.entries(cleanLocal)) {
    const cloudBest = mergedScores[levelId] ?? 0;
    if (localScore > cloudBest) {
      if (!(levelId in mergedScores) && Object.keys(mergedScores).length >= 50) {
        continue;
      }
      mergedScores[levelId] = localScore;
      improvedLevels.push(levelId);
    }
  }

  if (improvedLevels.length === 0) {
    return { profile, uploadedCount: 0 };
  }

  if (services.auth.currentUser?.uid !== uid) {
    return null;
  }

  const batch = writeBatch(services.db);
  for (const levelId of improvedLevels) {
    batch.set(doc(services.db, "levels", levelId, "scores", uid), {
      uid,
      username: profile.username,
      score: mergedScores[levelId],
      updatedAt: serverTimestamp(),
    });
  }

  await batch.commit();

  return {
    profile: {
      username: profile.username,
      scores: mergedScores,
      updatedAt: new Date(),
    },
    uploadedCount: improvedLevels.length,
  };
}

/**
 * Submit a single score number against `levelId`. Writes directly to the single-source-of-truth
 * document `levels/{levelId}/scores/{uid}` only if `rawScore` beats the stored high score.
 */
export async function submitLevelScore(
  levelId: string,
  rawScore: number,
  expectedUid?: string,
): Promise<{ updated: boolean; bestScore: number }> {
  const services = getFirebaseServices();
  const user = services?.auth.currentUser;
  const score = normalizeScore(rawScore);

  if (!services || !user || !isValidLevelId(levelId) || score <= 0) {
    return { updated: false, bestScore: score };
  }
  if (expectedUid && user.uid !== expectedUid) {
    return { updated: false, bestScore: score };
  }
  const uid = user.uid;

  const [userSnap, boardSnap] = await Promise.all([
    getDoc(doc(services.db, "users", uid)),
    getDoc(doc(services.db, "levels", levelId, "scores", uid)),
  ]);

  if (!userSnap.exists()) {
    return { updated: false, bestScore: score };
  }
  const username = typeof userSnap.data().username === "string" ? userSnap.data().username.trim() : "";
  if (!username) {
    return { updated: false, bestScore: score };
  }

  const prevBest = boardSnap.exists()
    ? normalizeScore(typeof boardSnap.data().score === "number" ? boardSnap.data().score : 0)
    : 0;
  if (score <= prevBest) {
    return { updated: false, bestScore: prevBest };
  }

  if (services.auth.currentUser?.uid !== uid) {
    return { updated: false, bestScore: prevBest };
  }

  await setDoc(doc(services.db, "levels", levelId, "scores", uid), {
    uid,
    username,
    score,
    updatedAt: serverTimestamp(),
  });

  return { updated: true, bestScore: score };
}

/**
 * Fetch the Top N (default 10) high scores for a level from `levels/{levelId}/scores`.
 */
export async function getLevelHighScores(
  levelId: string,
  limitCount: number = DEFAULT_LEADERBOARD_LIMIT,
): Promise<LeaderboardEntry[]> {
  const services = getFirebaseServices();
  if (!services || !isValidLevelId(levelId)) {
    return [];
  }

  const q = query(
    collection(services.db, "levels", levelId, "scores"),
    orderBy("score", "desc"),
    limit(Math.max(1, Math.min(50, Math.floor(limitCount)))),
  );

  const snap = await getDocs(q);
  return snap.docs.map((docSnap) => {
    const d = docSnap.data();
    return {
      uid: typeof d.uid === "string" ? d.uid : docSnap.id,
      username: typeof d.username === "string" ? d.username : "ANON",
      score: normalizeScore(typeof d.score === "number" ? d.score : 0),
      updatedAt: toDateOrNull(d.updatedAt),
    };
  });
}

/**
 * Fetch Top N high scores across multiple levels in parallel.
 */
export async function getAllLevelHighScores(
  levelIds: string[],
  limitCount: number = DEFAULT_LEADERBOARD_LIMIT,
): Promise<Record<string, LeaderboardEntry[]>> {
  const entries = await Promise.all(
    levelIds.map(async (id) => [id, await getLevelHighScores(id, limitCount)] as const),
  );
  return Object.fromEntries(entries);
}
