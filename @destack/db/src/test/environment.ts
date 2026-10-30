import { testHost } from "@destack/test/host";
import { type SimulatedClock, SimulatedScheduler } from "@destack/environment/simulation";
import type { DatabaseEnvironment } from "../database/connection.ts";

/** Make a world on a clock: the host's effects, its work counted on a separate scheduler. */
export function environmentOn(clock: SimulatedClock): DatabaseEnvironment & {
    readonly clock: SimulatedClock;
    readonly scheduler: SimulatedScheduler;
} {
    return { ...testHost(), clock, scheduler: new SimulatedScheduler({ isOrdered: false }) };
}
