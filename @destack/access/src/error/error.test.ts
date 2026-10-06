import { expect, test } from "@destack/test";
import { AccessError } from "./error.ts";

test("challenge a caller with the step-up an insufficient authentication names", () => {
    const refused = new AccessError("INSUFFICIENT_AUTHENTICATION", "authenticate again", {
        stepUp: { assurance: 3 },
    });
    expect(refused.toServiceError()).toEqual({
        code: "INSUFFICIENT_AUTHENTICATION",
        message: "authenticate again",
        data: { assurance: 3 },
    });
});
