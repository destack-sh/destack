import { TEST_DIALECTS } from "@destack/db/test";
import { expect, onTestFinished, refusal, test } from "@destack/test";
import { customer, Customer } from "../src/object/index.ts";
import { Finance, ids } from "./fixture/finance.ts";

test.each(TEST_DIALECTS)(
    "keep an account's billing details, refusing a second customer and billing by a viewer on %s",
    async (dialect) => {
        const finance = await Finance.open(dialect);
        onTestFinished(() => finance.close());

        // keep the buyer's billing details with an empty tax list by default
        const buyer = await finance.call("alice", customer, "create", ids.buyer, {
            email: "billing@acme.test",
            name: "Acme",
            address: { line1: "Main Street 1", city: "Vienna", country: "AT" },
        });
        expect([buyer.address, buyer.taxIds]).toEqual([
            { line1: "Main Street 1", city: "Vienna", country: "AT" },
            [],
        ]);

        // refuse a second customer of the account, a field outside the billing details, and billing by a viewer
        expect([
            await refusal(
                finance.call("alice", customer, "create", ids.buyer, {
                    email: "other@acme.test",
                    name: "Acme",
                }),
            ),
            await refusal(
                finance.call("alice", customer, "update", ids.buyer, {
                    id: buyer.id,
                    providerId: "cus_1",
                }),
            ),
            await refusal(
                finance.call("bob", customer, "update", ids.buyer, { id: buyer.id, name: "Bob" }),
            ),
        ]).toEqual([
            ["DUPLICATE", "a record with the same unique key exists"],
            ["BAD_REQUEST", "invalid input to customer.update"],
            ["FORBIDDEN", "permission denied: bill"],
        ]);
    },
);

test.each(TEST_DIALECTS)(
    "resolve the paying account: the organisation's payer, or the account itself for a user's account on %s",
    async (dialect) => {
        const finance = await Finance.open(dialect);
        onTestFinished(() => finance.close());

        // consolidate Acme's accounts to its payer, and keep Carol's shop paying for itself
        const database = finance.test.database;
        expect([
            await Customer.payer(database, ids.buyer),
            await Customer.payer(database, ids.payer),
            await Customer.payer(database, ids.shop),
        ]).toEqual([ids.payer, ids.payer, ids.shop]);
    },
);
