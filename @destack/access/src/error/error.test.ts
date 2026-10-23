import { expect, test } from "@destack/test";
import { AccessError } from "./error.ts";

test("challenge a caller with the step-up an insufficient authentication names", () => {
    const refused = AccessError.of("INSUFFICIENT_AUTHENTICATION", "authenticate again", {
        data: { assurance: 3 },
    });
    expect(refused.toCallError()).toEqual({
        code: "UNAUTHORIZED",
        reason: "INSUFFICIENT_AUTHENTICATION",
        domain: "@destack/access",
        message: "authenticate again",
        data: { assurance: 3 },
    });
});
