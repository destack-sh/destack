import { t } from "@destack/locale";
import { asc, eq } from "@destack/db";
import { expect, onTestFinished, test } from "@destack/test";
import { account, Region } from "@destack/account/object";
import { product, productFeature, subscription } from "../src/object/index.ts";
import { Finance, ids } from "./fixture/finance.ts";
import { storage, support } from "./fixture/storage.ts";
import { RequestId } from "@destack/sync";

/** A month in milliseconds. */
const MONTH = 30 * 86_400_000;

test("grant the default product until an account subscribes", async () => {
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

    // derive every account's support from its plan, then again once the buyer subscribes to the paid plan
    const read = async () => {
        const accounts = await finance.database
            .select({ id: account.table.id })
            .from(account.table)
            .orderBy(asc(account.table.id));
        const values = await Promise.all(
            accounts.map(async ({ id }) => {
                const plan = await finance.entitlements(id);
                const granted = plan.find((each) => each.feature === support.name);

                return granted?.kind === "static" ? [[id, granted.value]] : [];
            }),
        );

        return values.flat();
    };
    await finance.derive();
    const before = await read();
    await finance.customer();
    const now = finance.fixture.clock.now();
    const subscribed = await finance.subscribe(offer, { start: now, end: now + MONTH });
    await finance.store.mutate(finance.store.as(subscription), {
        id: RequestId.create(finance.store.environment),
        calls: [subscription.call("activate", subscription.target(subscribed))],
    });
    await finance.derive();

    // grant the default to every account of the region the universe stores, and the paid plan's items alone to the buyer once it subscribes
    const { database } = finance.fixture.space.universe.storage;
    const served = await Region.require(database.snapshot(), "eu");
    const rows = await database
        .select({ id: account.table.id })
        .from(account.table)
        .where(eq(account.table.regionId, served.id))
        .orderBy(asc(account.table.id));
    const accounts = rows.map((row) => row.id);
    expect({ before, after: await read() }).toEqual({
        before: accounts.map((scope) => [scope, "community"]),
        after: accounts.map((scope) => [scope, scope === ids.buyer ? "priority" : "community"]),
    });
});
