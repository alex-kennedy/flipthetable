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
};

export type FirebaseServices = {
  app: FirebaseApp;
  auth: Auth;
  db: Firestore;
};

let cachedServices: FirebaseServices | null = null;

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

  cachedServices = { app, auth, db };
  return cachedServices;
}
