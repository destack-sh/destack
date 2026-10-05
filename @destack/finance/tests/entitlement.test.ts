import { eq } from "@destack/db";
import { TEST_DIALECTS } from "@destack/db/test";
import { SystemCall } from "@destack/object/server";
import { aligned } from "@destack/schema";
import { expect, onTestFinished, refusal, test } from "@destack/test";
import {
    Entitlement,
    meterEvent,
    price,
    Price,
    purchase,
    type Purchase,
    subscription,
    type Subscription,
    subscriptionItem,
} from "../src/object/index.ts";
import {
    CALL_LIMIT,
    Finance,
    ids,
    type Offer,
    PACK_SEATS,
    QUOTA_LIMIT,
    SEATS,
} from "./fixture/finance.ts";
import {
    calls,
    quota,
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
        row.source,
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

/** Append meter events to the buyer's account as the system, by CloudEvents id. */
async function append(
    finance: Finance,
    events: Readonly<Record<string, { meter: string; value: number; time: number }>>,
) {
    for (const [eventId, input] of Object.entries(events)) {
        await finance.server.executeAsSystem(
            meterEvent,
            "create",
            [
                {
                    scope: ids.buyer,
                    input: { eventId, packageId: storage.id, source: SOURCE, ...input },
                },
            ],
            Date.now(),
        );
    }
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

/** Record a pending purchase of a price with its snapshot, the lifetime pack's by default. */
async function buy(
    finance: Finance,
    offer: Offer,
    reference: Offer["once"] = offer.once,
): Promise<Purchase> {
    const snapshot = await Price.snapshot(finance.test.database, reference);
    const input = {
        seller: offer.seller,
        price: reference,
        amount: 9900,
        ...snapshot,
    };

    return aligned(
        await finance.server.executeAsSystem(
            purchase,
            "create",
            [{ scope: ids.buyer, input }],
            Date.now(),
        ),
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
                ["api.calls", "metered", null, CALL_LIMIT, 0, END, "subscription"],
                ["seats", "static", SEATS, null, null, null, "subscription"],
                ["storage.quota", "metered", null, QUOTA_LIMIT, 0, null, "subscription"],
                ["support", "static", "priority", null, null, null, "subscription"],
                ["sync", "boolean", null, null, null, null, "subscription"],
            ],
        ]);

        // remove them once the subscription is canceled
        await transition(finance, "cancel", due);
        await finance.entitle();
        expect(await finance.entitlements(ids.buyer)).toEqual([]);
    },
);

test.each(TEST_DIALECTS)(
    "derive entitlements from a paid purchase's grants, remove them once it is refunded, and refuse a purchase granting metered features on %s",
    async (dialect) => {
        const { finance, offer } = await open(dialect);

        // record a pending purchase of the one-time price, granting nothing yet
        const now = Date.now();
        const pending = await buy(finance, offer);
        await finance.entitle();
        expect(await finance.entitlements(ids.buyer)).toEqual([]);

        // grant each feature of the pack once it is paid
        const paid = aligned(
            await finance.server.executeAsSystem(purchase, "pay", [SystemCall.of(pending)], now),
            0,
        );
        await finance.entitle();
        expect([paid.paidAt, derived(await finance.entitlements(ids.buyer))]).toEqual([
            now,
            [
                ["seats", "static", PACK_SEATS, null, null, null, "purchase"],
                ["support", "static", "community", null, null, null, "purchase"],
                ["sync", "boolean", null, null, null, null, "purchase"],
            ],
        ]);

        // remove them once it is refunded
        await finance.server.executeAsSystem(purchase, "refund", [SystemCall.of(paid)], now);
        await finance.entitle();
        expect(await finance.entitlements(ids.buyer)).toEqual([]);

        // refuse buying a recurring price once, and the plan's metered grants once, which credit grants cover
        const once = await finance.call("carol", price, "create", ids.shop, {
            parentId: offer.product,
            currency: "EUR",
            unitAmount: 9900,
            type: "one_time",
        });
        expect([
            await refusal(buy(finance, offer, offer.monthly)),
            await refusal(buy(finance, offer, { scope: ids.shop, id: once.id })),
        ]).toEqual([
            ["BAD_REQUEST", "a purchase needs a one-time price"],
            ["BAD_REQUEST", `a purchase cannot grant metered feature ${storage.id}/api.calls`],
        ]);
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
            ["storage.quota", 300, null],
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
            ["storage.quota", 300, null],
        ]);
    },
);

test.each(TEST_DIALECTS)(
    "resolve each feature's effective grant across two subscriptions and a lifetime pack on %s",
    async (dialect) => {
        const { finance, offer } = await open(dialect);

        // subscribe to the plan and a fortnight later to an add-on of more calls and unlimited storage, and buy the pack
        await transition(
            finance,
            "activate",
            await finance.subscribe(offer, { start: START, end: END }),
        );
        const addon = [
            { packageId: storage.id, feature: calls.name, value: ADDON_CALLS },
            { packageId: storage.id, feature: quota.name, value: null },
        ];
        const later = await finance.subscribe(offer, { start: MIDDLE, end: NEXT }, addon);
        await transition(finance, "activate", later);
        await finance.server.executeAsSystem(
            purchase,
            "pay",
            [SystemCall.of(await buy(finance, offer))],
            Date.now(),
        );

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
            [sync, seats, support, calls, quota].map((feature) =>
                Entitlement.resolve(rows, feature.reference),
            ),
        ).toEqual([
            { kind: "boolean" },
            { kind: "static", value: PACK_SEATS },
            { kind: "static", value: "community" },
            { kind: "metered", limit: CALL_LIMIT + ADDON_CALLS, usage: 30, resetAt: END },
            { kind: "metered", limit: null, usage: 0, resetAt: null },
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
                granted.map((row) => [row.sourceId === item.id ? "addon" : "plan", row.limit]),
            ),
            Entitlement.resolve(rows, calls.reference),
        ]).toEqual([
            { addon: ADDON_CALLS, plan: CALL_LIMIT },
            { kind: "metered", limit: CALL_LIMIT + ADDON_CALLS, usage: 0, resetAt: END },
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
                grants: (await Price.snapshot(finance.test.database, offer.monthly)).grants.map(
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
        const calling = { kind: "metered", limit: CALL_LIMIT, usage: 10, resetAt: END };
        expect([counted, await read()]).toEqual([
            [calling, { kind: "static", value: "priority" }],
            [calling, { kind: "static", value: "community" }],
        ]);
    },
);
