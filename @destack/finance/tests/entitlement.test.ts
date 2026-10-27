import type { Environment } from "@destack/environment";
import { testSimulation } from "@destack/object/test";
import { eq, Snapshot } from "@destack/db";
import { nth } from "@destack/schema";
import { expect, onTestFinished, test } from "@destack/test";
import { Price, subscription, type Subscription, subscriptionItem } from "../src/object/index.ts";
import { spaces } from "../src/test/index.ts";
import { CALL_LIMIT, Finance, ids, resolved, STORAGE_LIMIT, SEATS } from "./fixture/finance.ts";
import type { Entitlement } from "@destack/account/object";
import {
    calls,
    storageLimit,
    requests,
    seats,
    storage,
    stored,
    support,
    sync,
} from "./fixture/storage.ts";
import { RequestId } from "@destack/sync";

/** The start of the subscription's first billing period. */
const START = Date.UTC(2026, 9, 1);

/** The start of the add-on's billing period, within the plan's first. */
const MIDDLE = Date.UTC(2026, 9, 15);

/** The API calls the add-on grants each period. */
const ADDON_CALLS = 500;

/** The seats the add-on grants, more than the plan's. */
const ADDON_SEATS = 25;

/** The end of the first billing period, where the second starts. */
const END = Date.UTC(2026, 10, 1);

/** The end of the second billing period. */
const NEXT = Date.UTC(2026, 11, 1);

/** Read the derived fields of entitlements, without their bookkeeping. */
function derived(rows: readonly Entitlement[]) {
    return rows.map((row) => [row.feature, row.kind, row.value, row.limit, row.resetAt]);
}

/** Open the service with Alice's customer for Acme's buyer account and Carol's offer. */
async function open() {
    const finance = await Finance.open();
    onTestFinished(() => finance.close());
    const offer = await finance.offer();
    await finance.customer();

    return { finance, offer };
}

/** Route uses to the buyer's account as a cloud machine would, by identity, and rate them. */
async function append(
    finance: Finance,
    events: Readonly<Record<string, { meter: string; value: number; time: number }>>,
) {
    await finance.fixture.use(
        ids.buyer,
        Object.entries(events).map(([id, use]) => ({
            id,
            space: spaces.notes,
            meter: { packageId: storage.id, name: use.meter },
            quantity: use.value,
            time: use.time,
        })),
    );
}

/** Move a subscription through a provider's transition as the system. */
async function transition(
    world: Environment,
    finance: Finance,
    name: "activate" | "fail" | "cancel",
    row: Subscription,
): Promise<Subscription> {
    return nth(
        await finance.store.mutate(finance.store.as(subscription), {
            id: RequestId.create(world),
            calls: [subscription.call(name, subscription.target(row))],
        }),
        0,
    );
}

test("derive entitlements from an active or past due subscription's grants and remove them once it is canceled", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    const { finance, offer } = await open();

    // subscribe the buyer to the monthly price, granting nothing while incomplete
    const incomplete = await finance.subscribe(offer, { start: START, end: END });
    await finance.entitle();
    expect(await finance.entitlements(ids.buyer)).toEqual([]);

    // grant each feature of the plan while active, and still while its payment is past due
    const due = await transition(
        world,
        finance,
        "fail",
        await transition(world, finance, "activate", incomplete),
    );
    await finance.entitle();
    expect([due.status, derived(await finance.entitlements(ids.buyer))]).toEqual([
        "past_due",
        [
            ["api.calls", "metered", null, CALL_LIMIT, END],
            ["seats", "static", SEATS, null, null],
            ["storage.limit", "metered", null, STORAGE_LIMIT, null],
            ["support", "static", "priority", null, null],
            ["sync", "boolean", null, null, null],
        ],
    ]);

    // remove them once the subscription is canceled
    await transition(world, finance, "cancel", due);
    await finance.entitle();
    expect(await finance.entitlements(ids.buyer)).toEqual([]);
});

test("count metered usage within the subscription's period and start again with the next period", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    const { finance, offer } = await open();
    const active = await transition(
        world,
        finance,
        "activate",
        await finance.subscribe(offer, { start: START, end: END }),
    );

    // sum the requests within the period and take the latest stored size
    await append(finance, {
        before: { meter: requests.name, value: 5, time: START - 1 },
        first: { meter: requests.name, value: 10, time: START },
        second: { meter: requests.name, value: 20, time: END - 1 },
        small: { meter: stored.name, value: 100, time: START - 1 },
        large: { meter: stored.name, value: 300, time: START },
    });
    await finance.entitle();
    const usage = async () =>
        (await finance.resolutions(ids.buyer)).flatMap((resolution) =>
            resolution.kind === "metered"
                ? [[resolution.feature, resolution.usage, resolution.resetAt]]
                : [],
        );
    expect(await usage()).toEqual([
        ["api.calls", 30, END],
        ["storage.limit", 300, null],
    ]);

    // start the requests again once the provider renews the subscription into the next period
    await finance.store.mutate(finance.store.as(subscription), {
        id: RequestId.create(world),
        calls: [
            subscription.call("sync", {
                ...subscription.target(active),
                providerId: active.providerId,
                customer: "cus_buyer",
                status: "active",
                currentPeriodStart: END,
                currentPeriodEnd: NEXT,
                cancelAtPeriodEnd: false,
                canceledAt: null,
                trialEnd: null,
                items: [],
            }),
        ],
    });
    await append(finance, { third: { meter: requests.name, value: 7, time: END } });
    await finance.entitle();
    expect(await usage()).toEqual([
        ["api.calls", 7, NEXT],
        ["storage.limit", 300, null],
    ]);
});

test("resolve each feature's effective grant across a subscription's plan and add-on items", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    const { finance, offer } = await open();

    // subscribe to the plan with an add-on item of more calls, unlimited storage, more seats and community support
    const subscribed = await finance.subscribe(offer, { start: START, end: END });
    const addon = [
        { packageId: storage.id, feature: calls.name, value: ADDON_CALLS },
        { packageId: storage.id, feature: storageLimit.name, value: null },
        { packageId: storage.id, feature: seats.name, value: ADDON_SEATS },
        { packageId: storage.id, feature: support.name, value: "community" },
    ];
    await finance.item(subscribed, offer.monthly, addon);
    await transition(world, finance, "activate", subscribed);

    // count requests in the period and the one after
    await append(finance, {
        first: { meter: requests.name, value: 10, time: START },
        middle: { meter: requests.name, value: 20, time: MIDDLE },
        last: { meter: requests.name, value: 40, time: END },
    });
    await finance.entitle();

    // grant access, the most seats, the latest support, summed calls in the first period, and unlimited storage
    const rows = await finance.resolutions(ids.buyer);
    expect(
        [sync, seats, support, calls, storageLimit].map((feature) =>
            resolved(rows, feature.reference),
        ),
    ).toEqual([
        { kind: "boolean" },
        { kind: "static", value: ADDON_SEATS },
        { kind: "static", value: "community" },
        {
            kind: "metered",
            limit: CALL_LIMIT + ADDON_CALLS,
            usage: 30,
            resetAt: END,
            state: "within",
        },
        { kind: "metered", limit: null, usage: 0, resetAt: null, state: "within" },
    ]);
});

test("add the limits of a subscription's items granting one feature twice into one entitlement", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    const { finance, offer } = await open();

    // bill the plan and an add-on of more calls in one subscription
    const active = await transition(
        world,
        finance,
        "activate",
        await finance.subscribe(offer, { start: START, end: END }),
    );
    const addon = [{ packageId: storage.id, feature: calls.name, value: ADDON_CALLS }];
    await finance.item(active, offer.monthly, addon);
    await finance.entitle();

    // store one row for the feature with the summed limit
    const rows = await finance.resolutions(ids.buyer);
    expect([
        rows.filter((row) => row.feature === calls.name).length,
        resolved(rows, calls.reference),
    ]).toEqual([
        1,
        {
            kind: "metered",
            limit: CALL_LIMIT + ADDON_CALLS,
            usage: 0,
            resetAt: END,
            state: "within",
        },
    ]);
});

test("count a meter event into the usage of its meter's entitlements, deriving the account's other features with it", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    const { finance, offer } = await open();
    const active = await transition(
        world,
        finance,
        "activate",
        await finance.subscribe(offer, { start: START, end: END }),
    );
    await finance.entitle();

    // change the item's support grant behind the controller and append requests in and after the period
    const offered = await Price.offer(offer.monthly, Snapshot.live(finance.database));
    await finance.database.transaction(
        async (transaction) =>
            await transaction
                .update(subscriptionItem.table)
                .set({
                    grants: offered.grants.map((grant) =>
                        grant.feature === support.name ? { ...grant, value: "community" } : grant,
                    ),
                })
                .where(eq(subscriptionItem.table.parentId, active.id)),
    );
    await append(finance, {
        first: { meter: requests.name, value: 10, time: START },
        later: { meter: requests.name, value: 50, time: END },
    });

    // count the requests within the period, and take the changed support tier with them
    const rows = await finance.resolutions(ids.buyer);
    expect([resolved(rows, calls.reference), resolved(rows, support.reference)]).toEqual([
        { kind: "metered", limit: CALL_LIMIT, usage: 10, resetAt: END, state: "within" },
        { kind: "static", value: "community" },
    ]);
});

test("let a grant lapse once the latest release of its feature's package no longer declares the feature", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    const { finance, offer } = await open();

    // grant the plan's features to an active subscription
    await transition(
        world,
        finance,
        "activate",
        await finance.subscribe(offer, { start: START, end: END }),
    );
    await finance.entitle();
    const features = async () =>
        derived(await finance.entitlements(ids.buyer)).map(([feature]) => feature);
    expect(await features()).toEqual(["api.calls", "seats", "storage.limit", "support", "sync"]);

    // drop the support entitlement once a later release of storage stops declaring it
    await finance.publish(storage, "2026.10.0", [
        requests,
        stored,
        sync,
        seats,
        calls,
        storageLimit,
    ]);
    await finance.entitle();
    expect(await features()).toEqual(["api.calls", "seats", "storage.limit", "sync"]);
});
