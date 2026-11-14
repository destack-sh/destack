import type { Given, SetStep } from "@destack/package/declare";
import type { Clock } from "@destack/environment";
import type { SimulatedTelemetry } from "@destack/environment/simulation";
import { type JsonValue, present, schema } from "@destack/schema";
import type { Driver } from "@destack/test";
import {
    terminalInteraction,
    type TerminalObservation,
    type TerminalStep,
} from "../scenario/interaction.ts";
import { TerminalFixture } from "./terminal.ts";

/** A driver that plays terminal scenarios as command lines on the owner's home computer. */
export class TerminalDriver implements Driver<TerminalStep, TerminalObservation> {
    /** The interaction the driver plays. */
    static readonly interaction = terminalInteraction;
    /** The settings its worlds run with: SQLite in WAL, synchronous NORMAL, as in production. */
    static readonly configuration = { durability: "normal" };

    /** The terminal on the owner's home computer. */
    readonly #terminal: TerminalFixture;
    /** What the last command line printed. */
    #output = "";

    /** Take an opened terminal. */
    constructor(terminal: TerminalFixture) {
        this.#terminal = terminal;
    }

    /** The world's clock. */
    get clock(): Clock {
        return this.#terminal.computer.clock;
    }

    /** The telemetry the world records. */
    get telemetry(): SimulatedTelemetry {
        return this.#terminal.computer.world.simulation.telemetry;
    }

    /** Open a terminal on the owner's home computer, given no examples. */
    static async start(_given: Given): Promise<TerminalDriver> {
        return new TerminalDriver(await TerminalFixture.open());
    }

    /** Run a command line until it ends, refusing one that fails. */
    async act(step: SetStep | TerminalStep): Promise<void> {
        // refuse the set step, as a terminal world has no example properties
        if (step.action === "set") {
            throw new TypeError("a terminal world takes no example properties");
        }

        // run the command line, refusing a failure with what it printed
        const printed = await this.#terminal.printed(step.words);
        if (printed.code !== 0) {
            throw new TypeError(
                `'${step.words.join(" ")}' exited ${printed.code}: '${printed.error}'`,
            );
        }
        this.#output = printed.output;
    }

    /** Read a field of what the last command line printed, of each item when it printed a list. */
    observe(observation: TerminalObservation): JsonValue {
        // parse the output, then read the field of the object or of each item
        const printed = schema.json().parse(JSON.parse(this.#output));
        const field = (value: JsonValue) =>
            present(
                schema.record(schema.string(), schema.json()).parse(value)[observation.field],
                `the printed field ${observation.field}`,
            );

        return Array.isArray(printed) ? printed.map(field) : field(printed);
    }

    /** Read the fingerprint of everything the world did so far. */
    fingerprint(): number {
        return this.#terminal.computer.world.simulation.fingerprint;
    }

    /** Close the terminal's computer. */
    async [Symbol.asyncDispose](): Promise<void> {
        await this.#terminal[Symbol.asyncDispose]();
    }
}
