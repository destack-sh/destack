import { t } from "@destack/locale";
import { TEST_DIALECTS } from "@destack/db/test";
import { Package } from "@destack/package";
import { schema } from "@destack/schema";
import { expect, onTestFinished, test } from "@destack/test";
import { defineFeature } from "../src/declare/index.ts";
import { price, product, productFeature, seller } from "../src/object/index.ts";
import { FinanceFixture } from "../src/test/index.ts";
import type { StackDefinition } from "@destack/stack/declare";

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
function catalog(
    fee: number,
    isPro: boolean,
): Required<Pick<StackDefinition, "sellers" | "products">> {
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
        products: isPro ? { go, pro } : { go },
    };
}

test.each(TEST_DIALECTS)(
    "declare the platform's seller active and its plans with their prices and grants, store them as declared, price new terms anew, archive the plans the stack stops declaring and keep its seller on %s",
    async (dialect) => {
        // declare Go and Pro twice
        const finance = await FinanceFixture.open(dialect, [[tiers, [seats]]]);
        onTestFinished(() => finance.close());
        await finance.declare(catalog(800, true));
        const again = await finance.declare(catalog(800, true));

        // raise Go's fee, and stop declaring Pro and the seller
        const changed = await finance.declare({ products: catalog(900, false).products });
        const { database } = finance.test;
        const sellers = await database.select().from(seller.table);
        const products = await database.select().from(product.table);
        const named = new Map(products.map((row) => [row.id, row.name.source]));
        const prices = await database.select().from(price.table);
        const grants = await database.select().from(productFeature.table);

        expect({
            again: again.steps,
            changed: changed.steps.map((step) => [step.action, step.target.split("/")[0]]),
            sellers: sellers.map((row) => [row.status, row.providerId]),
            products: products.map((row) => [row.name.source, row.active]).toSorted(byJson),
            prices: prices
                .map((row) => [named.get(row.parentId), row.unitAmount, row.active])
                .toSorted(byJson),
            grants: grants
                .map((row) => [named.get(row.parentId), row.feature, row.value])
                .toSorted(byJson),
        }).toEqual({
            again: [],
            // write Go's new price, then retire in reverse order: Pro's grant, the old prices, Pro
            changed: [
                ["create", "price"],
                ["delete", "product-feature"],
                ["update", "price"],
                ["update", "price"],
                ["update", "product"],
            ],
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
    },
);

/** Order rows by their JSON text. */
function byJson(left: unknown, right: unknown): number {
    return JSON.stringify(left).localeCompare(JSON.stringify(right));
}
