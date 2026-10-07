import { TEST_DIALECTS } from "@destack/db/test";
import { expect, onTestFinished, refusal, test } from "@destack/test";
import { subscription } from "../src/object/index.ts";
import { Finance, ids } from "./fixture/finance.ts";

/** The start of the subscription's first billing period. */
const START = Date.UTC(2026, 9, 1);

/** The end of the first billing period. */
const END = Date.UTC(2026, 10, 1);

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

test.each(TEST_DIALECTS)("refuse subscribing without a customer on %s", async (dialect) => {
    const finance = await Finance.open(dialect);
    onTestFinished(() => finance.close());
    const offer = await finance.offer();

    // refuse a subscription of an account without its billing details
    expect(await refusal(finance.subscribe(offer, { start: START, end: END }))).toEqual([
        "PRECONDITION_FAILED",
        `account ${ids.buyer} has no customer`,
    ]);
});
