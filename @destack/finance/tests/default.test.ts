import { asc, eq } from "@destack/db";
import { TEST_DIALECTS } from "@destack/db/test";
import { expect, onTestFinished, refusal, test } from "@destack/test";
import { SystemCall } from "@destack/object";
import { entitlement, price, product, productFeature, subscription } from "../src/object/index.ts";
import { Finance, ids } from "./fixture/finance.ts";
import { storage, support } from "./fixture/storage.ts";

/** A month in milliseconds. */
const MONTH = 30 * 86_400_000;

test.each(TEST_DIALECTS)(
    "grant every account without a subscription to the shop its default product, refusing prices on it, and stop once the account subscribes on %s",
    async (dialect) => {
        // offer the shop's paid plan and its default product granting community support
        const finance = await Finance.open(dialect);
        onTestFinished(() => finance.close());
        const offer = await finance.offer();
        const free = await finance.call("carol", product, "create", ids.shop, {
            name: "Free",
            description: "Community support.",
            isDefault: true,
        });
        await finance.call("carol", productFeature, "create", ids.shop, {
            parentId: free.id,
            packageId: storage.id,
            feature: support.name,
            value: "community",
        });
        const priced = await refusal(
            finance.call("carol", price, "create", ids.shop, {
                parentId: free.id,
                currency: "EUR",
                unitAmount: 0,
                recurring: { interval: "month", intervalCount: 1, usage: "licensed" },
            }),
        );

        // derive every account's entitlements, then again once the buyer subscribes to the paid plan
        const read = async () => {
            const rows = await finance.test.database
                .select({ scope: entitlement.table.scope, source: entitlement.table.source })
                .from(entitlement.table)
                .where(eq(entitlement.table.feature, support.name))
                .orderBy(asc(entitlement.table.scope));

            return rows.map((row) => [row.scope, row.source.type]);
        };
        await finance.entitle();
        const before = await read();
        await finance.customer();
        const now = Date.now();
        const subscribed = await finance.subscribe(offer, { start: now, end: now + MONTH });
        await finance.server.executeAsSystem(
            subscription,
            "activate",
            [SystemCall.of(subscribed)],
            now,
        );
        await finance.entitle();

        // grant the default to every account, and the paid plan's items alone to the buyer once it subscribes
        const accounts = [ids.buyer, ids.payer, ids.shop, ids.destack].toSorted();
        expect({ priced, before, after: await read() }).toEqual({
            priced: [
                "BAD_REQUEST",
                `product ${free.id} is every account's default, which has no prices`,
            ],
            before: accounts.map((scope) => [scope, "product"]),
            after: accounts.map((scope) => [
                scope,
                scope === ids.buyer ? "subscription-item" : "product",
            ]),
        });
    },
);
