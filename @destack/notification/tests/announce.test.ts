import { TEST_DIALECTS } from "@destack/db/test";
import { Subject } from "@destack/sync";
import { expect, test } from "@destack/test";
import { subscription } from "../src/index.ts";
import { document } from "./fixture/document.ts";
import { actors, documentParent, serveSpace } from "./fixture/space.ts";

test.for(TEST_DIALECTS)(
    "expand an announcement to every subscriber, or every member, in batches, never to its author, on %s",
    async (dialect) => {
        const fixture = await serveSpace(dialect, {
            batch: 2,
        });
        const states = async () =>
            (await fixture.notifications())
                .map((row) => [row.recipient, row.reason, row.count])
                .toSorted((left, right) => String(left[0]).localeCompare(String(right[0])));

        // subscribe the author and four viewers to a document
        const plan = await fixture.call(document, "create", { title: "Launch plan" });
        await fixture.call(subscription, "create", documentParent(plan.id));
        for (const viewer of ["bob", "carol", "dave", "erin"] as const) {
            fixture.as("alice");
            await fixture.call(document, "grant", {
                id: plan.id,
                relation: "viewer",
                subject: actors[viewer],
            });
            fixture.as(viewer);
            await fixture.call(subscription, "create", documentParent(plan.id));
        }

        // announce the document to its subscribers as one announcement, expanded two at a time
        fixture.as("alice");
        await fixture.call(document, "broadcast", { id: plan.id, audience: "subscribers" });
        await fixture.dispatch();
        const subscribed = await states();

        // announce it to the space's members, collapsing into their unread notifications
        await fixture.join("alice");
        await fixture.join("bob");
        await fixture.join("carol");
        await fixture.call(document, "broadcast", { id: plan.id, audience: "members" });
        await fixture.dispatch();
        const members = await states();

        // announce it to the users with edit, found through their grants, in batches of two
        for (const editor of ["dave", "erin"] as const) {
            await fixture.call(document, "grant", {
                id: plan.id,
                relation: "editor",
                subject: actors[editor],
            });
        }
        await fixture.call(document, "broadcast", { id: plan.id, audience: "editors" });
        await fixture.dispatch();
        const editors = await states();
        expect([subscribed, members, editors]).toEqual([
            [
                state("bob", "subscribed", 1),
                state("carol", "subscribed", 1),
                state("dave", "subscribed", 1),
                state("erin", "subscribed", 1),
            ],
            [
                state("bob", "mention", 2),
                state("carol", "mention", 2),
                state("dave", "subscribed", 1),
                state("erin", "subscribed", 1),
            ],
            [
                state("bob", "mention", 2),
                state("carol", "mention", 2),
                state("dave", "mention", 2),
                state("erin", "mention", 2),
            ],
        ]);
    },
);

test.for(TEST_DIALECTS)(
    "notify each subscriber for the reason it subscribed, skipping the principals an announcement excludes, on %s",
    async (dialect) => {
        const fixture = await serveSpace(dialect, { batch: 2 });

        // subscribe bob by a mention, carol and dave by hand
        const plan = await fixture.call(document, "create", { title: "Launch plan" });
        for (const viewer of ["bob", "carol", "dave"] as const) {
            await fixture.call(document, "grant", {
                id: plan.id,
                relation: "viewer",
                subject: actors[viewer],
            });
        }
        await fixture.call(document, "remark", {
            id: plan.id,
            text: "@bob, a look?",
            mentions: [actors.bob],
        });
        for (const viewer of ["carol", "dave"] as const) {
            fixture.as(viewer);
            await fixture.call(subscription, "create", documentParent(plan.id));
        }

        // announce to the subscribers but dave, as a call that told dave already
        fixture.as("alice");
        await fixture.call(document, "broadcast", {
            id: plan.id,
            audience: "subscribers",
            excluded: [actors.dave],
        });
        await fixture.dispatch();
        const announced = (await fixture.notifications()).filter((row) => row.name === "change");
        expect(announced.map((row) => [row.recipient, row.reason])).toEqual(
            (
                [
                    [Subject.key(actors.bob), "mention"],
                    [Subject.key(actors.carol), "subscribed"],
                ] satisfies [string, string][]
            ).toSorted((left, right) => left[0].localeCompare(right[0])),
        );
    },
);

/** Read an actor's notification as its recipient key, reason and count. */
function state(actor: keyof typeof actors, reason: string, count: number) {
    return [Subject.key(actors[actor]), reason, count];
}
