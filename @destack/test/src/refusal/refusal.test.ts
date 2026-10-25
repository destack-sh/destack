import { expect, test } from "vitest";
import { refusal } from "./refusal.ts";

/** An error with a code and a reason, as call errors are. */
class ReasonedFailure extends Error {
    /** The failure's code. */
    readonly code = "NOT_FOUND";
    /** The failure's reason within its domain. */
    readonly reason = "NOTE_NOT_FOUND";
    /** The package whose reason it is. */
    readonly domain = "@destack/note";
}

/** A package's failure, which knows the call error its caller receives. */
class NoteError extends Error {
    /** The note's callers, who receive the failure. */
    readonly audience = "caller";

    /** Convert the failure to the call error a caller receives. */
    toCallError() {
        return {
            code: "CONFLICT",
            reason: "NOTE_TAKEN",
            domain: "@destack/note",
            message: this.message,
        };
    }
}

test("answer done for a call that succeeds", async () => {
    expect(await refusal(Promise.resolve(1))).toBe("done");
});

test("answer a refusal's code, reason and message, a package error's as its caller receives it, a thrown one too", async () => {
    expect([
        await refusal(Promise.reject(new ReasonedFailure("no note"))),
        await refusal(Promise.reject(new NoteError("note taken"))),
        await refusal(() => {
            throw new NoteError("note taken");
        }),
    ]).toEqual([
        ["NOT_FOUND", "NOTE_NOT_FOUND", "no note"],
        ["CONFLICT", "NOTE_TAKEN", "note taken"],
        ["CONFLICT", "NOTE_TAKEN", "note taken"],
    ]);
});

test("rethrow a failure without a code and reason, failing the test that awaits it", async () => {
    const failure = new Error("broken");

    await expect(refusal(Promise.reject(failure))).rejects.toBe(failure);
});
