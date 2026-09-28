import { expect, test } from "@destack/test";
import { PublishDefinition, requirePublish } from "./publish.ts";

/** Report the message a publish definition fails with, or none. */
function failure(publish: unknown): string | undefined {
    try {
        requirePublish(PublishDefinition.parse(publish));
    } catch (error) {
        return (error as Error).message;
    }

    return undefined;
}

test("require the default export condition last and every condition to load a published output", () => {
    const outputs = { server: { target: "server" }, browser: { target: "browser" } };

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
