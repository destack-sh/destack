import { TEST_DIALECTS } from "@destack/db/test";
import { Subject } from "@destack/sync";
import { expect, test } from "@destack/test";
import { notification, subscription } from "../src/index.ts";
import { change } from "./fixture/document.ts";
import { actors, serveSpace } from "./fixture/space.ts";

test.for(TEST_DIALECTS)(
    "collect a recipient's summarized notifications into one email at the next summary time, dropping those read, on %s",
    async (dialect) => {
        const { call, as, homes, mails, dispatch, notifications, deliveries, wait, now, host } =
            await serveSpace(dialect);

        // let erin in Vienna, where it is noon, take changes in the summary going out at 08:00 and 18:00 by email
        homes.emails.set(Subject.key(actors.erin), "erin@example.com");
        homes.zones.set(Subject.key(actors.erin), "Europe/Vienna");
        homes.set("erin", change.preference, {
            channels: ["desktop", "push", "email"],
            delivery: "summary",
        });

        // subscribe erin to two documents bob edits
        const plan = await call("create", { title: "Launch plan" });
        const draft = await call("create", { title: "Draft" });
        for (const document of [plan, draft]) {
            as("alice");
            await call("grant", { id: document.id, relation: "viewer", subject: actors.erin });
            await call("grant", { id: document.id, relation: "editor", subject: actors.bob });
            as("erin");
            await call("create", host(document.id), subscription);
        }

        // change the plan twice and the draft once and defer each email to the summary at 16:00 UTC
        as("bob");
        await call("edit", { id: plan.id, summary: "Rewrote the intro" });
        await call("edit", { id: plan.id, summary: "Added a timeline" });
        await call("edit", { id: draft.id, summary: "Fixed a typo" });
        await dispatch();
        const [planned, drafted] = await notifications();
        const evening = Date.UTC(2026, 8, 28, 16, 0);
        expect([
            (await deliveries(planned!.id)).map((row) => [
                row.channel,
                row.state,
                row.dueAt,
                row.isSummarized,
            ]),
            (await deliveries(drafted!.id)).map((row) => [
                row.channel,
                row.state,
                row.dueAt,
                row.isSummarized,
            ]),
        ]).toEqual([[["email", "pending", evening, true]], [["email", "pending", evening, true]]]);

        // read the draft's change, then send one summary of the rest at 18:00 in Vienna
        as("erin");
        await call("read", { id: drafted!.id }, notification);
        wait((evening - now()) / 60_000);
        await dispatch();
        expect([
            mails.sent.map((mail) => [mail.to, mail.subject, mail.text]),
            (await deliveries(planned!.id)).map((row) => [row.state, row.reason]),
            (await deliveries(drafted!.id)).map((row) => [row.state, row.reason]),
        ]).toEqual([
            [["erin@example.com", "Scheduled summary", "2 changes"]],
            [["sent", null]],
            [["skipped", "read"]],
        ]);
    },
);
