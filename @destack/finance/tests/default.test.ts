import { t } from "@destack/locale";
import { asc, eq } from "@destack/db";
import { expect, onTestFinished, refusal, test } from "@destack/test";
import { account, entitlement } from "@destack/account/object";
import { price, product, productFeature, subscription } from "../src/object/index.ts";
import { Finance, ids } from "./fixture/finance.ts";
import { storage, support } from "./fixture/storage.ts";
import { RequestId } from "@destack/sync";

/** A month in milliseconds. */
const MONTH = 30 * 86_400_000;

test("grant every account without a subscription to the shop its default product, refusing prices on it, and stop once the account subscribes", async () => {
    // offer the shop's paid plan and its default product granting community support
    const finance = await Finance.open();
    onTestFinished(() => finance.close());
    const offer = await finance.offer();
    const free = await finance.call(
        "carol",
        product.call("create", {
            accountId: ids.shop,
            name: t`Free`,
            description: t`Community support.`,
            isDefault: true,
        }),
    );
    await finance.call(
        "carol",
        productFeature.call("create", {
            parentId: free.id,
            packageId: storage.id,
            feature: support.name,
            value: "community",
        }),
    );
    const priced = await refusal(
        finance.call(
            "carol",
            price.call("create", {
                parentId: free.id,
                currency: "EUR",
                unitAmount: 0,
                recurring: { interval: "month", intervalCount: 1, usage: "licensed" },
            }),
        ),
    );

    // derive every account's entitlements, then again once the buyer subscribes to the paid plan
    const read = async () => {
        const rows = await finance.database
            .select({ accountId: entitlement.table.scope, value: entitlement.table.value })
            .from(entitlement.table)
            .where(eq(entitlement.table.feature, support.name))
            .orderBy(asc(entitlement.table.scope));

        return rows.map((row) => [row.accountId, row.value]);
    };
    await finance.entitle();
    const before = await read();
    await finance.customer();
    const now = finance.fixture.clock.now();
    const subscribed = await finance.subscribe(offer, { start: now, end: now + MONTH });
    await finance.store.mutate(finance.store.as(subscription), {
        id: RequestId.create(finance.store.environment),
        calls: [subscription.call("activate", subscription.target(subscribed))],
    });
    await finance.entitle();

    // grant the default to every account of the residency the universe stores, and the paid plan's items alone to the buyer once it subscribes
    const residents = await finance.fixture.space.universe.storage.database
        .select({ id: account.table.id })
        .from(account.table)
        .where(eq(account.table.residencyId, "eu"))
        .orderBy(asc(account.table.id));
    const accounts = residents.map((row) => row.id);
    expect({ priced, before, after: await read() }).toEqual({
        priced: [
            "BAD_REQUEST",
            "DEFAULT_PRODUCT_PRICED",
            `product ${free.id} is every account's default, so it cannot have prices`,
        ],
        before: accounts.map((scope) => [scope, "community"]),
        after: accounts.map((scope) => [scope, scope === ids.buyer ? "priority" : "community"]),
    });
});
