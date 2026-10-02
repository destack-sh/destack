import { expect, test } from "@destack/test";
import { Publication } from "./publication.ts";

/** Report the message a publication fails with, or none. */
function failure(publish: unknown): string | undefined {
    try {
        Publication.require(Publication.schema.parse(publish));
    } catch (error) {
        if (!(error instanceof Error)) {
            throw error;
        }

        return error.message;
    }

    return undefined;
}

test("require the default export condition last and every condition to load a published output", () => {
    const outputs = { server: { runtime: "bun" }, browser: { runtime: "browser" } };

    // accept default last, and refuse default elsewhere, no default and an undeclared output
    expect([
        failure({ outputs, conditions: { browser: "browser", default: "server" } }),
        failure({ outputs, conditions: { default: "server", browser: "browser" } }),
        failure({ outputs, conditions: { node: "server" } }),
        failure({ outputs, conditions: {} }),
        failure({ outputs, conditions: { worker: "worker", default: "server" } }),
    ]).toEqual([
        undefined,
        "the default export condition must be last",
        "the default export condition must be last",
        "the default export condition must be last",
        "export condition worker loads undeclared output worker",
    ]);
});
