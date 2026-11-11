import { expect, test } from "@destack/test";
import { AccessError } from "./error.ts";

test("challenge a caller with the step-up an insufficient authentication names", () => {
    const refused = AccessError.of(
        "ASSURANCE_UNAUTHENTICATED",
        "authentication required: authenticate again",
        {
            data: { assurance: 3 },
        },
    );
    expect(refused.toCallError()).toEqual({
        code: "UNAUTHORIZED",
        reason: "ASSURANCE_UNAUTHENTICATED",
        domain: "@destack/access",
        message: "authentication required: authenticate again",
        data: { assurance: 3 },
    });
});
