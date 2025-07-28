import type { ReactiveSession } from "@destack-web/language/core/runtime/session";
import { type ComponentChildren, createContext } from "preact";
import { useContext } from "preact/hooks";

const SessionContext = createContext<ReactiveSession | null>(null);

interface SessionProviderProps {
  session: ReactiveSession;
  children: ComponentChildren;
}

/** Provider component to make a session available to child components */
export const SessionProvider = ({ session, children }: SessionProviderProps) => {
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
  if (!session) {
    throw new Error("useSession must be called within a SessionProvider");
  }
  return session;
}
