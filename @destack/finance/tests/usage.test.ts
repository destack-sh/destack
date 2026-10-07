import { allowance } from "@destack/account/object";
import { asc, eq } from "@destack/db";
import { announcement } from "@destack/notification";
import { TEST_DIALECTS } from "@destack/db/test";
import { SystemCall } from "@destack/object";
import { aligned, type JsonObject } from "@destack/schema";
import { expect, onTestFinished, refusal, test } from "@destack/test";
import {
    charge,
    Entitlement,
    price,
    Price,
    product,
    productFeature,
    subscription,
    type Subscription,
    subscriptionItem,
} from "../src/object/index.ts";
import { Finance, ids, type Offer } from "./fixture/finance.ts";
import {
    archived,
    calls,
    capacity,
    disk,
    diskLimit,
    diskSku,
    requests,
    storage,
    stored,
} from "./fixture/storage.ts";
import { CatalogReference } from "@destack/finance";

/** The start of the first billing period. */
const START = Date.UTC(2026, 9, 1);

/** The end of the first billing period. */
const END = Date.UTC(2026, 10, 1);

/** A day in milliseconds. */
const DAY = 86_400_000;

/** The bytes the free plan keeps in storage and the archive together. */
const CAPACITY = 1000;

/** A gigabyte, as storage prices count it. */
const GB = 1_000_000_000;

/** The monthly fee of the paid plan, and the usage it includes, in euro cents. */
const FEE = 900;

/** A monthly recurrence. */
const MONTHLY = { interval: "month", intervalCount: 1 } as const;

/** Open the service with Acme's customer and Carol's seller. */
async function open(
    dialect: (typeof TEST_DIALECTS)[number],
    options: Parameters<typeof Finance.open>[1] = {},
) {
    const finance = await Finance.open(dialect, options);
    onTestFinished(() => finance.close());
    const offer = await finance.offer();
    await finance.customer();

    return { finance, offer };
}

/** Offer a product in Carol's shop at some prices, granting some features, and return the prices' qualified references. */
async function offerPlan(
    finance: Finance,
    grants: readonly (readonly [{ readonly name: string }, number | null])[],
    prices: readonly JsonObject[],
): Promise<{ readonly scope: string; readonly id: string }[]> {
    // create the product with its grants
    const plan = await finance.call("carol", product, "create", ids.shop, {
        name: "Plan",
        description: "A plan.",
    });
    for (const [feature, value] of grants) {
        await finance.call("carol", productFeature, "create", ids.shop, {
            parentId: plan.id,
            packageId: storage.id,
            feature: feature.name,
            value,
        });
    }

    // create its prices
    const created = [];
    for (const terms of prices) {
        const row = await finance.call("carol", price, "create", ids.shop, {
            parentId: plan.id,
            ...terms,
        });
        created.push({ scope: ids.shop, id: row.id });
    }

    return created;
}

/** Offer a free plan in Carol's shop keeping some bytes, returning its price's qualified reference. */
async function offerFree(
    finance: Finance,
): Promise<{ readonly scope: string; readonly id: string }> {
    const prices = await offerPlan(
        finance,
        [[capacity, CAPACITY]],
        [
            {
                currency: "EUR",
                unitAmount: 0,
                recurring: { ...MONTHLY, usage: "licensed" },
            },
        ],
    );

    return aligned(prices, 0);
}

/** Subscribe the buyer to prices for a period as the provider would, active at once, including the usage its prices include when given. */
async function subscribe(
    finance: Finance,
    offer: Offer,
    references: readonly { readonly scope: string; readonly id: string }[],
    period: { readonly start: number; readonly end: number },
    includedUsage = 0,
): Promise<Subscription> {
    // create the subscription with an item per price
    const input = {
        seller: offer.seller,
        currentPeriodStart: period.start,
        currentPeriodEnd: period.end,
        includedUsage,
        providerId: `sub_${crypto.randomUUID()}`,
    };
    const row = aligned(
        await finance.server.executeAsSystem(
            subscription,
            "create",
            [{ scope: ids.buyer, input }],
            Date.now(),
        ),
        0,
    );
    for (const reference of references) {
        const offered = await Price.offer(finance.test.database, reference);
        await finance.server.executeAsSystem(
            subscriptionItem,
            "create",
            [{ scope: ids.buyer, input: { parentId: row.id, price: reference, ...offered } }],
            Date.now(),
        );
    }

    // activate it
    return aligned(
        await finance.server.executeAsSystem(
            subscription,
            "activate",
            [SystemCall.of(row)],
            Date.now(),
        ),
        0,
    );
}

/** Route one use to the buyer's account as a cell would, by source and identity, and rate it. */
async function append(
    finance: Finance,
    event: {
        readonly id: string;
        readonly source: string;
        readonly meter: string;
        readonly value: number;
        readonly level?: number;
        readonly sku?: CatalogReference;
        readonly time: number;
    },
): Promise<void> {
    const { meter, value, ...use } = event;
    await finance.fixture.use(ids.buyer, [
        { ...use, meter: { packageId: storage.id, name: meter }, quantity: value },
    ]);
}

/** Read the buyer's resolved usage of a feature. */
async function resolved(finance: Finance, feature: typeof calls) {
    return Entitlement.resolve(await finance.entitlements(ids.buyer), feature.reference);
}

test.each(TEST_DIALECTS)(
    "add up each space's latest bytes across both meters a feature counts, near its limit and blocked at it, announcing each state to the account's readers of billing once a period on %s",
    async (dialect) => {
        // subscribe the buyer to a free plan keeping 1000 bytes
        const { finance, offer } = await open(dialect);
        const free = await offerFree(finance);
        await subscribe(finance, offer, [free], { start: START, end: END });
        await finance.entitle();

        // keep 400 bytes in one space after 300, 200 in another, and archive 300 in the first: near
        const at = START + DAY;
        await append(finance, {
            id: "1",
            source: "/spaces/a",
            meter: stored.name,
            value: 300,
            time: at,
        });
        await append(finance, {
            id: "2",
            source: "/spaces/a",
            meter: stored.name,
            value: 400,
            time: at + 1,
        });
        await append(finance, {
            id: "3",
            source: "/spaces/b",
            meter: stored.name,
            value: 200,
            time: at,
        });
        await append(finance, {
            id: "4",
            source: "/spaces/a",
            meter: archived.name,
            value: 300,
            time: at,
        });
        const near = await resolved(finance, capacity);

        // archive 500 in the first space: past the limit, blocked
        await append(finance, {
            id: "5",
            source: "/spaces/a",
            meter: archived.name,
            value: 500,
            time: at + 2,
        });
        const blocked = await resolved(finance, capacity);

        // archive more while blocked, which announces nothing again
        await append(finance, {
            id: "6",
            source: "/spaces/a",
            meter: archived.name,
            value: 600,
            time: at + 3,
        });
        const announced = await finance.test.database
            .select()
            .from(announcement.table)
            .orderBy(asc(announcement.table.createdAt), asc(announcement.table.key));

        expect([near, blocked]).toEqual([
            { kind: "metered", limit: CAPACITY, usage: 900, resetAt: null, state: "near" },
            { kind: "metered", limit: CAPACITY, usage: 1100, resetAt: null, state: "blocked" },
        ]);

        // announce near and blocked once each to the readers of the entitlement, the account's billing readers
        expect(
            announced.map((row) => [row.name, row.key, row.thread, row.audience, row.payload]),
        ).toEqual([
            [
                "usageLimit",
                `near:${START}`,
                `${capacity.name}:${START}`,
                { kind: "permission", permission: "read", reason: "subscribed" },
                { feature: capacity.name, state: "near", usage: 900, limit: CAPACITY },
            ],
            [
                "usageLimit",
                `blocked:${START}`,
                `${capacity.name}:${START}`,
                { kind: "permission", permission: "read", reason: "subscribed" },
                { feature: capacity.name, state: "blocked", usage: 1100, limit: CAPACITY },
            ],
        ]);
    },
);

test.each(TEST_DIALECTS)(
    "bill an averaged meter by the shares its events carry, 2 GB kept for half the month as 1 GB-month, and cap the level kept now on %s",
    async (dialect) => {
        // subscribe the buyer to a free plan keeping 2 GB on disk
        const { finance, offer } = await open(dialect);
        const prices = await offerPlan(
            finance,
            [[diskLimit, 2 * GB]],
            [
                {
                    currency: "EUR",
                    unitAmount: 0,
                    recurring: { ...MONTHLY, usage: "licensed" },
                },
            ],
        );
        await subscribe(finance, offer, prices, { start: START, end: END });
        await finance.entitle();
        const period = { start: START, end: END };
        const read = async () => ({
            capped: await resolved(finance, diskLimit),
            billed: await finance.fixture.meterUsage.read(ids.buyer, disk, period),
        });

        // keep 2 GB for the first half of the month, its share of the average half of 2 GB: blocked at the level
        const half = (END - START) / 2;
        const event = { source: "/spaces/a", meter: disk.name };
        await append(finance, { ...event, id: "1", value: GB, level: 2 * GB, time: START + half });
        const kept = await read();

        // keep nothing for the rest of the month: within the cap, the bill still 1 GB-month
        await append(finance, { ...event, id: "2", value: 0, level: 0, time: END - 1 });
        const emptied = await read();

        const capped = (usage: number, state: string) => ({
            kind: "metered",
            limit: 2 * GB,
            usage,
            resetAt: null,
            state,
        });
        expect({ kept, emptied }).toEqual({
            kept: { capped: capped(2 * GB, "blocked"), billed: GB },
            emptied: { capped: capped(0, "within"), billed: GB },
        });
    },
);

test.each(TEST_DIALECTS)(
    "charge a seller's metered usage against the included usage, billing on past it, and block at a spending limit on %s",
    async (dialect) => {
        // subscribe the buyer to a plan including 9 euro of requests at 10 cents each
        const { finance, offer } = await open(dialect);
        const prices = await offerPlan(
            finance,
            [[calls, null]],
            [
                {
                    currency: "EUR",
                    unitAmount: FEE,
                    recurring: { ...MONTHLY, usage: "licensed" },
                },
                {
                    currency: "EUR",
                    unitAmountDecimal: "10",
                    recurring: { ...MONTHLY, usage: "metered", meter: requests.reference },
                },
            ],
        );
        const active = await subscribe(finance, offer, prices, { start: START, end: END }, FEE);
        await finance.entitle();
        const read = () => resolved(finance, calls);

        // spend near the included usage, then past it with billing on
        const states = [];
        await append(finance, {
            id: "1",
            source: "/spaces/a",
            meter: requests.name,
            value: 72,
            time: START,
        });
        states.push(await read());
        await append(finance, {
            id: "2",
            source: "/spaces/a",
            meter: requests.name,
            value: 18,
            time: START + 1,
        });
        states.push(await read());

        // limit the spend past the included usage to 1 euro: near the 10 euro budget, then blocked at it
        await finance.call("alice", subscription, "update", ids.buyer, {
            id: active.id,
            spendingLimit: 100,
        });
        await finance.entitle();
        states.push(await read());
        await append(finance, {
            id: "3",
            source: "/spaces/a",
            meter: requests.name,
            value: 10,
            time: START + 2,
        });
        states.push(await read());

        // describe the limit as unlimited calls, and the state as the budget's
        const usage = (count: number, state: string) => ({
            kind: "metered",
            limit: null,
            usage: count,
            resetAt: END,
            state,
        });
        expect(states).toEqual([
            usage(72, "near"),
            usage(90, "over"),
            usage(90, "near"),
            usage(100, "blocked"),
        ]);
    },
);

test.each(TEST_DIALECTS)(
    "rate each space's SKU usage at list cost, billing nothing without a subscription to the seller, and refuse usage into a subscription period that ended on %s",
    async (dialect) => {
        // keep 100 GB-months in one space and 50 in another, at 2 cents a GB-month, without a subscription
        const { finance, offer } = await open(dialect);
        const now = Date.now();
        for (const [id, source, gigabytes] of [
            ["1", "/spaces/a", 100],
            ["2", "/spaces/b", 50],
        ] as const) {
            await append(finance, {
                id,
                source,
                meter: disk.name,
                value: gigabytes * GB,
                level: gigabytes * GB,
                sku: diskSku.reference,
                time: now,
            });
        }
        const rated = await charges(finance);

        // subscribe to a plan whose period ended, then use storage again
        const paid = await offerPlan(
            finance,
            [[capacity, CAPACITY]],
            [{ currency: "EUR", unitAmount: FEE, recurring: { ...MONTHLY, usage: "licensed" } }],
        );
        const ended = await subscribe(
            finance,
            offer,
            paid,
            { start: now - 30 * DAY, end: now - DAY },
            FEE,
        );
        const late = await refusal(
            append(finance, {
                id: "3",
                source: "/spaces/a",
                meter: disk.name,
                value: GB,
                level: GB,
                sku: diskSku.reference,
                time: now - 2 * DAY,
            }),
        );

        // keep 2 euro and 1 euro of list cost billing nothing, and wait for the provider to start the next period
        expect({ rated, late }).toEqual({
            rated: [
                ["Usage", "/spaces/a", 100, "200", "0", "open"],
                ["Usage", "/spaces/b", 50, "100", "0", "open"],
            ],
            late: [
                "SERVICE_UNAVAILABLE",
                `the period of subscription ${ended.id} ended, retry once the next one starts`,
            ],
        });
    },
);

/** Read the buyer's charges as category, resource, quantity in pricing units, list and billed cost and status. */
async function charges(finance: Finance) {
    const rows = await finance.test.database
        .select()
        .from(charge.table)
        .where(eq(charge.table.scope, ids.buyer))
        .orderBy(asc(charge.table.chargeCategory), asc(charge.table.resourceId));

    return rows.map((row) => [
        row.chargeCategory,
        row.resourceId,
        row.pricingQuantity,
        row.listCost,
        row.billedCost,
        row.status,
    ]);
}

test.each(TEST_DIALECTS)(
    "derive the account's storage allowance from the capacity its plan grants, blocked once usage passes it, and none for a feature declaring no allowance on %s",
    async (dialect) => {
        // keep the buyer on the free plan, past its capacity
        const { finance, offer } = await open(dialect);
        const free = await offerFree(finance);
        await subscribe(finance, offer, [free], { start: START, end: END });
        await finance.entitle();
        const within = await allowances(finance);
        await append(finance, {
            id: "1",
            source: "/spaces/a",
            meter: stored.name,
            value: 1200,
            time: START + DAY,
        });

        expect({ within, blocked: await allowances(finance) }).toEqual({
            within: [["storage", CAPACITY, 0, "within"]],
            blocked: [["storage", CAPACITY, 1200, "blocked"]],
        });
    },
);

/** Read the buyer's allowances as resource, limit, usage and state. */
async function allowances(finance: Finance) {
    const rows = await finance.test.database
        .select()
        .from(allowance.table)
        .where(eq(allowance.table.scope, ids.buyer));

    return rows.map((row) => [row.resource, row.limit, row.used, row.state]);
}
