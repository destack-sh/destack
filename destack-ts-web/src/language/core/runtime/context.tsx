import type { ReactiveSession } from "@destack-web/language/core/runtime/session";

let ACTIVE_SESSION: ReactiveSession | null = null;

export function setActiveSession(session: ReactiveSession): void {
  ACTIVE_SESSION = session;
}

export function getActiveSession(): ReactiveSession | null {
  return ACTIVE_SESSION;
}

export function useSession(): ReactiveSession {
  if (ACTIVE_SESSION == null) {
    throw new Error("no active session");
  }
  return ACTIVE_SESSION;
}

export function SessionProvider(_: { session: ReactiveSession; children?: unknown }): unknown {
  // no-op provider for compatibility
  setActiveSession(_.session);
  return _.children ?? null;
}
