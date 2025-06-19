import { Temporal } from "temporal-polyfill"
import { v4 as uuidv4 } from 'uuid';
import { Session } from "@/language";

// forever constants
export const VERSION = "2025.06.19.0"
export const FLOAT_EPSILON = 1e-6
export const BEGINNING_OF_TIME = Temporal.ZonedDateTime.from("1970-01-01T00:00:00+00:00")

// builtin destackes :Builtins
export const DESTACK_SLUG = "destack"
export const DESTACK_ID = "11111111-1111-1111-1111-000000000000"

// runtime constants
export const NONCE = uuidv4()
export const UNSET = Symbol("UNSET")
export const EMPTY_LIST: any[] = []
export const EMPTY_SET: Set<any> = new Set()
export const EMPTY_DICT: Record<string, any> = {}

// runtime context
export const IS_IN_USER_CODE = false
export const ACTIVE_SESSION: Session | null = null
