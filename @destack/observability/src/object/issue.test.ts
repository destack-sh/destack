import { schema } from "@destack/schema";
import { expect, test } from "@destack/test";
import { Issue } from "./issue.ts";

test("reopen a resolved or ignored issue by its condition, and never an open one", () => {
    const recorded = { lastSeenAt: 2000, revisedAt: 300, afterAt: 200 };
    const after = schema
        .identifier("installation-revision")
        .parse("installation-revision-01996ab0-0000-7000-8000-000000000001");
    expect([
        Issue.reopens(resolved({ kind: "event" }), recorded),
        Issue.reopens({ ...resolved({ kind: "event" }), status: "unresolved" }, recorded),
        Issue.reopens({ ...resolved({ kind: "event" }), status: "ignored" }, recorded),
        Issue.reopens(resolved({ kind: "revision", after }), recorded),
        Issue.reopens(resolved({ kind: "revision", after }), {
            ...recorded,
            revisedAt: 100,
        }),
        Issue.reopens(resolved({ kind: "revision", after }), {
            ...recorded,
            revisedAt: undefined,
        }),
        Issue.reopens(resolved({ kind: "time", at: 2000 }), recorded),
        Issue.reopens(resolved({ kind: "time", at: 2001 }), recorded),
        Issue.reopens(resolved({ kind: "count", count: 5 }), recorded),
        Issue.reopens(resolved({ kind: "count", count: 6 }), recorded),
        Issue.reopens(resolved({ kind: "never" }), recorded),
    ]).toEqual([true, false, true, true, false, false, true, false, true, false, false]);
});

/** Build a resolved issue with a reopening condition, at five events by default. */
function resolved(until: Issue["until"], count = 5) {
    return { status: "resolved" as const, until, count };
}
