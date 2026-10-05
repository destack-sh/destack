import { TEST_DIALECTS } from "@destack/db/test";
import { SystemCall } from "@destack/object";
import { aligned } from "@destack/schema";
import { expect, onTestFinished, refusal, test } from "@destack/test";
import { Price, purchase, subscription, type Subscription } from "../src/object/index.ts";
import { Finance, ids } from "./fixture/finance.ts";

/** The start of the subscription's first billing period. */
const START = Date.UTC(2026, 9, 1);

/** The end of the first billing period. */
const END = Date.UTC(2026, 10, 1);

/** Run a provider's transition on a subscription as the system at a time. */
async function transition(
    finance: Finance,
    name: "trial" | "activate" | "fail" | "cancel",
    row: Subscription,
    now: number,
): Promise<Subscription> {
    return aligned(
        await finance.server.executeAsSystem(subscription, name, [SystemCall.of(row)], now),
        0,
    );
}

test.each(TEST_DIALECTS)(
    "let the account's viewers read a subscription and its managers cancel it at period end, and keep the provider's transitions to the system on %s",
    async (dialect) => {
        const finance = await Finance.open(dialect);
        onTestFinished(() => finance.close());
        await finance.customer();
        const created = await finance.subscribe(await finance.offer(), { start: START, end: END });

        // let the account's viewer read it, and keep it from strangers
        const read = (person: "bob" | "eve") =>
            refusal(finance.call(person, subscription, "get", ids.buyer, { id: created.id }));
        expect([created.status, await read("bob"), await read("eve")]).toEqual([
            "incomplete",
            "done",
            ["NOT_FOUND", `no scope ${ids.buyer}`],
        ]);

        // let the owner cancel at the period's end, and refuse the viewer
        const cancelling = await finance.call("alice", subscription, "update", ids.buyer, {
            id: created.id,
            cancelAtPeriodEnd: true,
        });
        expect([
            cancelling.cancelAtPeriodEnd,
            await refusal(
                finance.call("bob", subscription, "update", ids.buyer, {
                    id: created.id,
                    cancelAtPeriodEnd: false,
                }),
            ),
        ]).toEqual([true, ["FORBIDDEN", "permission denied: bill"]]);

        // refuse the provider's transitions to the owner
        expect(
            await refusal(
                finance.call("alice", subscription, "activate", ids.buyer, { id: created.id }),
            ),
        ).toEqual(["FORBIDDEN", "permission denied: provide"]);
    },
);

test.each(TEST_DIALECTS)(
    "move a subscription through a trial, activation and failed payment to its cancellation, and refuse leaving it on %s",
    async (dialect) => {
        const finance = await Finance.open(dialect);
        onTestFinished(() => finance.close());
        await finance.customer();
        const created = await finance.subscribe(await finance.offer(), { start: START, end: END });

        // move it through each state as the provider would
        const now = Date.now();
        const trialing = await transition(finance, "trial", created, now);
        const active = await transition(finance, "activate", trialing, now);
        const due = await transition(finance, "fail", active, now);
        const canceled = await transition(finance, "cancel", due, now);
        expect([
            [trialing.status, active.status, due.status, canceled.status],
            [due.canceledAt, canceled.canceledAt],
        ]).toEqual([
            ["trialing", "active", "past_due", "canceled"],
            [null, now],
        ]);

        // refuse leaving the canceled state
        expect(await refusal(transition(finance, "activate", canceled, now))).toEqual([
            "CONFLICT",
            "subscription cannot activate while status is canceled",
        ]);
    },
);

test.each(TEST_DIALECTS)(
    "refuse subscribing or buying without a customer, and an item billing a one-time price on %s",
    async (dialect) => {
        const finance = await Finance.open(dialect);
        onTestFinished(() => finance.close());
        const offer = await finance.offer();

        // refuse a subscription and a purchase of an account without its billing details
        const snapshot = await Price.snapshot(finance.test.database, offer.once);
        const missing = `account ${ids.buyer} has no customer`;
        expect([
            await refusal(finance.subscribe(offer, { start: START, end: END })),
            await refusal(
                finance.server.executeAsSystem(
                    purchase,
                    "create",
                    [
                        {
                            scope: ids.buyer,
                            input: {
                                seller: offer.seller,
                                price: offer.once,
                                amount: 9900,
                                ...snapshot,
                            },
                        },
                    ],
                    Date.now(),
                ),
            ),
        ]).toEqual([
            ["PRECONDITION_FAILED", missing],
            ["PRECONDITION_FAILED", missing],
        ]);

        // refuse billing the one-time price each period
        await finance.customer();
        const created = await finance.subscribe(offer, { start: START, end: END });
        expect(await refusal(finance.item(created, offer.once))).toEqual([
            "BAD_REQUEST",
            "a subscription item needs a recurring price",
        ]);
    },
);
