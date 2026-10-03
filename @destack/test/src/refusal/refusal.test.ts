import { schema } from "@destack/schema";
import { expect, test } from "vitest";
import { refusal } from "./refusal.ts";

/** An error carrying a code, as service errors do. */
class CodedFailure extends Error {
    /** The failure's code. */
    readonly code = "NOT_FOUND";
}

test("answer done for a call that succeeds", async () => {
    expect(await refusal(Promise.resolve(1))).toBe("done");
});

test("answer a refusal's code and message", async () => {
    expect(await refusal(Promise.reject(new CodedFailure("no note")))).toEqual([
        "NOT_FOUND",
        "no note",
    ]);
});

test("reject a failure without a code, failing the test that awaits it", async () => {
    await expect(refusal(Promise.reject(new Error("broken")))).rejects.toBeInstanceOf(schema.Error);
});
