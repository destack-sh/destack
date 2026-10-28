import type { LogPosition } from "@destack/db";

/** The revision a test follower subscribes at: the first position of its own log. */
export const FOLLOWER_REVISION: LogPosition = { log: "follower", epoch: "follower", sequence: 1 };
