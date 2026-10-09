/** An hour, in milliseconds. */
export const HOUR = 60 * 60 * 1000;

/** A day, in milliseconds. */
export const DAY = 24 * HOUR;

/** The flush of every telemetry and analytics kind: hot for an hour or until a scope holds its row cap. */
export const TELEMETRY_FLUSH = { maxAge: HOUR, maxRows: 100_000 } as const;
