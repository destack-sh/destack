import { t } from "@destack/locale";
import { Snapshot } from "@destack/db";
import { expect, onTestFinished, refusal, test } from "@destack/test";
import { price, Price, product, productFeature, seller } from "../src/object/index.ts";
import { Finance, ids } from "./fixture/finance.ts";
import { calls, domains, hosting, support, sync } from "./fixture/storage.ts";

test("offer a product at a price granting declared features, read by buyers while active", async () => {
    const finance = await Finance.open();
    onTestFinished(() => finance.close());

    // onboard Carol's shop as a seller pricing credits in euros, and offer a monthly plan
    const shop = await finance.call(
        "carol",
        seller.call("create", {
            accountId: ids.shop,
            creditPrices: { EUR: "1" },
        }),
    );
    const plan = await finance.call(
        "carol",
        product.call("create", {
            accountId: ids.shop,
            name: t`Pro`,
            description: t`Sync, priority support and API access.`,
        }),
    );
    const monthly = await finance.call(
        "carol",
        price.call("create", {
            parentId: plan.id,
            currency: "EUR",
            unitAmount: 900,
            recurring: { interval: "month", intervalCount: 1, usage: "licensed" },
        }),
    );

    // expect the seller onboarding, without a connected account or country before onboarding
    expect([
        shop.status,
        shop.provider,
        shop.providerId,
        shop.country,
        monthly.billingScheme,
    ]).toEqual(["onboarding", "stripe", null, null, "per_unit"]);

    // grant access to sync, priority support and a thousand API calls a period
    const grants = [
        [sync, null],
        [support, "priority"],
        [calls, 1000],
    ] as const;
    for (const [feature, value] of grants) {
        await finance.call(
            "carol",
            productFeature.call("create", {
                parentId: plan.id,
                packageId: feature.package.id,
                feature: feature.name,
                value,
            }),
        );
    }
    const granted = await finance.call(
        "carol",
        productFeature.call("list", {
            where: { parentId: plan.id },
            orderBy: { feature: "asc" },
        }),
    );
    expect(granted.items.map((item) => [item.feature, item.value])).toEqual([
        ["api.calls", 1000],
        ["support", "priority"],
        ["sync", null],
    ]);

    // let buyers outside the shop read its active products alone
    const draft = await finance.call(
        "carol",
        product.call("create", {
            accountId: ids.shop,
            name: t`Team`,
            description: t`Not offered yet.`,
            active: false,
        }),
    );
    const listed = (shopper: "alice" | "carol") =>
        finance.call(
            shopper,
            product.call("list", {
                where: { accountId: ids.shop },
                orderBy: { createdAt: "asc" },
            }),
        );
    expect([
        (await listed("alice")).items.map((item) => item.name.source),
        (await listed("carol")).items.map((item) => item.name.source),
        draft.active,
    ]).toEqual([["Pro"], ["Pro", "Team"], false]);
});

test("let only a shop's sellers offer products in it, granting only features of packages its account publishes", async () => {
    const finance = await Finance.open();
    onTestFinished(() => finance.close());
    const offer = await finance.offer();

    // refuse a product by someone outside the shop
    expect(
        await refusal(
            finance.call(
                "alice",
                product.call("create", {
                    accountId: ids.shop,
                    name: t`Fake`,
                    description: t`Not Carol's.`,
                }),
            ),
        ),
    ).toEqual(["FORBIDDEN", "PERMISSION_DENIED", "permission denied: sell"]);

    // refuse Carol's shop granting a feature of a destack package
    const grant = { packageId: hosting.id, feature: domains.name, value: null };
    expect(
        await refusal(
            finance.call(
                "carol",
                productFeature.call("create", {
                    ...grant,
                    parentId: offer.product,
                }),
            ),
        ),
    ).toEqual([
        "FORBIDDEN",
        "PUBLISHER_MISMATCH",
        "account shop sells package @destack/hosting, which another account publishes",
    ]);

    // let the destack account grant its own package's feature
    const hosted = await finance.call(
        "dave",
        product.call("create", {
            accountId: finance.platform,
            name: t`Hosting`,
            description: t`Apps on custom domains.`,
        }),
    );
    const granted = await finance.call(
        "dave",
        productFeature.call("create", {
            ...grant,
            parentId: hosted.id,
        }),
    );
    expect([granted.packageId, granted.feature]).toEqual([hosting.id, domains.name]);
});

test("refuse offering a price or a product taken off sale", async () => {
    const finance = await Finance.open();
    onTestFinished(() => finance.close());
    const offer = await finance.offer();

    // take the monthly price off sale before the plan
    const offered = () => refusal(Price.offer(offer.monthly, Snapshot.live(finance.database)));
    const before = await offered();
    await finance.call(
        "carol",
        price.call("update", {
            id: offer.monthly.id,
            active: false,
        }),
    );
    const inactivePrice = await offered();
    await finance.call(
        "carol",
        price.call("update", {
            id: offer.monthly.id,
            active: true,
        }),
    );
    await finance.call(
        "carol",
        product.call("update", {
            id: offer.product,
            active: false,
        }),
    );
    expect([before, inactivePrice, await offered()]).toEqual([
        "done",
        ["BAD_REQUEST", "PRICE_INACTIVE", `price ${offer.monthly.id} is inactive`],
        ["BAD_REQUEST", "PRODUCT_INACTIVE", `product ${offer.product} is inactive`],
    ]);
});

test("refuse a price whose included usage exceeds its fee, as the buyer would owe usage they never paid", async () => {
    const finance = await Finance.open();
    onTestFinished(() => finance.close());

    // onboard Carol's shop, and offer a plan whose fee includes 900 of usage
    await finance.call(
        "carol",
        seller.call("create", {
            accountId: ids.shop,
            creditPrices: { EUR: "1" },
        }),
    );
    const plan = await finance.call(
        "carol",
        product.call("create", {
            accountId: ids.shop,
            name: t`Go`,
            description: t`9 a month of usage.`,
        }),
    );
    const monthly = {
        parentId: plan.id,
        currency: "EUR",
        recurring: { interval: "month", intervalCount: 1, usage: "licensed" },
    } as const;
    await finance.call(
        "carol",
        price.call("create", {
            ...monthly,
            unitAmount: 900,
            includedUsage: 900,
        }),
    );

    // refuse a fee below the usage it includes
    expect(
        await refusal(
            finance.call(
                "carol",
                price.call("create", {
                    ...monthly,
                    unitAmount: 500,
                    includedUsage: 900,
                }),
            ),
        ),
    ).toEqual([
        "BAD_REQUEST",
        "INCLUDED_USAGE_INVALID",
        "a price's included usage of 900 exceeds its fee of 500 EUR",
    ]);
});

test("refuse a price in a currency its seller prices no credit in, which usage of credit-listed SKUs could not convert into", async () => {
    const finance = await Finance.open();
    onTestFinished(() => finance.close());

    // onboard Carol's shop pricing credits in euros alone, and offer a plan
    const shop = await finance.call(
        "carol",
        seller.call("create", {
            accountId: ids.shop,
            creditPrices: { EUR: "1" },
        }),
    );
    const plan = await finance.call(
        "carol",
        product.call("create", {
            accountId: ids.shop,
            name: t`Go`,
            description: t`9 a month of usage.`,
        }),
    );

    // refuse a price in dollars
    expect(
        await refusal(
            finance.call(
                "carol",
                price.call("create", {
                    parentId: plan.id,
                    currency: "USD",
                    unitAmount: 900,
                    recurring: { interval: "month", intervalCount: 1, usage: "licensed" },
                }),
            ),
        ),
    ).toEqual(["BAD_REQUEST", "CURRENCY_UNPRICED", `seller ${shop.id} has no credit price in USD`]);
});
