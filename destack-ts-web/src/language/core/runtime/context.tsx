import { ReactiveSession } from "@destack-web/language/core/runtime/session";
import { ACTIVE_SESSION, Supergraph } from "@destack/language";
import { useSignals } from "@preact/signals-react/runtime";
import React, { createContext, useContext, type ReactNode } from "react";

const SessionContext = createContext<ReactiveSession | null>(null);

interface SessionProviderProps {
  session: ReactiveSession;
  children: ReactNode;
}

/** Provider component to make a session available to child components */
export const SessionProvider: React.FC<SessionProviderProps> = ({ session, children }) => {
  React.useEffect(() => {
    const prevSession = ACTIVE_SESSION.get();
    ACTIVE_SESSION.set(session);

    return () => {
      ACTIVE_SESSION.set(prevSession);
    };
  }, [session]);

  return <SessionContext.Provider value={session}>{children}</SessionContext.Provider>;
};

/** Get the currently active session from React context (if any) */
export function getActiveSession(): ReactiveSession | null {
  return useContext(SessionContext);
}

/**
 * Get the currently active Session from React context (error if none).
 * Also activates global signals (useSignals)
 * */
export function useSession(): ReactiveSession {
  const session = useContext(SessionContext);
  if (session == null) {
    throw new Error("no active Session");
  }
  useSignals();
  return session;
}

/**
 * Gets the currently active Supergraph.
 */
export function useSupergraph(): Supergraph {
  const session = useSession();
  return session.supergraph;
}
