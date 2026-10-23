import type { Failure } from "@destack/error";
import { expect, test } from "@destack/test";
import { EventError, type EventReason } from "./error.ts";

test("map every event failure reason to the call error its caller receives", () => {
    // convert a failure of each reason
    const reasons: readonly EventReason[] = [
        "INVALID_CURSOR",
        "CONTENTS_DIFFER",
        "CONCURRENT_UPDATE",
    ];
    const received = Object.fromEntries(
        reasons.map((reason) => [reason, new EventError(reason, "failed").toCallError()]),
    );

    // receive each reason's call error with the failure's message
    const expected: Readonly<Record<EventReason, Failure>> = {
        INVALID_CURSOR: {
            code: "BAD_REQUEST",
            reason: "INVALID_CURSOR",
            domain: "@destack/event",
            message: "failed",
        },
        CONTENTS_DIFFER: {
            code: "CONFLICT",
            reason: "CONTENTS_DIFFER",
            domain: "@destack/event",
            message: "failed",
        },
        CONCURRENT_UPDATE: {
            code: "SERVICE_UNAVAILABLE",
            reason: "CONCURRENT_UPDATE",
            domain: "@destack/event",
            message: "failed",
        },
    };
    expect(received).toEqual(expected);
});
