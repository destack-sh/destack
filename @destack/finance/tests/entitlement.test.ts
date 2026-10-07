import { eq } from "@destack/db";
import { TEST_DIALECTS } from "@destack/db/test";
import { SystemCall } from "@destack/object";
import { aligned } from "@destack/schema";
import { expect, onTestFinished, test } from "@destack/test";
import {
    Entitlement,
    Price,
    subscription,
    type Subscription,
    subscriptionItem,
} from "../src/object/index.ts";
import { CALL_LIMIT, Finance, ids, STORAGE_LIMIT, SEATS } from "./fixture/finance.ts";
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

/** The CloudEvents source of the fixture's meter events. */
const SOURCE = "/cells/eu-1";

/** Read the derived fields of entitlements, without their bookkeeping. */
function derived(rows: readonly Entitlement[]) {
    return rows.map((row) => [
        row.feature,
        row.kind,
        row.value,
        row.limit,
        row.usage,
        row.resetAt,
        row.source.type,
    ]);
}

/** Open the service with Alice's customer for Acme's buyer account and Carol's offer. */
async function open(dialect: (typeof TEST_DIALECTS)[number]) {
    const finance = await Finance.open(dialect);
    onTestFinished(() => finance.close());
    const offer = await finance.offer();
    await finance.customer();

    return { finance, offer };
}

/** Route uses to the buyer's account as a cell would, by identity, and rate them. */
async function append(
    finance: Finance,
    events: Readonly<Record<string, { meter: string; value: number; time: number }>>,
) {
    await finance.fixture.use(
        ids.buyer,
        Object.entries(events).map(([id, use]) => ({
            id,
            source: SOURCE,
            meter: { packageId: storage.id, name: use.meter },
            quantity: use.value,
            time: use.time,
        })),
    );
}

/** Move a subscription through a provider's transition as the system. */
async function transition(
    finance: Finance,
    name: "activate" | "fail" | "cancel",
    row: Subscription,
): Promise<Subscription> {
    return aligned(
        await finance.server.executeAsSystem(subscription, name, [SystemCall.of(row)], Date.now()),
        0,
    );
}

test.each(TEST_DIALECTS)(
    "derive entitlements from an active or past due subscription's grants and remove them once it is canceled on %s",
    async (dialect) => {
        const { finance, offer } = await open(dialect);

        // subscribe the buyer to the monthly price, granting nothing while incomplete
        const incomplete = await finance.subscribe(offer, { start: START, end: END });
        await finance.entitle();
        expect(await finance.entitlements(ids.buyer)).toEqual([]);

        // grant each feature of the plan while active, and still while its payment is past due
        const due = await transition(
            finance,
            "fail",
            await transition(finance, "activate", incomplete),
        );
        await finance.entitle();
        expect([due.status, derived(await finance.entitlements(ids.buyer))]).toEqual([
            "past_due",
            [
                ["api.calls", "metered", null, CALL_LIMIT, 0, END, "subscription-item"],
                ["seats", "static", SEATS, null, null, null, "subscription-item"],
                ["storage.limit", "metered", null, STORAGE_LIMIT, 0, null, "subscription-item"],
                ["support", "static", "priority", null, null, null, "subscription-item"],
                ["sync", "boolean", null, null, null, null, "subscription-item"],
            ],
        ]);

        // remove them once the subscription is canceled
        await transition(finance, "cancel", due);
        await finance.entitle();
        expect(await finance.entitlements(ids.buyer)).toEqual([]);
    },
);

test.each(TEST_DIALECTS)(
    "count metered usage within the subscription's period and start again with the next period on %s",
    async (dialect) => {
        const { finance, offer } = await open(dialect);
        const active = await transition(
            finance,
            "activate",
            await finance.subscribe(offer, { start: START, end: END }),
        );

        // sum the requests within the period and keep the latest stored size
        await append(finance, {
            before: { meter: requests.name, value: 5, time: START - 1 },
            first: { meter: requests.name, value: 10, time: START },
            second: { meter: requests.name, value: 20, time: END - 1 },
            small: { meter: stored.name, value: 100, time: START - 1 },
            large: { meter: stored.name, value: 300, time: START + 1 },
        });
        await finance.entitle();
        const usage = async () =>
            (await finance.entitlements(ids.buyer)).flatMap((row) =>
                row.kind === "metered" ? [[row.feature, row.usage, row.resetAt]] : [],
            );
        expect(await usage()).toEqual([
            ["api.calls", 30, END],
            ["storage.limit", 300, null],
        ]);

        // start the requests again once the provider records the next period
        await finance.server.executeAsSystem(
            subscription,
            "record",
            [SystemCall.of(active, { currentPeriodStart: END, currentPeriodEnd: NEXT })],
            Date.now(),
        );
        await append(finance, { third: { meter: requests.name, value: 7, time: END } });
        await finance.entitle();
        expect(await usage()).toEqual([
            ["api.calls", 7, NEXT],
            ["storage.limit", 300, null],
        ]);
    },
);

test.each(TEST_DIALECTS)(
    "resolve each feature's effective grant across two subscriptions on %s",
    async (dialect) => {
        const { finance, offer } = await open(dialect);

        // subscribe to the plan and a fortnight later to an add-on of more calls, unlimited storage, more seats and community support
        await transition(
            finance,
            "activate",
            await finance.subscribe(offer, { start: START, end: END }),
        );
        const addon = [
            { packageId: storage.id, feature: calls.name, value: ADDON_CALLS },
            { packageId: storage.id, feature: storageLimit.name, value: null },
            { packageId: storage.id, feature: seats.name, value: ADDON_SEATS },
            { packageId: storage.id, feature: support.name, value: "community" },
        ];
        const later = await finance.subscribe(offer, { start: MIDDLE, end: NEXT }, addon);
        await transition(finance, "activate", later);

        // count requests in both overlapping periods
        await append(finance, {
            first: { meter: requests.name, value: 10, time: START },
            middle: { meter: requests.name, value: 20, time: MIDDLE },
            last: { meter: requests.name, value: 40, time: END },
        });
        await finance.entitle();

        // grant access, the most seats, the latest support, summed calls in the first period, and unlimited storage
        const rows = await finance.entitlements(ids.buyer);
        expect(
            [sync, seats, support, calls, storageLimit].map((feature) =>
                Entitlement.resolve(rows, feature.reference),
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
    },
);

test.each(TEST_DIALECTS)(
    "keep one entitlement per item of a subscription granting one feature twice, and add their limits on %s",
    async (dialect) => {
        const { finance, offer } = await open(dialect);

        // bill the plan and an add-on of more calls in one subscription
        const active = await transition(
            finance,
            "activate",
            await finance.subscribe(offer, { start: START, end: END }),
        );
        const addon = [{ packageId: storage.id, feature: calls.name, value: ADDON_CALLS }];
        const item = await finance.item(active, offer.monthly, addon);
        await finance.entitle();

        // keep a row per item, and resolve their summed limit
        const rows = await finance.entitlements(ids.buyer);
        const granted = rows.filter((row) => row.feature === calls.name);
        expect([
            Object.fromEntries(
                granted.map((row) => [row.source.id === item.id ? "addon" : "plan", row.limit]),
            ),
            Entitlement.resolve(rows, calls.reference),
        ]).toEqual([
            { addon: ADDON_CALLS, plan: CALL_LIMIT },
            {
                kind: "metered",
                limit: CALL_LIMIT + ADDON_CALLS,
                usage: 0,
                resetAt: END,
                state: "within",
            },
        ]);
    },
);

test.each(TEST_DIALECTS)(
    "count a meter event into the usage of its meter's entitlements without deriving the other features again on %s",
    async (dialect) => {
        const { finance, offer } = await open(dialect);
        const active = await transition(
            finance,
            "activate",
            await finance.subscribe(offer, { start: START, end: END }),
        );
        await finance.entitle();

        // change the item's support grant behind the controller and append requests in and after the period
        await finance.test.database
            .update(subscriptionItem.table)
            .set({
                grants: (await Price.offer(finance.test.database, offer.monthly)).grants.map(
                    (grant) =>
                        grant.feature === support.name ? { ...grant, value: "community" } : grant,
                ),
            })
            .where(eq(subscriptionItem.table.parentId, active.id));
        await append(finance, {
            first: { meter: requests.name, value: 10, time: START },
            later: { meter: requests.name, value: 50, time: END },
        });

        // count the requests within the period, and keep the support tier until derivation runs
        const read = async () => {
            const rows = await finance.entitlements(ids.buyer);

            return [
                Entitlement.resolve(rows, calls.reference),
                Entitlement.resolve(rows, support.reference),
            ];
        };
        const counted = await read();
        await finance.entitle();
        const calling = {
            kind: "metered",
            limit: CALL_LIMIT,
            usage: 10,
            resetAt: END,
            state: "within",
        };
        expect([counted, await read()]).toEqual([
            [calling, { kind: "static", value: "priority" }],
            [calling, { kind: "static", value: "community" }],
        ]);
    },
);

test.each(TEST_DIALECTS)(
    "let a grant lapse once the latest release of its feature's package no longer declares the feature on %s",
    async (dialect) => {
        const { finance, offer } = await open(dialect);

        // grant the plan's features to an active subscription
        await transition(
            finance,
            "activate",
            await finance.subscribe(offer, { start: START, end: END }),
        );
        await finance.entitle();
        const features = async () =>
            derived(await finance.entitlements(ids.buyer)).map(([feature]) => feature);
        expect(await features()).toEqual([
            "api.calls",
            "seats",
            "storage.limit",
            "support",
            "sync",
        ]);

        // drop the support entitlement once a later release of storage stops declaring it
        finance.publish(storage, "2026.10.0", [requests, stored, sync, seats, calls, storageLimit]);
        await finance.entitle();
        expect(await features()).toEqual(["api.calls", "seats", "storage.limit", "sync"]);
    },
);
