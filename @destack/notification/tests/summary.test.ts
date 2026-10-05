import { TEST_DIALECTS } from "@destack/db/test";
import { aligned } from "@destack/schema";
import { expect, test } from "@destack/test";
import { notification, subscription } from "../src/index.ts";
import { change, document } from "./fixture/document.ts";
import { actors, documentParent, serveSpace } from "./fixture/space.ts";

test.for(TEST_DIALECTS)(
    "collect a recipient's summarized notifications into one email at the next summary time, dropping those read, on %s",
    async (dialect) => {
        const fixture = await serveSpace(dialect);

        // let erin in Vienna, where it is noon, take changes in the summary going out at 08:00 and 18:00 by email
        await fixture.reach("erin", { email: "erin@example.com", timeZone: "Europe/Vienna" });
        await fixture.set("erin", change.preference, {
            channels: ["desktop", "push", "email"],
            delivery: "summary",
        });

        // subscribe erin to two documents bob edits
        const plan = await fixture.call(document, "create", { title: "Launch plan" });
        const draft = await fixture.call(document, "create", { title: "Draft" });
        for (const edited of [plan, draft]) {
            fixture.as("alice");
            await fixture.call(document, "grant", {
                id: edited.id,
                relation: "viewer",
                subject: actors.erin,
            });
            await fixture.call(document, "grant", {
                id: edited.id,
                relation: "editor",
                subject: actors.bob,
            });
            fixture.as("erin");
            await fixture.call(subscription, "create", documentParent(edited.id));
        }

        // change the plan twice and the draft once and defer each email to the summary at 16:00 UTC
        fixture.as("bob");
        await fixture.call(document, "edit", { id: plan.id, summary: "Rewrote the intro" });
        await fixture.call(document, "edit", { id: plan.id, summary: "Added a timeline" });
        await fixture.call(document, "edit", { id: draft.id, summary: "Fixed a typo" });
        await fixture.dispatch();
        const planned = aligned(await fixture.notifications(), 0);
        const drafted = aligned(await fixture.notifications(), 1);
        const evening = Date.UTC(2026, 8, 28, 16, 0);
        expect([
            (await fixture.deliveries(planned.id)).map((row) => [
                row.channel,
                row.state,
                row.dueAt,
                row.isSummarized,
            ]),
            (await fixture.deliveries(drafted.id)).map((row) => [
                row.channel,
                row.state,
                row.dueAt,
                row.isSummarized,
            ]),
        ]).toEqual([[["email", "pending", evening, true]], [["email", "pending", evening, true]]]);

        // read the draft's change, then send one summary of the rest at 18:00 in Vienna
        fixture.as("erin");
        await fixture.call(notification, "read", { id: drafted.id });
        fixture.wait((evening - fixture.now()) / 60_000);
        await fixture.dispatch();
        expect([
            fixture.mails.sent.map((mail) => [mail.to, mail.subject, mail.text]),
            (await fixture.deliveries(planned.id)).map((row) => [row.state, row.reason]),
            (await fixture.deliveries(drafted.id)).map((row) => [row.state, row.reason]),
        ]).toEqual([
            [["erin@example.com", "Scheduled summary", "2 changes"]],
            [["sent", null]],
            [["skipped", "read"]],
        ]);
    },
);

test.for(TEST_DIALECTS)(
    "send a recipient's email and scheduled summary with its lines in their German, with the catalogs of the declaring package and the inbox, on %s",
    async (dialect) => {
        const fixture = await serveSpace(dialect);

        // let erin read Austrian German and receive email, subscribed to a document bob edits
        await fixture.reach("erin", { email: "erin@example.com", locale: "de-AT" });
        const plan = await fixture.call(document, "create", { title: "Launch plan" });
        await fixture.call(document, "grant", {
            id: plan.id,
            relation: "viewer",
            subject: actors.erin,
        });
        await fixture.call(document, "grant", {
            id: plan.id,
            relation: "editor",
            subject: actors.bob,
        });
        fixture.as("erin");
        await fixture.call(subscription, "create", documentParent(plan.id));

        // change the plan, emailing erin once the email delay passes
        fixture.as("bob");
        await fixture.call(document, "edit", { id: plan.id, summary: "Rewrote the intro" });
        await fixture.dispatch();
        fixture.wait(16);
        await fixture.dispatch();

        // take later changes in the summary, and change the plan again for the summary at 18:00 UTC
        await fixture.set("erin", change.preference, {
            channels: ["desktop", "push", "email"],
            delivery: "summary",
        });
        await fixture.call(document, "edit", { id: plan.id, summary: "Added a timeline" });
        await fixture.dispatch();
        fixture.wait((Date.UTC(2026, 8, 28, 18, 0) - fixture.now()) / 60_000);
        await fixture.dispatch();
        expect([
            (await fixture.notifications()).length,
            fixture.mails.sent.map((mail) => [mail.subject, mail.text]),
        ]).toEqual([
            2,
            [
                ["Dokument geändert", "Rewrote the intro"],
                ["Geplante Zusammenfassung", "1 Änderung"],
            ],
        ]);
    },
);
