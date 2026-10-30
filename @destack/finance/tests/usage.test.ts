import { DeclarationReference } from "@destack/package/declare";
import { t } from "@destack/locale";
import { entitlement } from "@destack/account/object";
import { asc, eq, Snapshot } from "@destack/db";
import { announcement } from "@destack/notification";
import { nth, type Identifier, schema, Uuid } from "@destack/schema";
import { testSimulation } from "@destack/object/test";
import type { CallInput } from "@destack/object";
import { ObjectReference, RequestId } from "@destack/sync";
import { expect, onTestFinished, refusal, test } from "@destack/test";
import {
    Usage,
    charge,
    price,
    Price,
    product,
    productFeature,
    seller,
    subscription,
    type Subscription,
    subscriptionItem,
    usage,
} from "../src/object/index.ts";
import { spaces } from "../src/test/index.ts";
import { Finance, ids, resolved, type Offer } from "./fixture/finance.ts";
import {
    archived,
    calls,
    capacity,
    disk,
    diskLimit,
    diskSku,
    machineSku,
    requests,
    storage,
    stored,
    uptime,
} from "./fixture/storage.ts";
import { Event, EventTime } from "@destack/event";

/** The start of the first billing period. */
const START = Date.UTC(2026, 9, 1);

/** The end of the first billing period. */
const END = Date.UTC(2026, 10, 1);

/** A day in milliseconds. */
const DAY = 86_400_000;

/** An hour in milliseconds. */
const HOUR = 3_600_000;

/** The bytes the free plan stores in storage and the archive together. */
const CAPACITY = 1000;

/** A gigabyte, as storage prices count it. */
const GB = 1_000_000_000;

/** The monthly fee of the paid plan, and the usage it includes, in euro cents. */
const FEE = 900;

/** A monthly recurrence. */
const MONTHLY = { interval: "month", intervalCount: 1 } as const;

/** Open the service with Acme's customer and Carol's seller. */
async function open(options: Parameters<typeof Finance.open>[0] = {}) {
    const finance = await Finance.open(options);
    onTestFinished(() => finance.close());
    const offer = await finance.offer();
    await finance.customer();

    return { finance, offer };
}

/** Offer a product in Carol's shop at some prices and features, returning the prices. */
async function offerPlan(
    finance: Finance,
    grants: readonly (readonly [{ readonly name: string }, number | null])[],
    prices: readonly Omit<CallInput<typeof price, "create">, "parentId">[],
): Promise<{ readonly scope: string; readonly id: string }[]> {
    // create the product with its grants
    const plan = await finance.call(
        "carol",
        product.call("create", {
            accountId: ids.shop,
            name: t`Plan`,
            description: t`A plan.`,
        }),
    );
    for (const [feature, value] of grants) {
        await finance.call(
            "carol",
            productFeature.call("create", {
                parentId: plan.id,
                packageId: storage.id,
                feature: feature.name,
                value,
            }),
        );
    }

    // create its prices
    const created = [];
    for (const terms of prices) {
        const row = await finance.call(
            "carol",
            price.call("create", {
                parentId: plan.id,
                ...terms,
            }),
        );
        created.push({ scope: finance.fixture.home, id: row.id });
    }

    return created;
}

/** Offer a free plan in Carol's shop storing some bytes, returning its price. */
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

    return nth(prices, 0);
}

/** Subscribe the buyer to prices for a period as the provider would, active at once. */
async function subscribe(
    finance: Finance,
    offer: Offer,
    references: readonly { readonly scope: string; readonly id: string }[],
    period: { readonly start: number; readonly end: number },
): Promise<Subscription> {
    // create the subscription with an item per price
    const input = {
        ...ObjectReference.columns("seller", seller.reference(offer.seller.scope, offer.seller.id)),
        currentPeriodStart: period.start,
        currentPeriodEnd: period.end,
        providerId: `sub_${Uuid.v4(finance.fixture.environment.random)}`,
    };
    const row = nth(
        await finance.store.mutate(finance.store.as(subscription), {
            id: RequestId.create(finance.fixture.environment),
            calls: [
                subscription.call("create", {
                    ...subscription.in(finance.fixture.home),
                    accountId: ids.buyer,
                    ...input,
                }),
            ],
        }),
        0,
    );
    for (const reference of references) {
        const offered = await Price.offer(reference, Snapshot.live(finance.database));
        await finance.store.mutate(finance.store.as(subscriptionItem), {
            id: RequestId.create(finance.fixture.environment),
            calls: [
                subscriptionItem.call("create", {
                    ...subscriptionItem.in(finance.fixture.home),
                    parentId: row.id,
                    ...ObjectReference.columns(
                        "price",
                        price.reference(reference.scope, reference.id),
                    ),
                    ...offered,
                }),
            ],
        });
    }

    // activate it
    return nth(
        await finance.store.mutate(finance.store.as(subscription), {
            id: RequestId.create(finance.fixture.environment),
            calls: [subscription.call("activate", subscription.target(row))],
        }),
        0,
    );
}

/** Append one use of a space or machine the buyer pays for, as it measured it, and rate it. */
async function append(
    finance: Finance,
    event: {
        readonly id: string;
        readonly space?: Identifier<"space">;
        readonly machine?: Identifier<"machine">;
        readonly meter: string;
        readonly value: number;
        readonly level?: number;
        readonly sku?: DeclarationReference;
        readonly time: number;
    },
): Promise<void> {
    const { meter, value, ...use } = event;
    await finance.fixture.use(ids.buyer, [
        { ...use, meter: { packageId: storage.id, name: meter }, quantity: value },
    ]);
}

/** The moment two days into the period, after the levels the storage tests read are stored. */
const AFTER_STORING = START + 2 * DAY;

/** Read the buyer's resolved usage of a feature. */
async function entitled(finance: Finance, feature: typeof calls) {
    return resolved(await finance.resolutions(ids.buyer), feature.reference);
}

test("add up each space's latest bytes across both meters a feature counts, near its limit and blocked at it, announcing each state to the account's readers of billing once a period", async () => {
    // subscribe the buyer to a free plan storing 1000 bytes, reading two days into the period
    const { finance, offer } = await open({ start: AFTER_STORING });
    const free = await offerFree(finance);
    await subscribe(finance, offer, [free], { start: START, end: END });
    await finance.entitle();

    // store 400 bytes in one space after 300, 200 in another, and archive 300 in the first: near
    const at = START + DAY;
    await append(finance, {
        id: "1",
        space: spaces.notes,
        meter: stored.name,
        value: 300,
        time: at,
    });
    await append(finance, {
        id: "2",
        space: spaces.notes,
        meter: stored.name,
        value: 400,
        time: at + 1,
    });
    await append(finance, {
        id: "3",
        space: spaces.docs,
        meter: stored.name,
        value: 200,
        time: at,
    });
    await append(finance, {
        id: "4",
        space: spaces.notes,
        meter: archived.name,
        value: 300,
        time: at,
    });
    const near = await entitled(finance, capacity);

    // an hour later, archive 500 in the first space: past the limit, blocked
    await finance.fixture.simulation.jump(finance.fixture.clock.now() + HOUR);
    await append(finance, {
        id: "5",
        space: spaces.notes,
        meter: archived.name,
        value: 500,
        time: at + 2,
    });
    const blocked = await entitled(finance, capacity);

    // archive more while blocked, which announces nothing again
    await append(finance, {
        id: "6",
        space: spaces.notes,
        meter: archived.name,
        value: 600,
        time: at + 3,
    });
    const announced = await finance.database
        .select()
        .from(announcement.table)
        .orderBy(asc(announcement.table.createdAt), asc(announcement.table.key));

    expect([near, blocked]).toEqual([
        { kind: "metered", limit: CAPACITY, usage: 900, resetAt: null, state: "near" },
        { kind: "metered", limit: CAPACITY, usage: 1100, resetAt: null, state: "blocked" },
    ]);

    // announce near and blocked once each on the account's customer, to its billing readers
    expect(
        announced.map((row) => [row.name, row.key, row.thread, row.audience, row.payload]),
    ).toEqual([
        [
            "usageLimit",
            `${DeclarationReference.key(capacity.reference)}:near:${START}`,
            `${DeclarationReference.key(capacity.reference)}:${START}`,
            { kind: "permission", permission: "read", reason: "subscribed" },
            { feature: capacity.name, state: "near", usage: 900, limit: CAPACITY },
        ],
        [
            "usageLimit",
            `${DeclarationReference.key(capacity.reference)}:blocked:${START}`,
            `${DeclarationReference.key(capacity.reference)}:${START}`,
            { kind: "permission", permission: "read", reason: "subscribed" },
            { feature: capacity.name, state: "blocked", usage: 1100, limit: CAPACITY },
        ],
    ]);
});

test("announce a usage state once, though its entitlement write is lost and derived again", async () => {
    // store 900 of the free plan's 1000 bytes: near the limit, reading two days into the period
    const { finance, offer } = await open({ start: AFTER_STORING });
    const free = await offerFree(finance);
    await subscribe(finance, offer, [free], { start: START, end: END });
    await finance.entitle();
    await append(finance, {
        id: "1",
        space: spaces.notes,
        meter: stored.name,
        value: 900,
        time: START + DAY,
    });

    // lose the entitlement write at the universe after its near state was announced, then derive again
    await finance.fixture.space.universe.storage.database.transaction(
        async (transaction) =>
            await transaction
                .delete(entitlement.table)
                .where(eq(entitlement.table.scope, ids.buyer)),
    );
    await finance.entitle();
    const announced = await finance.database
        .select({ key: announcement.table.key })
        .from(announcement.table);

    // keep the one announcement of near, as its key was announced this period
    expect(announced).toEqual([
        { key: `${DeclarationReference.key(capacity.reference)}:near:${START}` },
    ]);
});

test("bill an averaged meter by the shares its events carry, 2 GB stored for half the month as 1 GB-month, and cap the level stored now", async () => {
    // subscribe the buyer to a free plan storing 2 GB on disk, read at the month's last instant
    const { finance, offer } = await open({
        start: END - 1,
    });
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
        capped: await entitled(finance, diskLimit),
        billed: await finance.fixture.usage(ids.buyer, disk, period),
    });

    // store 2 GB for the first half of the month, its share of the average half of 2 GB: blocked at the level
    const half = (END - START) / 2;
    const event = { space: spaces.notes, meter: disk.name };
    await append(finance, { ...event, id: "1", value: GB, level: 2 * GB, time: START + half });
    const averaged = await read();

    // store nothing for the rest of the month: within the cap, the bill still 1 GB-month
    await append(finance, { ...event, id: "2", value: 0, level: 0, time: END - 1 });
    const emptied = await read();

    const capped = (used: number, state: string) => ({
        kind: "metered",
        limit: 2 * GB,
        usage: used,
        resetAt: null,
        state,
    });
    expect({ averaged, emptied }).toEqual({
        averaged: { capped: capped(2 * GB, "blocked"), billed: GB },
        emptied: { capped: capped(0, "within"), billed: GB },
    });
});

test("charge a seller's metered usage against the included usage, billing on past it, and block at a spending limit", async () => {
    // subscribe the buyer to a plan including 9 euro of requests at 10 cents each
    const { finance, offer } = await open();
    const prices = await offerPlan(
        finance,
        [[calls, null]],
        [
            {
                currency: "EUR",
                unitAmount: FEE,
                includedUsage: FEE,
                recurring: { ...MONTHLY, usage: "licensed" },
            },
            {
                currency: "EUR",
                unitAmountDecimal: "10",
                recurring: { ...MONTHLY, usage: "metered", meter: requests.reference },
            },
        ],
    );
    const active = await subscribe(finance, offer, prices, { start: START, end: END });
    await finance.entitle();
    const read = () => entitled(finance, calls);

    // spend near the included usage, then past it with billing on
    const states = [];
    await append(finance, {
        id: "1",
        space: spaces.notes,
        meter: requests.name,
        value: 72,
        time: START,
    });
    states.push(await read());
    await append(finance, {
        id: "2",
        space: spaces.notes,
        meter: requests.name,
        value: 18,
        time: START + 1,
    });
    states.push(await read());

    // limit the spend past the included usage to 1 euro: near the 10 euro budget, then blocked at it
    await finance.call(
        "alice",
        subscription.call("update", {
            id: active.id,
            spendingLimit: 100,
        }),
    );
    await finance.entitle();
    states.push(await read());
    await append(finance, {
        id: "3",
        space: spaces.notes,
        meter: requests.name,
        value: 10,
        time: START + 2,
    });
    states.push(await read());

    // describe the limit as unlimited calls, and the state as the budget's
    const metered = (count: number, state: string) => ({
        kind: "metered",
        limit: null,
        usage: count,
        resetAt: END,
        state,
    });
    expect(states).toEqual([
        metered(72, "near"),
        metered(90, "over"),
        metered(90, "near"),
        metered(100, "blocked"),
    ]);
});

test("rate each space's SKU usage at list cost, billing nothing without a subscription to the seller, and refuse usage into a subscription period that ended", async () => {
    // store 100 GB-months in one space and 50 in another, at 2 cents a GB-month, without a subscription
    const { finance, offer } = await open();
    const now = finance.fixture.clock.now();
    for (const [id, space, gigabytes] of [
        ["1", spaces.notes, 100],
        ["2", spaces.docs, 50],
    ] as const) {
        await append(finance, {
            id,
            space,
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
        [
            {
                currency: "EUR",
                unitAmount: FEE,
                includedUsage: FEE,
                recurring: { ...MONTHLY, usage: "licensed" },
            },
        ],
    );
    const ended = await subscribe(finance, offer, paid, {
        start: now - 30 * DAY,
        end: now - DAY,
    });
    const late = await refusal(
        append(finance, {
            id: "3",
            space: spaces.notes,
            meter: disk.name,
            value: GB,
            level: GB,
            sku: diskSku.reference,
            time: now - 2 * DAY,
        }),
    );

    // spend 2 euro and 1 euro of list cost billing nothing, and wait for the provider to start the next period
    expect({ rated, late }).toEqual({
        rated: [
            ["Usage", spaces.notes, 100, "200", "0", "open"],
            ["Usage", spaces.docs, 50, "100", "0", "open"],
        ],
        late: [
            "SERVICE_UNAVAILABLE",
            "PERIOD_ENDED",
            `the period of subscription ${ended.id} ended: retry once the next one starts`,
        ],
    });
});

test("charge each machine's awake hours up to the SKU's 10 hour cap per period, two machines apart, and start the next period fresh", async () => {
    // run machine a 8 then 6 hours and machine b 3 hours in October, then machine a 4 hours in November
    const { finance } = await open();
    const machines = await finance.fixture.declareMachines(ids.buyer, 2);
    const [alpha, beta] = [nth(machines, 0), nth(machines, 1)];
    for (const [id, machine, hours, time] of [
        ["1", alpha, 8, START + DAY],
        ["2", alpha, 6, START + 2 * DAY],
        ["3", beta, 3, START + DAY],
        ["4", alpha, 4, END + DAY],
    ] as const) {
        await append(finance, {
            id,
            machine,
            meter: uptime.name,
            value: hours,
            sku: machineSku.reference,
            time,
        });
    }
    const rows = await finance.database
        .select()
        .from(charge.table)
        .where(eq(charge.table.accountId, ids.buyer))
        .orderBy(asc(charge.table.chargePeriodStart), asc(charge.table.resourceId));

    // tell each line's 14 consumed hours apart from the 10 it prices at 5 credit-cents
    expect(
        rows.map((row) => [
            row.chargePeriodStart,
            row.resourceId,
            row.consumedQuantity,
            row.pricingQuantity,
            row.listCost,
        ]),
    ).toEqual([
        [START, alpha, 14, 10, "50"],
        [START, beta, 3, 3, "15"],
        [END, alpha, 4, 4, "20"],
    ]);
});

test("rate an account's usage after its hot rows flush, as a restarted controller lists only the payers with usage", async () => {
    // append 100 GB-months measured two hours ago, and flush them before any rating
    const { finance } = await open();
    const now = finance.fixture.clock.now();
    await finance.fixture.append(ids.buyer, [
        {
            id: "1",
            space: spaces.notes,
            meter: { packageId: storage.id, name: disk.name },
            quantity: 100 * GB,
            level: 100 * GB,
            sku: diskSku.reference,
            time: now - 2 * HOUR,
        },
    ]);
    await finance.fixture.flush(ids.buyer);

    // list the buyer alone among the replicated accounts, and rate its usage
    expect({
        listed: await finance.fixture.rateListed(),
        charges: await charges(finance),
    }).toEqual({
        listed: [ids.buyer],
        charges: [["Usage", spaces.notes, 100, "200", "0", "open"]],
    });
});

test("rate an account's usage once when two ratings of it overlap", async () => {
    // append 100 and 50 GB-months in two spaces, then rate twice at once
    const { finance } = await open();
    const now = finance.fixture.clock.now();
    await finance.fixture.append(
        ids.buyer,
        [
            ["1", spaces.notes, 100],
            ["2", spaces.docs, 50],
        ].map(([id, space, gigabytes]) => ({
            id: String(id),
            space: schema.identifier("space").parse(space),
            meter: { packageId: storage.id, name: disk.name },
            quantity: Number(gigabytes) * GB,
            level: Number(gigabytes) * GB,
            sku: diskSku.reference,
            time: now,
        })),
    );
    await Promise.all([finance.fixture.rate(ids.buyer), finance.fixture.rate(ids.buyer)]);

    // count each space's use once
    expect(await charges(finance)).toEqual([
        ["Usage", spaces.notes, 100, "200", "0", "open"],
        ["Usage", spaces.docs, 50, "100", "0", "open"],
    ]);
});

test("capture usage of a period its subscription billed already as refused, and rate the usage after it in the same page", async () => {
    // subscribe the buyer to a plan whose period started yesterday, capturing refused uses
    const { finance, offer } = await open();
    const now = finance.fixture.clock.now();
    const paid = await offerPlan(
        finance,
        [[capacity, CAPACITY]],
        [
            {
                currency: "EUR",
                unitAmount: FEE,
                includedUsage: FEE,
                recurring: { ...MONTHLY, usage: "licensed" },
            },
        ],
    );
    const running = await subscribe(finance, offer, paid, {
        start: now - DAY,
        end: now + 29 * DAY,
    });

    // use storage before the period and in it
    const use = (id: string, time: number) => ({
        id,
        space: spaces.notes,
        meter: { packageId: storage.id, name: disk.name },
        quantity: GB,
        level: GB,
        sku: diskSku.reference,
        time,
    });
    await finance.fixture.use(ids.buyer, [use("1", now - 2 * DAY), use("2", now)]);

    // capture the earlier use as refused, and rate the later one into the period
    expect({
        captured: finance.fixture.telemetry.take(),
        rated: await charges(finance),
    }).toEqual({
        captured: [
            {
                scope: "@destack/finance",
                name: "Error",
                message: `usage at ${now - 2 * DAY} is before the current period of subscription ${running.id}, which already billed it`,
            },
        ],
        rated: [["Usage", spaces.notes, 1, "2", "2", "open"]],
    });
});

test("store one credit line per account, seller and period, refusing a second one of the same seller", async () => {
    // credit the buyer for the period from Carol's shop and from another seller
    const { finance, offer } = await open();
    const credit = (sellerId: string) =>
        refusal(
            finance.store.mutate(finance.store.as(charge), {
                id: RequestId.create(finance.fixture.environment),
                calls: [
                    charge.call("create", {
                        ...charge.in(finance.fixture.home),
                        accountId: ids.buyer,
                        ...ObjectReference.columns(
                            "seller",
                            seller.reference(finance.fixture.home, sellerId),
                        ),
                        chargeCategory: "Credit",
                        chargePeriodStart: START,
                        chargePeriodEnd: END,
                        billingCurrency: "EUR",
                        listCost: "0",
                        billedCost: "-900",
                        effectiveCost: "-900",
                        status: "invoiced",
                    }),
                ],
            }),
        );
    const other = "seller-019f5530-8000-7000-8000-0000000000ee";

    expect([
        await credit(offer.seller.id),
        await credit(other),
        await credit(offer.seller.id),
    ]).toEqual([
        "done",
        "done",
        [
            "CONFLICT",
            "DUPLICATE",
            "another charge has the same account, seller, and chargePeriodStart",
        ],
    ]);
});

/** Read the buyer's charges as category, resource, quantity, list and billed cost and status. */
async function charges(finance: Finance) {
    const rows = await finance.database
        .select()
        .from(charge.table)
        .where(eq(charge.table.accountId, ids.buyer))
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

test("derive the account's storage entitlement from the capacity its plan grants, blocked once usage passes it, and no resource for a feature declaring none", async () => {
    // leave the buyer on the free plan, past its capacity, reading two days into the period
    const { finance, offer } = await open({ start: AFTER_STORING });
    const free = await offerFree(finance);
    await subscribe(finance, offer, [free], { start: START, end: END });
    await finance.entitle();
    const within = await resources(finance);
    await append(finance, {
        id: "1",
        space: spaces.notes,
        meter: stored.name,
        value: 1200,
        time: START + DAY,
    });

    expect({ within, blocked: await resources(finance) }).toEqual({
        within: [["storage", CAPACITY, 0, "within"]],
        blocked: [["storage", CAPACITY, 1200, "blocked"]],
    });
});

/** Read the buyer's metered features allowing platform resources, with limit, usage and state. */
async function resources(finance: Finance) {
    const resolutions = await finance.resolutions(ids.buyer);

    return resolutions.flatMap((resolution) =>
        resolution.resource === null || resolution.kind !== "metered"
            ? []
            : [[resolution.resource, resolution.limit, resolution.usage, resolution.state]],
    );
}

test("bill a page of a space's readings as one use per meter, SKU, installation and month, once when the page is billed again after the uses flush", async () => {
    // read 100 GB held an hour twice at the end of October and once in November, an hour before billing
    const { finance } = await open({
        start: END + 2 * HOUR,
    });
    const meter = DeclarationReference.key(disk.reference);
    const sku = DeclarationReference.key(diskSku.reference);
    const level = 100 * GB;
    const read = (id: string, time: number, payerId: string) => ({
        scope: spaces.notes,
        id,
        time: EventTime.of(time),
        keys: { meter, installationId: null, sku, payerId, value: level * HOUR },
        data: { level },
    });
    const page = {
        key: spaces.notes,
        claimedAt: END + 2 * HOUR,
        events: [
            read("1", END - 2 * HOUR, ids.buyer),
            read("2", END - HOUR, ids.buyer),
            read("3", END + HOUR, ids.buyer),
        ],
        after: 10,
        through: 20,
    };

    // bill the page, flush the uses, then bill it again as a retry of the claimed page does
    await finance.fixture.bill(ids.buyer, page);
    await finance.fixture.flush(ids.buyer);
    const hot = () => finance.database.select({ id: usage.table.id }).from(usage.table);
    const flushed = await hot();
    await finance.fixture.bill(ids.buyer, page);
    const uses = async (payer: string) =>
        (
            await finance.fixture.events.query(
                usage.eventKind(),
                { scopes: [payer] },
                { order: "ascending" },
            )
        ).events.map(({ id, time, keys, data }) => ({ id, time, keys, data }));

    // append October's two hours and November's one as their shares of each month, once each
    const use = async (
        from: number,
        to: number,
        hours: number,
        month: number,
        readings: readonly string[],
    ) => ({
        id: await Event.identity(
            spaces.notes,
            [ids.buyer, meter, sku, null, month, ...readings],
            finance.fixture.environment.crypto,
        ),
        time: EventTime.of(from),
        keys: {
            meter,
            sku,
            spaceId: spaces.notes,
            machineId: null,
            installationId: null,
            quantity: (hours * level * HOUR) / (month === START ? 31 * DAY : 30 * DAY),
            level,
        },
        data: { from, to },
    });
    expect({ flushed, retried: await hot(), uses: await uses(ids.buyer) }).toEqual({
        flushed: [],
        retried: [],
        uses: [
            await use(END - 2 * HOUR, END - HOUR, 2, START, ["1", "2"]),
            await use(END + HOUR, END + HOUR, 1, END, ["3"]),
        ],
    });
});

test("fold a page of readings into each payer's uses, a counter's amount as read and a level as its share of the month", async () => {
    // read 100 GB held an hour and 3 machine hours for the buyer, and 2 machine hours for another payer
    const disked = DeclarationReference.key(disk.reference);
    const up = DeclarationReference.key(uptime.reference);
    const diskKey = DeclarationReference.key(diskSku.reference);
    const machineKey = DeclarationReference.key(machineSku.reference);
    const level = 100 * GB;
    const { crypto } = testSimulation().environment("finance");
    const read = (
        id: string,
        meter: string,
        sku: string,
        payerId: string,
        value: number,
        data: object,
    ) => ({
        scope: spaces.notes,
        id,
        time: EventTime.of(START + HOUR),
        keys: { meter, installationId: null, sku, payerId, value },
        data,
    });
    const folded = await Usage.of(
        {
            key: spaces.notes,
            claimedAt: START + 2 * HOUR,
            events: [
                read("1", disked, diskKey, ids.buyer, level * HOUR, { level }),
                read("2", up, machineKey, ids.buyer, 3, {}),
                read("3", up, machineKey, ids.payer, 2, {}),
            ],
            after: 0,
            through: 3,
        },
        crypto,
    );

    // give each payer its own uses, keyed by the page's range
    const use = async (
        payer: string,
        meter: string,
        sku: string,
        quantity: number,
        isLevel: boolean,
        reading: string,
    ) => ({
        id: await Event.identity(spaces.notes, [payer, meter, sku, null, START, reading], crypto),
        time: EventTime.of(START + HOUR),
        meter,
        sku,
        spaceId: spaces.notes,
        machineId: null,
        installationId: null,
        quantity,
        level: isLevel ? level : null,
        data: { from: START + HOUR, to: START + HOUR },
    });
    const request = (payer: string, readings: string[]) =>
        RequestId.derive(START + 2 * HOUR, JSON.stringify([payer, ...readings]), crypto);
    expect(folded).toEqual([
        {
            accountId: ids.buyer,
            requestId: await request(ids.buyer, ["1", "2"]),
            items: [
                await use(ids.buyer, disked, diskKey, (level * HOUR) / (31 * DAY), true, "1"),
                await use(ids.buyer, up, machineKey, 3, false, "2"),
            ],
        },
        {
            accountId: ids.payer,
            requestId: await request(ids.payer, ["3"]),
            items: [await use(ids.payer, up, machineKey, 2, false, "3")],
        },
    ]);
});
