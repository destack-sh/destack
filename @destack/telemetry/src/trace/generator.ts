import type { IdGenerator } from "@opentelemetry/sdk-trace";

/** The bytes of a trace identifier's millisecond prefix, covering dates until the year 10889. */
const TIME_BYTES = 6;

/** Generates trace identifiers starting with their creation millisecond, so lookups know the time. */
export class TraceIdGenerator implements IdGenerator {
    /** Generate a 16-byte trace identifier: 6 bytes of Unix milliseconds followed by 10 random bytes. */
    generateTraceId(): string {
        const time = Date.now()
            .toString(16)
            .padStart(TIME_BYTES * 2, "0");
        const random = crypto.getRandomValues(new Uint8Array(16 - TIME_BYTES)).toHex();

        return `${time}${random}`;
    }

    /** Generate a random 8-byte span identifier. */
    generateSpanId(): string {
        return crypto.getRandomValues(new Uint8Array(8)).toHex();
    }
}
