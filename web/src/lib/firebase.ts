import { getAnalytics, isSupported, type Analytics } from "firebase/analytics";
import { getApp, getApps, initializeApp, type FirebaseApp } from "firebase/app";
import { getAuth, type Auth } from "firebase/auth";
import { getFirestore, type Firestore } from "firebase/firestore";

const firebaseConfig = {
  apiKey:
    process.env.NEXT_PUBLIC_FIREBASE_API_KEY ||
    "AIzaSyDe1IWN23wTYagAxatTe3FNktXC7PwN1MI",
  authDomain:
    process.env.NEXT_PUBLIC_FIREBASE_AUTH_DOMAIN ||
    "flipthetable-e28ab.firebaseapp.com",
  projectId:
    process.env.NEXT_PUBLIC_FIREBASE_PROJECT_ID || "flipthetable-e28ab",
  storageBucket:
    process.env.NEXT_PUBLIC_FIREBASE_STORAGE_BUCKET ||
    "flipthetable-e28ab.firebasestorage.app",
  messagingSenderId:
    process.env.NEXT_PUBLIC_FIREBASE_MESSAGING_SENDER_ID || "445921354660",
  appId:
    process.env.NEXT_PUBLIC_FIREBASE_APP_ID ||
    "1:445921354660:web:0b119297de424050245731",
  ...(process.env.NEXT_PUBLIC_FIREBASE_MEASUREMENT_ID
    ? { measurementId: process.env.NEXT_PUBLIC_FIREBASE_MEASUREMENT_ID }
    : {}),
};

export type FirebaseServices = {
  app: FirebaseApp;
  auth: Auth;
  db: Firestore;
};

let cachedServices: FirebaseServices | null = null;
let analyticsPromise: Promise<Analytics | null> | null = null;

/**
 * Lazily initializes Firebase Analytics when running in a supported browser
 * environment. Safe to call multiple times.
 */
export function initFirebaseAnalytics(app: FirebaseApp): Promise<Analytics | null> {
  if (typeof window === "undefined") {
    return Promise.resolve(null);
  }
  if (!analyticsPromise) {
    analyticsPromise = isSupported()
      .then((supported) => (supported ? getAnalytics(app) : null))
      .catch(() => null);
  }
  return analyticsPromise;
}

/**
 * Returns initialized Firebase client services in the browser, or `null` during
 * Next.js static export server-side prerendering.
 */
export function getFirebaseServices(): FirebaseServices | null {
  if (typeof window === "undefined") {
    return null;
  }
  if (cachedServices) {
    return cachedServices;
  }

  const app = getApps().length > 0 ? getApp() : initializeApp(firebaseConfig);
  const auth = getAuth(app);
  const db = getFirestore(app);
  void initFirebaseAnalytics(app);

  cachedServices = { app, auth, db };
  return cachedServices;
}
