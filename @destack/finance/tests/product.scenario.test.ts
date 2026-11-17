import { t } from "@destack/locale";
import { Snapshot } from "@destack/db";
import { expect, onTestFinished, refusal, test } from "@destack/test";
import { price, Price, product, productFeature, seller } from "../src/object/index.ts";
import { Finance, ids } from "./fixture/finance.ts";
import { calls, domains, hosting, support, sync } from "./fixture/storage.ts";

test("offer a product granting features to buyers while on sale", async () => {
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

    // let buyers outside the shop read its products on sale alone, its sellers the archived one too
    const team = await finance.call(
        "carol",
        product.call("create", {
            accountId: ids.shop,
            name: t`Team`,
            description: t`Not offered yet.`,
        }),
    );
    await finance.call("carol", product.call("archive", { id: team.id }));
    const listed = (shopper: "alice" | "carol") =>
        finance.call(
            shopper,
            product.call("list", {
                where: { accountId: ids.shop },
                orderBy: { createdAt: "asc" },
                archived: "include",
            }),
        );
    expect([
        (await listed("alice")).items.map((item) => item.name.source),
        (await listed("carol")).items.map((item) => item.name.source),
    ]).toEqual([["Pro"], ["Pro", "Team"]]);
});

test("authorize offers by the shop's role and the package's publisher", async () => {
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
    ).toEqual([
        "FORBIDDEN",
        "PERMISSION_DENIED",
        "permission 'sell' is forbidden: it is not granted",
    ]);

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
        "PUBLISHER_DENIED",
        "package '@destack/hosting' is forbidden: account 'shop' cannot sell it, as another account publishes it",
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

test("refuse offering an archived price or product", async () => {
    const finance = await Finance.open();
    onTestFinished(() => finance.close());
    const offer = await finance.offer();

    // take the monthly price off sale before the plan
    const offered = () => refusal(Price.offer(offer.monthly, Snapshot.live(finance.database)));
    const before = await offered();
    await finance.call("carol", price.call("archive", { id: offer.monthly.id }));
    const archivedPrice = await offered();
    await finance.call("carol", price.call("unarchive", { id: offer.monthly.id }));
    await finance.call("carol", product.call("archive", { id: offer.product }));
    expect([before, archivedPrice, await offered()]).toEqual([
        "done",
        ["BAD_REQUEST", "PRICE_INVALID", `price '${offer.monthly.id}' is invalid: it is archived`],
        ["BAD_REQUEST", "PRODUCT_INVALID", `product '${offer.product}' is invalid: it is archived`],
    ]);
});

test("refuse prices their product or seller cannot bill", async () => {
    const finance = await Finance.open();
    onTestFinished(() => finance.close());

    // onboard Carol's shop pricing credits in euros alone, with a plan and a default product
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
    const free = await finance.call(
        "carol",
        product.call("create", {
            accountId: ids.shop,
            name: t`Free`,
            description: t`Community support.`,
            isDefault: true,
        }),
    );

    // offer the plan at a fee including 900 of usage
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

    // refuse a fee below its included usage, a currency without a credit price, and a default product's price
    const refused = [];
    for (const terms of [
        { ...monthly, unitAmount: 500, includedUsage: 900 },
        { ...monthly, currency: "USD", unitAmount: 900 },
        { ...monthly, parentId: free.id, unitAmount: 0 },
    ]) {
        refused.push(await refusal(finance.call("carol", price.call("create", terms))));
    }
    expect(refused).toEqual([
        [
            "BAD_REQUEST",
            "USAGE_INVALID",
            "included usage '900' is invalid: it exceeds the price's fee of 500 'EUR'",
        ],
        [
            "BAD_REQUEST",
            "CURRENCY_INVALID",
            `currency 'USD' is invalid: seller '${shop.id}' has no credit price in it`,
        ],
        [
            "BAD_REQUEST",
            "PRODUCT_INVALID",
            `product '${free.id}' is invalid: it is every account's default, so it cannot have prices`,
        ],
    ]);
});
