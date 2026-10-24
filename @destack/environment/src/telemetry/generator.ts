import type { IdGenerator } from "@opentelemetry/sdk-trace";
import type { Clock } from "../clock/clock.ts";
import type { Random } from "../random/random.ts";

/** The bytes of a trace identifier, as W3C Trace Context sets them. */
const TRACE_ID_BYTES = 16;

/** The bytes of a span identifier, as W3C Trace Context sets them. */
const SPAN_ID_BYTES = 8;

/** The bytes of a trace identifier's millisecond prefix, covering dates until the year 10889. */
const TIME_BYTES = 6;

/** Generates trace identifiers that start with their creation millisecond. */
export class TraceIdGenerator implements IdGenerator {
    /** The clock and randomness the identifiers come from. */
    readonly #environment: { readonly clock: Clock; readonly random: Random };

    /** Generate identifiers from an environment's clock and randomness. */
    constructor(environment: { readonly clock: Clock; readonly random: Random }) {
        this.#environment = environment;
    }

    /** Generate a 16-byte trace identifier: 6 bytes of Unix milliseconds, then 10 random bytes. */
    generateTraceId(): string {
        const { clock, random } = this.#environment;
        const time = clock
            .now()
            .toString(16)
            .padStart(TIME_BYTES * 2, "0");

        return `${time}${random.bytes(TRACE_ID_BYTES - TIME_BYTES).toHex()}`;
    }

    /** Generate a random 8-byte span identifier. */
    generateSpanId(): string {
        return this.#environment.random.bytes(SPAN_ID_BYTES).toHex();
    }
}
