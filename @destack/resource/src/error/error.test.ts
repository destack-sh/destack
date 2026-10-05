import type { ServiceErrorReport } from "@destack/schema";
import { expect, test } from "@destack/test";
import { PlanError, ResourceError, type ResourceErrorCode } from "./error.ts";

test("map every binding failure code to the service error its caller receives", () => {
    // convert a failure of each code
    const codes: readonly ResourceErrorCode[] = ["NOT_BOUND", "ALREADY_BOUND"];
    const received = Object.fromEntries(
        codes.map((code) => [code, new ResourceError(code, "failed").toServiceError()]),
    );

    // receive each code's service error with the failure's message
    const expected: Readonly<Record<ResourceErrorCode, ServiceErrorReport>> = {
        NOT_BOUND: { code: "INTERNAL_SERVER_ERROR", message: "failed" },
        ALREADY_BOUND: { code: "INTERNAL_SERVER_ERROR", message: "failed" },
    };
    expect(received).toEqual(expected);
});

test("report an unreachable plan as a conflict naming every problem", () => {
    const unreachable = new PlanError([
        { target: "note", detail: "declare a conversion to version 2" },
        { target: "tag", detail: "declare a default for color" },
    ]);
    expect(unreachable.toServiceError()).toEqual({
        code: "CONFLICT",
        message: "note: declare a conversion to version 2; tag: declare a default for color",
    });
});
