import type { Failure } from "@destack/error";
import { expect, test } from "@destack/test";
import { PlanError, ResourceError, type ResourceReason } from "./error.ts";

test("map every resource binding failure reason to the call error its caller receives", () => {
    // convert a failure of each reason
    const reasons: readonly ResourceReason[] = ["NOT_BOUND", "ALREADY_BOUND"];
    const received = Object.fromEntries(
        reasons.map((reason) => [reason, new ResourceError(reason, "failed").toCallError()]),
    );

    // receive each reason's call error with the failure's message
    const expected: Readonly<Record<ResourceReason, Failure>> = {
        NOT_BOUND: {
            code: "INTERNAL_SERVER_ERROR",
            reason: "NOT_BOUND",
            domain: "@destack/resource",
            message: "failed",
        },
        ALREADY_BOUND: {
            code: "INTERNAL_SERVER_ERROR",
            reason: "ALREADY_BOUND",
            domain: "@destack/resource",
            message: "failed",
        },
    };
    expect(received).toEqual(expected);
});

test("report a plan no declaration change reaches as a conflict listing every problem", () => {
    const failure = new PlanError([
        { target: "table/note", detail: "declare a conversion to version 2" },
        { target: "table/tag", detail: "declare the dropped column" },
    ]);

    expect(failure.toCallError()).toEqual({
        code: "CONFLICT",
        reason: "UNREACHABLE_PLAN",
        domain: "@destack/resource",
        message:
            "table/note: declare a conversion to version 2; table/tag: declare the dropped column",
    });
});
