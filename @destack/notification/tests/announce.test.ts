import { TEST_DIALECTS } from "@destack/db/test";
import { Subject } from "@destack/sync";
import { expect, test } from "@destack/test";
import { subscription } from "../src/index.ts";
import { actors, serveSpace } from "./fixture/space.ts";

test.for(TEST_DIALECTS)(
    "expand an announcement to every subscriber, or every member, in batches, never to its author, on %s",
    async (dialect) => {
        const { call, as, dispatch, notifications, host, join } = await serveSpace(dialect, {
            batch: 2,
        });
        const states = async () =>
            (await notifications())
                .map((row) => [row.recipient, row.reason, row.count])
                .toSorted((left, right) => String(left[0]).localeCompare(String(right[0])));

        // subscribe the author and four viewers to a document
        const plan = await call("create", { title: "Launch plan" });
        await call("create", host(plan.id), subscription);
        for (const viewer of ["bob", "carol", "dave", "erin"] as const) {
            as("alice");
            await call("grant", { id: plan.id, relation: "viewer", subject: actors[viewer] });
            as(viewer);
            await call("create", host(plan.id), subscription);
        }

        // announce the document to its subscribers as one announcement, expanded two at a time
        as("alice");
        await call("broadcast", { id: plan.id, audience: "subscribers" });
        await dispatch();
        const subscribed = await states();

        // announce it to the space's members, collapsing into their unread notifications
        await join("alice");
        await join("bob");
        await join("carol");
        await call("broadcast", { id: plan.id, audience: "members" });
        await dispatch();
        const members = await states();

        // announce it to the users with edit, found through their grants, in batches of two
        for (const editor of ["dave", "erin"] as const) {
            await call("grant", { id: plan.id, relation: "editor", subject: actors[editor] });
        }
        await call("broadcast", { id: plan.id, audience: "editors" });
        await dispatch();
        const editors = await states();
        const state = (actor: keyof typeof actors, reason: string, count: number) => [
            Subject.key(actors[actor]),
            reason,
            count,
        ];
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
        const { call, as, dispatch, notifications, host } = await serveSpace(dialect, { batch: 2 });

        // subscribe bob by a mention, carol and dave by hand
        const plan = await call("create", { title: "Launch plan" });
        for (const viewer of ["bob", "carol", "dave"] as const) {
            await call("grant", { id: plan.id, relation: "viewer", subject: actors[viewer] });
        }
        await call("remark", { id: plan.id, text: "@bob, a look?", mentions: [actors.bob] });
        for (const viewer of ["carol", "dave"] as const) {
            as(viewer);
            await call("create", host(plan.id), subscription);
        }

        // announce to the subscribers but dave, as a call that told dave already
        as("alice");
        await call("broadcast", { id: plan.id, audience: "subscribers", excluded: [actors.dave] });
        await dispatch();
        const announced = (await notifications()).filter((row) => row.name === "change");
        expect(announced.map((row) => [row.recipient, row.reason])).toEqual(
            [
                [Subject.key(actors.bob), "mention"],
                [Subject.key(actors.carol), "subscribed"],
            ].toSorted((left, right) => left[0]!.localeCompare(right[0]!)),
        );
    },
);
