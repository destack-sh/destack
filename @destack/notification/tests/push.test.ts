import { nth } from "@destack/schema";
import { expect, test } from "@destack/test";
import { document } from "../src/test/notes.ts";
import { TestNotification } from "../src/test/notification.ts";

test("push a mention to each of a recipient's phones, and forget the subscription its push service forgot through the account package", async () => {
    await using fixture = await TestNotification.open();

    // give bob two phones, the old one forgotten by its push service
    const old = await fixture.browser("bob", "/bob-old");
    const phone = await fixture.browser("bob", "/bob-new");
    fixture.pushes.gone.add(old.url);

    // mention bob on a document he views
    const plan = await fixture.call(
        "alice",
        document.call("create", { ...fixture.scope, title: "Launch plan" }),
    );
    await fixture.call(
        "alice",
        document.call("grant", {
            ...fixture.scope,
            id: plan.id,
            relation: "viewer",
            subject: fixture.subject("bob"),
        }),
    );
    await fixture.call(
        "alice",
        document.call("remark", {
            ...fixture.scope,
            id: plan.id,
            text: "@bob, a question",
            mentions: [fixture.subject("bob")],
        }),
    );
    await fixture.dispatch();

    // send one push to each phone, failing the old phone's as gone
    const mentioned = nth((await fixture.inbox("bob")).entries, 0);
    const messages = await fixture.messages("bob", mentioned.notification.id);
    expect({
        pushed: fixture.pushes.requests.map((request) => request.url).toSorted(),
        messages: messages
            .flatMap((row) =>
                row.to.channel === "push" ? [[row.to.client, row.status, row.reason]] : [],
            )
            .toSorted((left, right) => String(left[0]).localeCompare(String(right[0]))),
    }).toEqual({
        pushed: [phone.url, old.url],
        messages: [
            [old.id, "failed", "RECIPIENT_GONE"],
            [phone.id, "sent", null],
        ].toSorted((left, right) => String(left[0]).localeCompare(String(right[0]))),
    });
});
