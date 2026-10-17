import { TEST_DIALECTS } from "@destack/db/test";
import { expect, onTestFinished, refusal, test } from "@destack/test";
import { customer } from "../src/object/index.ts";
import { Finance, ids } from "./fixture/finance.ts";

test.each(TEST_DIALECTS)(
    "store an account's billing details, refusing a second customer and billing by a viewer on %s",
    async (dialect) => {
        const finance = await Finance.open(dialect);
        onTestFinished(() => finance.close());

        // store the buyer's billing details with an empty tax list by default
        const buyer = await finance.call(
            "alice",
            customer.call("create", {
                accountId: ids.buyer,
                email: "billing@acme.test",
                name: "Acme",
                address: { line1: "Main Street 1", city: "Vienna", country: "AT" },
            }),
        );
        expect([buyer.address, buyer.taxIds]).toEqual([
            { line1: "Main Street 1", city: "Vienna", country: "AT" },
            [],
        ]);

        // refuse a second customer of the account, a field outside the billing details, and billing by a viewer
        expect([
            await refusal(
                finance.call(
                    "alice",
                    customer.call("create", {
                        accountId: ids.buyer,
                        email: "other@acme.test",
                        name: "Acme",
                    }),
                ),
            ),
            await refusal(
                finance.send("alice", customer, "update", { id: buyer.id, providerId: "cus_1" }),
            ),
            await refusal(
                finance.call("bob", customer.call("update", { id: buyer.id, name: "Bob" })),
            ),
        ]).toEqual([
            ["CONFLICT", "another customer has the same account"],
            ["BAD_REQUEST", 'invalid input: unrecognized key: "providerId"'],
            ["FORBIDDEN", "permission denied: bill"],
        ]);
    },
);
