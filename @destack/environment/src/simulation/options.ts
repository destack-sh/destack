import type { Process } from "../process/process.ts";

/** The variable that runs every world with jitter, exposing unscheduled real work, when `1`. */
const JITTER_VARIABLE = "DESTACK_SIMULATION_JITTER";

/** The variable that prints each world's span tree and histograms as it ends, when `tree`. */
const TELEMETRY_VARIABLE = "DESTACK_TELEMETRY";

/** The options a run gives every world: jitter, and the readback each prints as it ends. */
export class WorldOptions {
    /** Whether real work settles after a seeded random real delay, exposing untracked real work. */
    readonly isJittered: boolean;
    /** The stream a world's readback goes to, absent when the run asks for none. */
    readonly #readback: Process["stderr"] | undefined;

    /** Take the options, which only a run's process gives. */
    private constructor(isJittered: boolean, readback: Process["stderr"] | undefined) {
        this.isJittered = isJittered;
        this.#readback = readback;
    }

    /** Read the options a run asks for from its process's variables. */
    static of(process: Pick<Process, "variables" | "stderr">): WorldOptions {
        const isDescribed = process.variables[TELEMETRY_VARIABLE] === "tree";

        return new WorldOptions(
            process.variables[JITTER_VARIABLE] === "1",
            isDescribed ? process.stderr : undefined,
        );
    }

    /** Print a world's readback as it ends, when the run asks for one. */
    async describe(readback: () => Promise<string>): Promise<void> {
        if (this.#readback !== undefined) {
            this.#readback.writeSync(`${await readback()}\n`);
        }
    }
}
