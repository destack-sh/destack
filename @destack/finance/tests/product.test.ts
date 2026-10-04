import { TEST_DIALECTS } from "@destack/db/test";
import type { JsonValue } from "@destack/schema";
import { expect, onTestFinished, refusal, test } from "@destack/test";
import { price, Price, product, productFeature, seller } from "../src/object/index.ts";
import { Finance, ids } from "./fixture/finance.ts";
import { calls, domains, hosting, storage, support, sync } from "./fixture/storage.ts";

test.each(TEST_DIALECTS)(
    "offer a product at a price granting declared features, read by buyers while active on %s",
    async (dialect) => {
        const finance = await Finance.open(dialect);
        onTestFinished(() => finance.close());

        // onboard Carol's shop as a seller and offer a monthly plan
        const shop = await finance.call("carol", seller, "create", ids.shop, { country: "AT" });
        const plan = await finance.call("carol", product, "create", ids.shop, {
            name: "Pro",
            description: "Sync, priority support and API access.",
        });
        const monthly = await finance.call("carol", price, "create", ids.shop, {
            parentId: plan.id,
            currency: "EUR",
            unitAmount: 900,
            type: "recurring",
            recurring: { interval: "month", intervalCount: 1, usage: "licensed" },
        });
        expect([shop.status, shop.provider, monthly.billingScheme]).toEqual([
            "onboarding",
            "stripe",
            "per_unit",
        ]);

        // grant access to sync, priority support and a thousand API calls a period
        const grants = [
            [sync, null],
            [support, "priority"],
            [calls, 1000],
        ] as const;
        for (const [feature, value] of grants) {
            await finance.call("carol", productFeature, "create", ids.shop, {
                parentId: plan.id,
                packageId: feature.package.id,
                feature: feature.name,
                value,
            });
        }
        const granted = await finance.call("carol", productFeature, "list", ids.shop, {
            where: { parentId: plan.id },
            orderBy: { feature: "asc" },
        });
        expect(granted.items.map((item) => [item.feature, item.value])).toEqual([
            ["api.calls", 1000],
            ["support", "priority"],
            ["sync", null],
        ]);

        // let buyers outside the shop read its active products alone
        const draft = await finance.call("carol", product, "create", ids.shop, {
            name: "Team",
            description: "Not offered yet.",
            active: false,
        });
        const listed = (shopper: "alice" | "carol") =>
            finance.call(shopper, product, "list", ids.shop, { orderBy: { name: "asc" } });
        expect([
            (await listed("alice")).items.map((item) => item.name),
            (await listed("carol")).items.map((item) => item.name),
            draft.active,
        ]).toEqual([["Pro"], ["Pro", "Team"], false]);
    },
);

test.each(TEST_DIALECTS)(
    "refuse undeclared features, invalid grants, prices without an amount and products of strangers on %s",
    async (dialect) => {
        const finance = await Finance.open(dialect);
        onTestFinished(() => finance.close());
        const offer = await finance.offer();

        // refuse an undeclared feature, a value outside a feature's schema, and a value on access alone
        const grant = (feature: string, value: JsonValue) =>
            refusal(
                finance.call("carol", productFeature, "create", ids.shop, {
                    parentId: offer.product,
                    packageId: storage.id,
                    feature,
                    value,
                }),
            );
        expect([
            await grant("backup", true),
            await grant("support", "gold"),
            await grant("sync", true),
            await grant("api.calls", -1),
        ]).toEqual([
            ["NOT_FOUND", `feature ${storage.id}/backup is not declared`],
            ["BAD_REQUEST", `feature ${storage.id}/support does not accept the granted value`],
            ["BAD_REQUEST", `feature ${storage.id}/sync does not accept the granted value`],
            ["BAD_REQUEST", `feature ${storage.id}/api.calls does not accept the granted value`],
        ]);

        // refuse a price without its amount and products by someone outside the shop
        expect([
            await refusal(
                finance.call("carol", price, "create", ids.shop, {
                    parentId: offer.product,
                    currency: "EUR",
                    type: "one_time",
                }),
            ),
            await refusal(
                finance.call("alice", product, "create", ids.shop, {
                    name: "Fake",
                    description: "Not Carol's.",
                }),
            ),
        ]).toEqual([
            ["INVALID_RECORD", "a record fails a declared check"],
            ["FORBIDDEN", "permission denied: sell"],
        ]);
    },
);

test.each(TEST_DIALECTS)(
    "let a seller grant only features of packages its account publishes on %s",
    async (dialect) => {
        const finance = await Finance.open(dialect);
        onTestFinished(() => finance.close());
        const offer = await finance.offer();

        // refuse Carol's shop granting a feature of a destack package
        const grant = { packageId: hosting.id, feature: domains.name, value: null };
        expect(
            await refusal(
                finance.call("carol", productFeature, "create", ids.shop, {
                    ...grant,
                    parentId: offer.product,
                }),
            ),
        ).toEqual(["FORBIDDEN", "account shop does not publish package @destack/hosting"]);

        // let the destack account grant its own package's feature
        const hosted = await finance.call("dave", product, "create", ids.destack, {
            name: "Hosting",
            description: "Apps on custom domains.",
        });
        const granted = await finance.call("dave", productFeature, "create", ids.destack, {
            ...grant,
            parentId: hosted.id,
        });
        expect([granted.packageId, granted.feature]).toEqual([hosting.id, domains.name]);
    },
);

test.each(TEST_DIALECTS)(
    "refuse snapshotting a price or a product taken off sale on %s",
    async (dialect) => {
        const finance = await Finance.open(dialect);
        onTestFinished(() => finance.close());
        const offer = await finance.offer();

        // take the monthly price off sale, then the plan
        const snapshot = () => refusal(Price.snapshot(finance.test.database, offer.monthly));
        const before = await snapshot();
        await finance.call("carol", price, "update", ids.shop, {
            id: offer.monthly.id,
            active: false,
        });
        const inactivePrice = await snapshot();
        await finance.call("carol", price, "update", ids.shop, {
            id: offer.monthly.id,
            active: true,
        });
        await finance.call("carol", product, "update", ids.shop, {
            id: offer.product,
            active: false,
        });
        expect([before, inactivePrice, await snapshot()]).toEqual([
            "done",
            ["BAD_REQUEST", `price ${offer.monthly.id} is inactive`],
            ["BAD_REQUEST", `product ${offer.product} is inactive`],
        ]);
    },
);
