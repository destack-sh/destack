import { ObjectReference } from "@destack/sync";
import { expect, test } from "@destack/test";
import { Activity } from "../src/index.ts";
import { document } from "../src/test/notes.ts";
import { TestNotification } from "../src/test/notification.ts";

test("post a mention as the reader whose remark notifies, with the author's profile name", async () => {
    // share a document with bob, who mentions alice and himself in a remark
    await using fixture = await TestNotification.open();
    const { scope } = fixture;
    const plan = await fixture.call(
        "alice",
        document.call("create", { ...scope, title: "Launch plan" }),
    );
    await fixture.call(
        "alice",
        document.call("grant", {
            ...scope,
            id: plan.id,
            relation: "viewer",
            subject: fixture.subject("bob"),
        }),
    );
    await fixture.call(
        "bob",
        document.call("remark", {
            ...scope,
            id: plan.id,
            text: "@alice, a question",
            mentions: [fixture.subject("alice"), fixture.subject("bob")],
        }),
    );

    // record one activity for alice alone, posted by bob
    expect(
        (await fixture.activities()).map((row) => [
            Activity.recipient(row),
            ObjectReference.at("actor", row),
            row.name,
            row.reason,
            row.payload,
            row.count,
        ]),
    ).toEqual([
        [
            fixture.subject("alice"),
            fixture.subject("bob"),
            "mention",
            "mention",
            { author: "bob", excerpt: "@alice, a question" },
            1,
        ],
    ]);
});
