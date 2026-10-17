import { TEST_DIALECTS } from "@destack/db/test";
import { found } from "@destack/schema";
import { expect, test } from "@destack/test";
import { notification, subscription } from "../src/index.ts";
import { change, document } from "./fixture/document.ts";
import { actors, documentParent, serveSpace } from "./fixture/space.ts";

test.for(TEST_DIALECTS)(
    "collect a recipient's summarized notifications into one email at the next summary time, dropping those read, on %s",
    async (dialect) => {
        const fixture = await serveSpace(dialect);

        // let erin in Vienna, where it is noon, take changes in the summary going out at 08:00 and 18:00 by email
        await fixture.contact("erin", { email: "erin@example.com", timeZone: "Europe/Vienna" });
        await fixture.set("erin", change.preference, {
            channels: ["desktop", "push", "email"],
            delivery: "summary",
        });

        // subscribe erin to two documents bob edits
        const plan = await fixture.call(
            document.call("create", { ...fixture.scope, title: "Launch plan" }),
        );
        const draft = await fixture.call(
            document.call("create", { ...fixture.scope, title: "Draft" }),
        );
        for (const edited of [plan, draft]) {
            fixture.as("alice");
            await fixture.call(
                document.call("grant", {
                    ...fixture.scope,
                    id: edited.id,
                    relation: "viewer",
                    subject: actors.erin,
                }),
            );
            await fixture.call(
                document.call("grant", {
                    ...fixture.scope,
                    id: edited.id,
                    relation: "editor",
                    subject: actors.bob,
                }),
            );
            fixture.as("erin");
            await fixture.call(
                subscription.call("create", { ...fixture.scope, ...documentParent(edited.id) }),
            );
        }

        // change the plan twice and the draft once and defer each email to the summary at 16:00 UTC
        fixture.as("bob");
        await fixture.call(
            document.call("edit", { ...fixture.scope, id: plan.id, summary: "Rewrote the intro" }),
        );
        await fixture.call(
            document.call("edit", { ...fixture.scope, id: plan.id, summary: "Added a timeline" }),
        );
        await fixture.call(
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
        fixture.as("erin");
        await fixture.call(
            notification.call("read", { ...(await fixture.homeScope()), id: drafted }),
        );
        await fixture.wait((evening - fixture.clock.now()) / 60_000);
        await fixture.dispatch();
        const settled = async (id: string) =>
            (await fixture.messages("erin", id)).map((row) => [row.status, row.reason]);
        expect([
            fixture.mails.sent.map((mail) => [mail.to, mail.subject, mail.text]),
            await settled(planned),
            await settled(drafted),
        ]).toEqual([
            [["erin@example.com", "Scheduled summary", "2 changes"]],
            [["sent", null]],
            [],
        ]);
    },
);

test.for(TEST_DIALECTS)(
    "send a recipient's email and scheduled summary with its lines in their German, with the catalogs of the declaring package and the inbox, on %s",
    async (dialect) => {
        const fixture = await serveSpace(dialect);

        // let erin read Austrian German and receive email, subscribed to a document bob edits
        await fixture.contact("erin", { email: "erin@example.com", locale: "de-AT" });
        const plan = await fixture.call(
            document.call("create", { ...fixture.scope, title: "Launch plan" }),
        );
        await fixture.call(
            document.call("grant", {
                ...fixture.scope,
                id: plan.id,
                relation: "viewer",
                subject: actors.erin,
            }),
        );
        await fixture.call(
            document.call("grant", {
                ...fixture.scope,
                id: plan.id,
                relation: "editor",
                subject: actors.bob,
            }),
        );
        fixture.as("erin");
        await fixture.call(
            subscription.call("create", { ...fixture.scope, ...documentParent(plan.id) }),
        );

        // change the plan, emailing erin once the email delay passes
        fixture.as("bob");
        await fixture.call(
            document.call("edit", { ...fixture.scope, id: plan.id, summary: "Rewrote the intro" }),
        );
        await fixture.dispatch();
        await fixture.wait(16);
        await fixture.dispatch();

        // take later changes in the summary, and change the plan again for the summary at 18:00 UTC
        await fixture.set("erin", change.preference, {
            channels: ["desktop", "push", "email"],
            delivery: "summary",
        });
        await fixture.call(
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
    },
);
