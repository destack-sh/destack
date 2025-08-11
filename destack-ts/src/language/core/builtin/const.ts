import type { Branch, Session, Snapshot, Space } from "@destack/language";
import { uuid4 } from "@destack/utils";
import { ContextVar } from "@destack/utils/context";

// forever constants
export const VERSION = "2025.08.11.1";
export const EPSILON = 1e-6;
export const EPSILON_EXPONENT = 6;

// runtime constants
export const NONCE = uuid4();
export const UNSET = Symbol("UNSET");
export const EMPTY_LIST: any[] = [];
export const EMPTY_SET: Set<any> = new Set();
export const EMPTY_DICT: Record<string, any> = {};

// runtime context
export const ACTIVE_SESSION: ContextVar<Session | null> = new ContextVar(null);
export const ACTIVE_SPACE: ContextVar<Space | null> = new ContextVar(null);
export const ACTIVE_BRANCH: ContextVar<Branch | null> = new ContextVar(null);
export const ACTIVE_SNAPSHOT: ContextVar<Snapshot | null> = new ContextVar(null);
export const ACTIVE_EVENT: ContextVar<Event | null> = new ContextVar(null);

/** Gets the currently active Session (if any). */
export function getActiveSession(): Session | null {
  return ACTIVE_SESSION.get();
}

/** Gets the currently active Session (error if none). */
export function activeSession(): Session {
  const session = ACTIVE_SESSION.get();
  if (session == null) {
    throw new Error("no active Session");
  }
  return session;
}

/** Gets the currently active Space (if any). */
export function getActiveSpace(): Space | null {
  return ACTIVE_SPACE.get();
}

/** Gets the currently active Space (error if none). */
export function activeSpace(): Space {
  const space = ACTIVE_SPACE.get();
  if (space == null) {
    throw new Error("no active Space");
  }
  return space;
}

/** Gets the currently active Branch (if any). */
export function getActiveBranch(): Branch | null {
  return ACTIVE_BRANCH.get();
}

/** Gets the currently active Branch (error if none). */
export function activeBranch(): Branch {
  const branch = ACTIVE_BRANCH.get();
  if (branch == null) {
    throw new Error("no active Branch");
  }
  return branch;
}

/** Gets the currently active Snapshot (if any). */
export function getActiveSnapshot(): Snapshot | null {
  return ACTIVE_SNAPSHOT.get();
}

/** Gets the currently active Snapshot (error if none). */
export function activeSnapshot(): Snapshot {
  const snapshot = ACTIVE_SNAPSHOT.get();
  if (snapshot == null) {
    throw new Error("no active Snapshot");
  }
  return snapshot;
}

/** Gets the currently active Event (if any). */
export function getActiveEvent(): Event | null {
  return ACTIVE_EVENT.get();
}

/** Gets the currently active Event (error if none). */
export function activeEvent(): Event {
  const event = ACTIVE_EVENT.get();
  if (event == null) {
    throw new Error("no active Event");
  }
  return event;
}
