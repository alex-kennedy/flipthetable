#!/usr/bin/env node
/**
 * Firestore High-Score Integrity Checker & Reconciler (`flipthetable-e28ab`)
 *
 * Single-Source-of-Truth Schema:
 *   - `users/{uid}` -> `{ username: string, updatedAt: Timestamp }` (NO duplicate scores map!)
 *   - `levels/{levelId}/scores/{uid}` -> `{ uid: string, username: string, score: int, updatedAt: Timestamp }`
 *
 * Audits (and repairs with `--fix`):
 *   1. Legacy/unexpected fields on `users/{uid}` (e.g. stale `scores` map that could diverge from leaderboards)
 *   2. Missing `users/{uid}` profile doc for any UID present on `levels/{levelId}/scores/{uid}`
 *   3. Username desync (`levels/{levelId}/scores/{uid}.username !== users/{uid}.username`)
 *   4. UID mismatch (`levels/{levelId}/scores/{uid}.uid !== uid`), non-integer or out-of-bounds scores, or extra fields
 *
 * Usage:
 *   npm --prefix web run db:check        # Read-only audit (exits 1 if issues found)
 *   npm --prefix web run db:fix          # Atomically repairs all issues via Firestore :commit
 */

import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const PROJECT_ID = process.env.NEXT_PUBLIC_FIREBASE_PROJECT_ID || "flipthetable-e28ab";
const DATABASE_ROOT = `projects/${PROJECT_ID}/databases/(default)/documents`;
const BASE_URL = `https://firestore.googleapis.com/v1/${DATABASE_ROOT}`;
const FIX_MODE = process.argv.includes("--fix");

const ALLOWED_USER_FIELDS = new Set(["username", "updatedAt"]);
const ALLOWED_BOARD_FIELDS = new Set(["uid", "username", "score", "updatedAt"]);

function getAccessToken() {
  if (process.env.FIRESTORE_ACCESS_TOKEN) {
    return process.env.FIRESTORE_ACCESS_TOKEN.trim();
  }
  for (const args of [
    ["auth", "print-access-token"],
    ["auth", "application-default", "print-access-token"],
  ]) {
    try {
      const out = execFileSync("gcloud", args, {
        encoding: "utf8",
        stdio: ["ignore", "pipe", "ignore"],
      }).trim();
      if (out) return out;
    } catch {
      // Try next token source.
    }
  }
  throw new Error(
    "Could not obtain a GCP access token. Run `gcloud auth login` or set FIRESTORE_ACCESS_TOKEN.",
  );
}

async function apiFetch(url, token, init = {}) {
  const res = await fetch(url, {
    ...init,
    headers: {
      Authorization: `Bearer ${token}`,
      "Content-Type": "application/json",
      ...(init.headers || {}),
    },
  });
  if (!res.ok) {
    const body = await res.text();
    throw new Error(`Firestore API ${res.status} ${res.statusText} on ${url}: ${body}`);
  }
  return res.json();
}

function parseFirestoreValue(val) {
  if (!val || typeof val !== "object") return undefined;
  if ("integerValue" in val) return Number(val.integerValue);
  if ("doubleValue" in val) return Number(val.doubleValue);
  if ("stringValue" in val) return val.stringValue;
  if ("booleanValue" in val) return Boolean(val.booleanValue);
  if ("timestampValue" in val) return val.timestampValue;
  if ("nullValue" in val) return null;
  if ("mapValue" in val) {
    const out = {};
    for (const [k, v] of Object.entries(val.mapValue?.fields || {})) {
      out[k] = parseFirestoreValue(v);
    }
    return out;
  }
  if ("arrayValue" in val) {
    return (val.arrayValue?.values || []).map(parseFirestoreValue);
  }
  return undefined;
}

function parseDocFields(doc) {
  const out = {};
  for (const [k, v] of Object.entries(doc.fields || {})) {
    out[k] = parseFirestoreValue(v);
  }
  return out;
}

function docIdFromName(name) {
  const parts = name.split("/");
  return parts[parts.length - 1];
}

async function listAllDocuments(collectionPath, token) {
  const docs = [];
  let pageToken = "";
  do {
    const qs = new URLSearchParams({ pageSize: "300" });
    if (pageToken) qs.set("pageToken", pageToken);
    const data = await apiFetch(`${BASE_URL}/${collectionPath}?${qs.toString()}`, token);
    if (Array.isArray(data.documents)) {
      docs.push(...data.documents);
    }
    pageToken = data.nextPageToken || "";
  } while (pageToken);
  return docs;
}

function loadManifestLevelIds() {
  try {
    const manifestPath = resolve(__dirname, "../public/assets/scenarios/manifest.json");
    const raw = JSON.parse(readFileSync(manifestPath, "utf8"));
    return (raw.scenarios || []).map((s) => s.id).filter(Boolean);
  } catch {
    return ["corporate", "building_site", "boardroom", "all_hands", "breakroom", "warehouse"];
  }
}

async function discoverLevelIds(token) {
  const levelIds = new Set(loadManifestLevelIds());

  const cgData = await apiFetch(`${BASE_URL}:runQuery`, token, {
    method: "POST",
    body: JSON.stringify({
      structuredQuery: {
        from: [{ collectionId: "scores", allDescendants: true }],
      },
    }),
  });

  if (Array.isArray(cgData)) {
    for (const row of cgData) {
      const match = row.document?.name?.match(/\/levels\/([^/]+)\/scores\/([^/]+)$/);
      if (match) {
        levelIds.add(match[1]);
      }
    }
  }

  return [...levelIds].sort();
}

function isValidIntScore(val) {
  return typeof val === "number" && Number.isInteger(val) && val > 0 && val <= 100_000_000;
}

function isValidHandle(name) {
  return (
    typeof name === "string" &&
    name.length >= 1 &&
    name.length <= 24 &&
    name.trim().length >= 1
  );
}

async function main() {
  console.log(
    `Checking Firestore integrity for project: ${PROJECT_ID} (${FIX_MODE ? "FIX MODE" : "AUDIT ONLY"})`,
  );
  const token = getAccessToken();

  const rawUsers = await listAllDocuments("users", token);
  const usersByUid = new Map();
  for (const raw of rawUsers) {
    const uid = docIdFromName(raw.name);
    const parsed = parseDocFields(raw);
    usersByUid.set(uid, {
      uid,
      raw,
      parsed,
      rawFields: raw.fields || {},
      username: typeof parsed.username === "string" ? parsed.username : "",
    });
  }

  const levelIds = await discoverLevelIds(token);
  const boardByLevel = new Map();
  for (const levelId of levelIds) {
    const rawScores = await listAllDocuments(`levels/${encodeURIComponent(levelId)}/scores`, token);
    const byUid = new Map();
    for (const raw of rawScores) {
      const docUid = docIdFromName(raw.name);
      const parsed = parseDocFields(raw);
      byUid.set(docUid, {
        docUid,
        raw,
        parsed,
        rawFields: raw.fields || {},
        uidField: parsed.uid,
        username: typeof parsed.username === "string" ? parsed.username : "",
        score: parsed.score,
      });
    }
    boardByLevel.set(levelId, byUid);
  }

  console.log(
    `Loaded ${usersByUid.size} user profile(s) across ${levelIds.length} level(s): [${levelIds.join(", ")}]\n`,
  );

  for (const [uid, u] of usersByUid.entries()) {
    const extraKeys = Object.keys(u.rawFields).filter((k) => !ALLOWED_USER_FIELDS.has(k));
    const extraNote = extraKeys.length > 0 ? ` [EXTRA FIELDS: ${extraKeys.join(", ")}]` : "";
    console.log(`  User ${uid}: username="${u.username}"${extraNote}`);
  }
  for (const [levelId, byUid] of boardByLevel.entries()) {
    if (byUid.size > 0) {
      const summary = [...byUid.values()]
        .sort((a, b) => Number(b.score || 0) - Number(a.score || 0))
        .map((e) => `${e.username}[${e.docUid.slice(0, 6)}..]=${e.score}`)
        .join(", ");
      console.log(`  Leaderboard [${levelId}]: ${summary}`);
    }
  }
  console.log("");

  const issues = [];
  const userRewrites = new Map(); // uid -> { username }
  const boardRewrites = new Map(); // `${levelId}/${uid}` -> { levelId, uid, username, score }

  const allUids = new Set(usersByUid.keys());
  for (const byUid of boardByLevel.values()) {
    for (const uid of byUid.keys()) {
      allUids.add(uid);
    }
  }

  for (const uid of allUids) {
    const userDoc = usersByUid.get(uid) || null;
    let canonicalUsername = userDoc?.username?.trim().slice(0, 24) || "";

    if (!canonicalUsername) {
      for (const levelId of levelIds) {
        const entry = boardByLevel.get(levelId)?.get(uid);
        if (entry?.username?.trim()) {
          canonicalUsername = entry.username.trim().slice(0, 24);
          break;
        }
      }
    }
    if (!canonicalUsername) {
      canonicalUsername = "PLAYER";
    }

    if (!userDoc) {
      issues.push({
        type: "MISSING_USER_DOC",
        detail: `Leaderboard entries exist for uid=${uid}, but users/${uid} is missing.`,
      });
      userRewrites.set(uid, { username: canonicalUsername });
    } else {
      const extraUserFields = Object.keys(userDoc.rawFields).filter(
        (k) => !ALLOWED_USER_FIELDS.has(k),
      );
      if (extraUserFields.length > 0) {
        issues.push({
          type: "REDUNDANT_USER_FIELDS",
          detail: `users/${uid} contains redundant/legacy field(s) [${extraUserFields.join(", ")}]=${JSON.stringify(
            Object.fromEntries(extraUserFields.map((k) => [k, userDoc.parsed[k]])),
          )} that can diverge from authoritative levels/*/scores/${uid} docs.`,
        });
        userRewrites.set(uid, { username: canonicalUsername });
      }
      if (!isValidHandle(userDoc.username) || userDoc.username !== canonicalUsername) {
        issues.push({
          type: "INVALID_USER_HANDLE",
          detail: `users/${uid} has invalid/untrimmed username="${userDoc.username}".`,
        });
        userRewrites.set(uid, { username: canonicalUsername });
      }
      if (!userDoc.rawFields.updatedAt || !("timestampValue" in userDoc.rawFields.updatedAt)) {
        issues.push({
          type: "MISSING_USER_TIMESTAMP",
          detail: `users/${uid} is missing a valid timestamp updatedAt.`,
        });
        userRewrites.set(uid, { username: canonicalUsername });
      }
    }

    for (const levelId of levelIds) {
      const entry = boardByLevel.get(levelId)?.get(uid);
      if (!entry) continue;

      let needsBoardRewrite = false;
      const safeScore = Math.max(1, Math.min(100_000_000, Math.floor(Number(entry.score) || 1)));

      const extraBoardFields = Object.keys(entry.rawFields).filter(
        (k) => !ALLOWED_BOARD_FIELDS.has(k),
      );
      if (extraBoardFields.length > 0) {
        issues.push({
          type: "EXTRA_BOARD_FIELDS",
          detail: `levels/${levelId}/scores/${uid} has extra field(s): [${extraBoardFields.join(", ")}].`,
        });
        needsBoardRewrite = true;
      }

      if (entry.uidField !== uid) {
        issues.push({
          type: "UID_FIELD_MISMATCH",
          detail: `levels/${levelId}/scores/${uid} has uid="${entry.uidField}" !== "${uid}".`,
        });
        needsBoardRewrite = true;
      }

      if (entry.username !== canonicalUsername) {
        issues.push({
          type: "USERNAME_DESYNC",
          detail: `levels/${levelId}/scores/${uid}.username="${entry.username}" !== users/${uid}.username="${canonicalUsername}".`,
        });
        needsBoardRewrite = true;
      }

      if (!isValidIntScore(entry.score) || !("integerValue" in (entry.rawFields.score || {}))) {
        issues.push({
          type: "INVALID_BOARD_SCORE_TYPE",
          detail: `levels/${levelId}/scores/${uid}.score is non-integer or out of bounds (${JSON.stringify(entry.score)}).`,
        });
        needsBoardRewrite = true;
      }

      if (!entry.rawFields.updatedAt || !("timestampValue" in entry.rawFields.updatedAt)) {
        issues.push({
          type: "MISSING_BOARD_TIMESTAMP",
          detail: `levels/${levelId}/scores/${uid} is missing a valid timestamp updatedAt.`,
        });
        needsBoardRewrite = true;
      }

      if (needsBoardRewrite) {
        boardRewrites.set(`${levelId}/${uid}`, {
          levelId,
          uid,
          username: canonicalUsername,
          score: safeScore,
        });
      }
    }
  }

  if (issues.length === 0) {
    console.log("✔ Database integrity check passed: 0 issues found (single source of truth verified).");
    return;
  }

  console.log(`✖ Found ${issues.length} integrity issue(s):`);
  for (const issue of issues) {
    console.log(`  - [${issue.type}] ${issue.detail}`);
  }

  if (!FIX_MODE) {
    console.log("\nRun `npm --prefix web run db:fix` to repair all issues atomically.");
    process.exitCode = 1;
    return;
  }

  const writes = [];
  for (const [uid, u] of userRewrites.entries()) {
    writes.push({
      update: {
        name: `${DATABASE_ROOT}/users/${uid}`,
        fields: {
          username: { stringValue: u.username },
        },
      },
      updateTransforms: [
        {
          fieldPath: "updatedAt",
          setToServerValue: "REQUEST_TIME",
        },
      ],
    });
  }

  for (const b of boardRewrites.values()) {
    writes.push({
      update: {
        name: `${DATABASE_ROOT}/levels/${b.levelId}/scores/${b.uid}`,
        fields: {
          uid: { stringValue: b.uid },
          username: { stringValue: b.username },
          score: { integerValue: String(b.score) },
        },
      },
      updateTransforms: [
        {
          fieldPath: "updatedAt",
          setToServerValue: "REQUEST_TIME",
        },
      ],
    });
  }

  await apiFetch(`${BASE_URL}:commit`, token, {
    method: "POST",
    body: JSON.stringify({ writes }),
  });

  console.log(`\n✔ Repaired ${writes.length} document(s) atomically.`);
}

main().catch((err) => {
  console.error("Integrity check error:", err instanceof Error ? err.message : err);
  process.exit(1);
});
