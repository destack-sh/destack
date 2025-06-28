import { Session } from "@destack/language";
import { ContextVar } from "@destack/utils/context";
import { Temporal } from "temporal-polyfill";
import { v4 as uuidv4 } from "uuid";

// forever constants
export const VERSION = "2025.06.28.1";
export const FLOAT_EPSILON = 1e-6;
export const BEGINNING_OF_TIME = Temporal.ZonedDateTime.from("1970-01-01T00:00:00+00:00[UTC]");

// runtime constants
export const NONCE = uuidv4();
export const UNSET = Symbol("UNSET");
export const EMPTY_LIST: any[] = [];
export const EMPTY_SET: Set<any> = new Set();
export const EMPTY_DICT: Record<string, any> = {};

// runtime context
export const IS_IN_USER_CODE: ContextVar<boolean> = new ContextVar(false);
export const ACTIVE_SESSION: ContextVar<Session | null> = new ContextVar(null);

export function activeSession(): Session {
  const session = ACTIVE_SESSION.get();
  if (session == null) {
    throw new Error("no active Session");
  }
  return session;
}
