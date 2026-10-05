import { expect, test } from "@destack/test";
import { ReportableError, type ServiceErrorReport } from "./report.ts";

/** A failure that a caller receives as a conflict. */
class TakenError extends Error implements ReportableError {
    /** Report the failure as a conflict. */
    toServiceError(): ServiceErrorReport {
        return { code: "CONFLICT", message: this.message };
    }
}

test("recognise an error with a service error and refuse plain errors and values", () => {
    expect([
        ReportableError.is(new TakenError("taken")),
        ReportableError.is(new Error("taken")),
        ReportableError.is({ toServiceError: () => ({ code: "CONFLICT", message: "taken" }) }),
    ]).toEqual([true, false, false]);
});
