/** The states of a run: due, waiting to wake, paused by its actor, or ended. */
export const RUN_STATES = [
    "pending",
    "waiting",
    "paused",
    "completed",
    "failed",
    "cancelled",
] as const;

/** The state of a run. */
export type RunState = (typeof RUN_STATES)[number];

/** The states of a live run, one per key at a time. */
export const LIVE_STATES = ["pending", "waiting", "paused"] as const satisfies readonly RunState[];

/** The states a run ends in. */
export const ENDED_STATES = [
    "completed",
    "failed",
    "cancelled",
] as const satisfies readonly RunState[];

/** A state a run ends in. */
export type EndedState = (typeof ENDED_STATES)[number];
