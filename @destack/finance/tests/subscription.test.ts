import { expect, onTestFinished, refusal, test } from "@destack/test";
import { subscription } from "../src/object/index.ts";
import { Finance, ids } from "./fixture/finance.ts";

/** The start of the subscription's first billing period. */
const START = Date.UTC(2026, 9, 1);

/** The end of the first billing period. */
const END = Date.UTC(2026, 10, 1);

test("let the account's viewers read a subscription and its managers cancel it at period end, and leave the provider's transitions to the system", async () => {
    const finance = await Finance.open();
    onTestFinished(() => finance.close());
    await finance.customer();
    const created = await finance.subscribe(await finance.offer(), { start: START, end: END });

    // let the account's viewer read it, and hide it from strangers
    const read = (person: "bob" | "eve") =>
        refusal(finance.call(person, subscription.call("get", { id: created.id })));
    expect([created.status, await read("bob"), await read("eve")]).toEqual([
        "incomplete",
        "done",
        ["NOT_FOUND", "NOT_FOUND", `subscription ${created.id} not found`],
    ]);

    // let the owner cancel at the period's end, and refuse the viewer
    const cancelling = await finance.call(
        "alice",
        subscription.call("update", {
            id: created.id,
            cancelAtPeriodEnd: true,
        }),
    );
    expect([
        cancelling.cancelAtPeriodEnd,
        await refusal(
            finance.call(
                "bob",
                subscription.call("update", {
                    id: created.id,
                    cancelAtPeriodEnd: false,
                }),
            ),
        ),
    ]).toEqual([true, ["FORBIDDEN", "PERMISSION_DENIED", "permission denied: bill"]]);

    // refuse the provider's transitions to the owner
    expect(
        await refusal(finance.call("alice", subscription.call("activate", { id: created.id }))),
    ).toEqual(["FORBIDDEN", "PERMISSION_DENIED", "permission denied: provide"]);
});

test("refuse subscribing without a customer", async () => {
    const finance = await Finance.open();
    onTestFinished(() => finance.close());
    const offer = await finance.offer();

    // refuse a subscription of an account without its billing details
    expect(await refusal(finance.subscribe(offer, { start: START, end: END }))).toEqual([
        "PRECONDITION_FAILED",
        "CUSTOMER_MISSING",
        `account ${ids.buyer} has no customer`,
    ]);
});
