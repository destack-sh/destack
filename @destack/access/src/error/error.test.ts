import { expect, test } from "@destack/test";
import { AccessError } from "./error.ts";

test("challenge a caller with the step-up an insufficient authentication names", () => {
    const refused = AccessError.of("AUTHENTICATION_INSUFFICIENT", "authenticate again", {
        data: { assurance: 3 },
    });
    expect(refused.toCallError()).toEqual({
        code: "UNAUTHORIZED",
        reason: "AUTHENTICATION_INSUFFICIENT",
        domain: "@destack/access",
        message: "authenticate again",
        data: { assurance: 3 },
    });
});
