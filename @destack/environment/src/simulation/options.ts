import type { Process } from "../process/process.ts";
import type { ReadableSpan } from "@opentelemetry/sdk-trace";
import type { SimulatedTelemetry } from "../telemetry/simulated.ts";

/** The variable that runs every world with jitter, exposing unscheduled real work, when `1`. */
const JITTER_VARIABLE = "DESTACK_SIMULATION_JITTER";

/** The variable that prints each world's span tree and histograms as it ends, when `tree`. */
const TELEMETRY_VARIABLE = "DESTACK_TELEMETRY";

/** The variable that records where each world's work began, listed in its failures, when `1`. */
export const SITES_VARIABLE = "DESTACK_SIMULATION_SITES";

/** The options a run gives every world: jitter, and what each prints and exports as it ends. */
export class WorldOptions {
    /** Whether real work settles after a seeded random real delay, exposing untracked real work. */
    readonly isJittered: boolean;
    /** Whether each work records where it began, which the world's failures list. */
    readonly isRecordingSites: boolean;
    /** The stream a world's readback goes to, absent when the run asks for none. */
    readonly #readback: Process["stderr"] | undefined;
    /** Post a world's host spans as it ends, absent when the run asks for no export. */
    readonly #exported: ((spans: readonly ReadableSpan[]) => Promise<void>) | undefined;

    /** Take the options, which only a run's process gives. */
    private constructor(
        isJittered: boolean,
        isRecordingSites: boolean,
        readback: Process["stderr"] | undefined,
        exported: ((spans: readonly ReadableSpan[]) => Promise<void>) | undefined,
    ) {
        // set the jitter, the sites, the readback and the export
        this.isJittered = isJittered;
        this.isRecordingSites = isRecordingSites;
        this.#readback = readback;
        this.#exported = exported;
    }

    /** Read the options a run asks for from its process's variables, with its spans' export. */
    static of(
        process: Pick<Process, "variables" | "stderr">,
        exported: ((spans: readonly ReadableSpan[]) => Promise<void>) | undefined,
    ): WorldOptions {
        const { variables } = process;

        return new WorldOptions(
            variables[JITTER_VARIABLE] === "1",
            variables[SITES_VARIABLE] === "1",
            variables[TELEMETRY_VARIABLE] === "tree" ? process.stderr : undefined,
            exported,
        );
    }

    /** Print a world's readback and export its spans as it ends, as the run asks. */
    async end(telemetry: SimulatedTelemetry): Promise<void> {
        // print the span tree and histograms
        if (this.#readback !== undefined) {
            this.#readback.writeSync(`${await telemetry.describe()}\n`);
        }

        // export the spans on the host's clock
        await this.#exported?.(telemetry.hostSpans);
    }
}
