import { found } from "@destack/schema";
import { expect, test } from "@destack/test";
import { notification, Preference, preferences, subscription } from "../src/index.ts";
import { change, document, documentParent } from "../src/test/notes.ts";
import { TestNotification } from "../src/test/notification.ts";

test("collect a recipient's summarized notifications into one email at the next summary time, dropping those read", async () => {
    await using fixture = await TestNotification.open();

    // let erin in Vienna, where it is noon, take changes in the summary going out at 08:00 and 18:00 by email
    await fixture.contact("erin", { timeZone: "Europe/Vienna" });
    await fixture.set("erin", preferences, {
        [Preference.key(change.reference)]: {
            channels: ["desktop", "push", "email"],
            delivery: "summary",
        },
    });

    // subscribe erin to two documents bob edits
    const plan = await fixture.call(
        "alice",
        document.call("create", { ...fixture.scope, title: "Launch plan" }),
    );
    const draft = await fixture.call(
        "alice",
        document.call("create", { ...fixture.scope, title: "Draft" }),
    );
    for (const edited of [plan, draft]) {
        await fixture.call(
            "alice",
            document.call("grant", {
                ...fixture.scope,
                id: edited.id,
                relation: "viewer",
                subject: fixture.subject("erin"),
            }),
        );
        await fixture.call(
            "alice",
            document.call("grant", {
                ...fixture.scope,
                id: edited.id,
                relation: "editor",
                subject: fixture.subject("bob"),
            }),
        );
        await fixture.call(
            "erin",
            subscription.call("create", { ...fixture.scope, ...documentParent(edited.id) }),
        );
    }

    // change the plan twice and the draft once and defer each email to the summary at 16:00 UTC
    await fixture.call(
        "bob",
        document.call("edit", { ...fixture.scope, id: plan.id, summary: "Rewrote the intro" }),
    );
    await fixture.call(
        "bob",
        document.call("edit", { ...fixture.scope, id: plan.id, summary: "Added a timeline" }),
    );
    await fixture.call(
        "bob",
        document.call("edit", { ...fixture.scope, id: draft.id, summary: "Fixed a typo" }),
    );
    await fixture.dispatch();
    const threads = new Map(
        (await fixture.inbox("erin")).entries.map((entry) => [
            entry.activity.thread,
            entry.notification.id,
        ]),
    );
    const planned = found(threads, plan.id);
    const drafted = found(threads, draft.id);
    const evening = Date.UTC(2026, 8, 28, 16, 0);
    const read = async (id: string) =>
        (await fixture.messages("erin", id)).map((row) => [row.to.channel, row.status]);
    expect([await read(planned), await read(drafted)]).toEqual([[], []]);

    // read the draft's change, then send one summary of the rest at 18:00 in Vienna
    await fixture.call(
        "erin",
        notification.call("read", { ...fixture.homeScope("erin"), id: drafted }),
    );
    await fixture.wait((evening - fixture.clock.now()) / 60_000);
    await fixture.dispatch();
    const settled = async (id: string) =>
        (await fixture.messages("erin", id)).map((row) => [row.status, row.reason]);
    expect([
        fixture.mails.sent.map((mail) => [mail.to, mail.subject, mail.text]),
        await settled(planned),
        await settled(drafted),
    ]).toEqual([[["erin@example.test", "Scheduled summary", "2 changes"]], [["sent", null]], []]);
});

test("send a recipient's email and scheduled summary with its lines in their German, with the catalogs of the declaring package and the inbox", async () => {
    await using fixture = await TestNotification.open();

    // let erin read Austrian German and receive email, her home running, subscribed to a document bob edits
    await fixture.home("erin");
    await fixture.contact("erin", { locale: "de-AT" });
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
            subject: fixture.subject("erin"),
        }),
    );
    await fixture.call(
        "alice",
        document.call("grant", {
            ...fixture.scope,
            id: plan.id,
            relation: "editor",
            subject: fixture.subject("bob"),
        }),
    );
    await fixture.call(
        "erin",
        subscription.call("create", { ...fixture.scope, ...documentParent(plan.id) }),
    );

    // change the plan, emailing erin once the email delay passes
    await fixture.call(
        "bob",
        document.call("edit", { ...fixture.scope, id: plan.id, summary: "Rewrote the intro" }),
    );
    await fixture.dispatch();
    await fixture.wait(16);

    // take later changes in the summary, and change the plan again for the summary at 18:00 UTC
    await fixture.set("erin", preferences, {
        [Preference.key(change.reference)]: {
            channels: ["desktop", "push", "email"],
            delivery: "summary",
        },
    });
    await fixture.call(
        "bob",
        document.call("edit", { ...fixture.scope, id: plan.id, summary: "Added a timeline" }),
    );
    await fixture.dispatch();
    await fixture.wait((Date.UTC(2026, 8, 28, 18, 0) - fixture.clock.now()) / 60_000);
    await fixture.dispatch();
    expect([
        (await fixture.inbox("erin")).entries.length,
        fixture.mails.sent.map((mail) => [mail.subject, mail.text]),
    ]).toEqual([
        2,
        [
            ["Dokument geändert", "\u2068Rewrote the intro\u2069"],
            ["Geplante Zusammenfassung", "1 Änderung"],
        ],
    ]);
});
