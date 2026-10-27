import { t } from "@destack/locale";
import { Package } from "@destack/package";
import { schema } from "@destack/schema";
import { expect, onTestFinished, test } from "@destack/test";
import { defineCatalog, defineFeature } from "../src/declare/index.ts";
import { price, product, productFeature, seller } from "../src/object/index.ts";
import { FinanceFixture } from "../src/test/index.ts";
import type { StackDocument } from "@destack/stack/declare";
import financePackage from "../src/package.ts";

/** The platform's package declaring the plans' features, which the platform's seller publishes. */
const tiers = Package.parse({
    id: "package-019f5530-8000-7000-8000-000000000501",
    name: "@platform/tiers",
    version: "2026.10.0",
});

/** The seats a plan grants. */
const seats = defineFeature(
    {
        name: "seats",
        description: "The seats an account may fill.",
        kind: "static",
        value: schema.number().int().min(1),
    },
    { package: tiers },
);

/** A monthly fee in euro cents, including as much usage. */
function monthly(unitAmount: number) {
    return {
        currency: "EUR",
        unitAmount,
        includedUsage: unitAmount,
        recurring: { interval: "month", intervalCount: 1, usage: "licensed" },
    } as const;
}

/** Declare the platform's seller and a Go plan at a fee, with or without its Pro sibling. */
function catalog(fee: number, isPro: boolean): StackDocument<readonly [typeof financePackage]> {
    const go = {
        name: t`Go`,
        description: t`Go.`,
        isDefault: false,
        prices: { eur: monthly(fee) },
        productFeatures: { seats: { packageId: tiers.id, feature: seats.name, value: 5 } },
    };
    const pro = { ...go, name: t`Pro`, prices: { eur: monthly(10_000) } };

    return {
        sellers: { platform: { provider: "stripe", creditPrices: { EUR: "1" } } },
        ...defineCatalog(isPro ? { go, pro } : { go }),
    };
}

test("declare the platform's seller active and its plans with their prices and grants, store them as declared, price new terms anew, archive the plans the stack stops declaring and keep its seller", async () => {
    // declare Go and Pro twice through the home's stack, which the platform's operator owns
    const finance = await FinanceFixture.open([[tiers, [seats]]]);
    onTestFinished(() => finance.close());
    const { database } = finance;
    const revisions = async () =>
        [
            ...(await database.select().from(seller.table)),
            ...(await database.select().from(product.table)),
            ...(await database.select().from(price.table)),
            ...(await database.select().from(productFeature.table)),
        ].map((row) => [row.id, row.revision]);
    await finance.declare(catalog(800, true));
    const declared = await revisions();
    await finance.declare(catalog(800, true));
    const again = await revisions();

    // raise Go's fee, and stop declaring Pro and the seller
    const { sellers: _sellers, ...plans } = catalog(900, false);
    await finance.declare(plans);
    const sellers = await database.select().from(seller.table);
    const products = await database.select().from(product.table);
    const named = new Map(products.map((row) => [row.id, row.name.source]));
    const prices = await database.select().from(price.table);
    const grants = await database.select().from(productFeature.table);

    expect({
        again,
        sellers: sellers.map((row) => [row.status, row.providerId]),
        products: products.map((row) => [row.name.source, row.active]).toSorted(byJson),
        prices: prices
            .map((row) => [named.get(row.parentId), row.unitAmount, row.active])
            .toSorted(byJson),
        grants: grants
            .map((row) => [named.get(row.parentId), row.feature, row.value])
            .toSorted(byJson),
    }).toEqual({
        // leave every row as it was on declaring the same catalog again
        again: declared,
        sellers: [["active", null]],
        products: [
            ["Go", true],
            ["Pro", false],
        ],
        prices: [
            ["Go", 800, false],
            ["Go", 900, true],
            ["Pro", 10_000, false],
        ],
        grants: [["Go", "seats", 5]],
    });
});

/** Order rows by their JSON text. */
function byJson(left: unknown, right: unknown): number {
    return JSON.stringify(left).localeCompare(JSON.stringify(right));
}
