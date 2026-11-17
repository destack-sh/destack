import { REASON_ATTRIBUTE } from "@destack/environment";
import { schema } from "@destack/schema";
import type { CapturedRecord, TestTelemetry } from "@destack/telemetry/test";

/** A refusal a test reads: its code, its reason and its message. */
export type Refusal = readonly [string, string, string];

/** The refusals a world's telemetry captured, as tests read them. */
export const Refusal = {
    /** Read a captured exception as a refusal, an undeclared failure's reason being its code. */
    of(record: CapturedRecord): Refusal {
        // read the code, and the reason a declared failure records beside it
        const { attributes } = record;
        const code = schema.string().parse(attributes["error.type"]);
        const reason = schema.string().exactOptional().parse(attributes[REASON_ATTRIBUTE]) ?? code;

        return [code, reason, schema.string().parse(attributes["exception.message"])];
    },

    /** List the exceptions a world captured since the last take, but the skipped ones. */
    captured(
        telemetry: TestTelemetry,
        isSkipped: (record: CapturedRecord) => boolean = () => false,
    ): CapturedRecord[] {
        return telemetry
            .records()
            .filter((record) => record.event === "exception" && !isSkipped(record));
    },

    /** Take the exceptions a world captured since the last take as refusals, but skipped ones. */
    take(
        telemetry: TestTelemetry,
        isSkipped: (record: CapturedRecord) => boolean = () => false,
    ): Refusal[] {
        const refusals = Refusal.captured(telemetry, isSkipped).map((record) => Refusal.of(record));
        telemetry.take();

        return refusals;
    },
};
