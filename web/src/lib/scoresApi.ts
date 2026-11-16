import {
  GoogleAuthProvider,
  onAuthStateChanged,
  signInWithPopup,
  signOut,
  type User,
} from "firebase/auth";
import {
  collection,
  doc,
  getDoc,
  getDocs,
  limit,
  orderBy,
  query,
  serverTimestamp,
  Timestamp,
  writeBatch,
} from "firebase/firestore";

import { getFirebaseServices } from "./firebase";

export const MAX_USERNAME_LENGTH = 24;
export const MAX_SCORE_VALUE = 100_000_000;
export const DEFAULT_LEADERBOARD_LIMIT = 10;

export type UserScoreDoc = {
  username: string;
  scores: Record<string, number>;
  updatedAt: Date | null;
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
 * Read a player's private profile (`username` + per-level `scores` map) from `users/{uid}`.
 */
export async function getUserProfile(uid: string): Promise<UserScoreDoc | null> {
  const services = getFirebaseServices();
  if (!services) return null;

  const snap = await getDoc(doc(services.db, "users", uid));
  if (!snap.exists()) {
    return null;
  }

  const data = snap.data();
  const username = typeof data.username === "string" ? data.username : "";
  const scores =
    data.scores && typeof data.scores === "object"
      ? sanitizeScoresMap(data.scores as Record<string, unknown>)
      : {};

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
 * Read only the single-number personal best map (`{ [levelId]: score }`) for `uid`.
 */
export async function getUserScores(uid: string): Promise<Record<string, number>> {
  const profile = await getUserProfile(uid);
  return profile?.scores ?? {};
}

/**
 * Create or change the signed-in player's arcade username (non-unique, any 1–24 char string),
 * merging any provided `localScores` and atomically propagating the handle across all
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

  const username = sanitizeUsername(rawUsername);
  const existing = await getUserProfile(user.uid);
  const cleanLocal = sanitizeScoresMap(localScores);

  const mergedScores: Record<string, number> = { ...(existing?.scores ?? {}) };
  for (const [levelId, localScore] of Object.entries(cleanLocal)) {
    const prev = mergedScores[levelId] ?? 0;
    if (localScore > prev && Object.keys(mergedScores).length < 50) {
      mergedScores[levelId] = localScore;
    } else if (levelId in mergedScores && localScore > prev) {
      mergedScores[levelId] = localScore;
    }
  }

  const batch = writeBatch(services.db);
  batch.set(doc(services.db, "users", user.uid), {
    username,
    scores: mergedScores,
    updatedAt: serverTimestamp(),
  });

  for (const [levelId, score] of Object.entries(mergedScores)) {
    if (score > 0) {
      batch.set(doc(services.db, "levels", levelId, "scores", user.uid), {
        uid: user.uid,
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
 * Sync `localStorage` best scores up to Firestore on sign-in and return the merged
 * profile (returns `null` if the user has not chosen a username yet).
 */
export async function syncLocalScores(
  localScores: Record<string, number>,
): Promise<UserScoreDoc | null> {
  const services = getFirebaseServices();
  const user = services?.auth.currentUser;
  if (!services || !user) {
    return null;
  }

  const profile = await getUserProfile(user.uid);
  if (!profile) {
    return null;
  }

  const cleanLocal = sanitizeScoresMap(localScores);
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
    return profile;
  }

  const batch = writeBatch(services.db);
  batch.set(doc(services.db, "users", user.uid), {
    username: profile.username,
    scores: mergedScores,
    updatedAt: serverTimestamp(),
  });

  for (const levelId of improvedLevels) {
    batch.set(doc(services.db, "levels", levelId, "scores", user.uid), {
      uid: user.uid,
      username: profile.username,
      score: mergedScores[levelId],
      updatedAt: serverTimestamp(),
    });
  }

  await batch.commit();

  return {
    username: profile.username,
    scores: mergedScores,
    updatedAt: new Date(),
  };
}

/**
 * Submit a single score number against `levelId`. Writes to `users/{uid}` and
 * `levels/{levelId}/scores/{uid}` only if `rawScore` beats the player's stored best.
 */
export async function submitLevelScore(
  levelId: string,
  rawScore: number,
): Promise<{ updated: boolean; bestScore: number }> {
  const services = getFirebaseServices();
  const user = services?.auth.currentUser;
  const score = normalizeScore(rawScore);

  if (!services || !user || !isValidLevelId(levelId) || score <= 0) {
    return { updated: false, bestScore: score };
  }

  const profile = await getUserProfile(user.uid);
  if (!profile) {
    return { updated: false, bestScore: score };
  }

  const prevBest = profile.scores[levelId] ?? 0;
  if (score <= prevBest) {
    return { updated: false, bestScore: prevBest };
  }

  const nextScores: Record<string, number> = {
    ...profile.scores,
    [levelId]: score,
  };

  const batch = writeBatch(services.db);
  batch.set(doc(services.db, "users", user.uid), {
    username: profile.username,
    scores: nextScores,
    updatedAt: serverTimestamp(),
  });
  batch.set(doc(services.db, "levels", levelId, "scores", user.uid), {
    uid: user.uid,
    username: profile.username,
    score,
    updatedAt: serverTimestamp(),
  });

  await batch.commit();

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
