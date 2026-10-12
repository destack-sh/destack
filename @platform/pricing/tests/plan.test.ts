import { user } from "@destack/account/object";
import { TEST_DIALECTS } from "@destack/db/test";
import { price, product, productFeature } from "@destack/finance/object";
import { FinanceFixture } from "@destack/finance/test";
import { databaseBytes, METERS } from "@destack/observability/meter";
import { expect, onTestFinished, test } from "@destack/test";
import {
    compute,
    customDomains,
    CREDIT_PRICES,
    database,
    network,
    PLANS,
    storage,
    support,
    tenancy,
    UNIT_SKUS,
} from "../src/index.ts";

/** The operator owning the platform's account. */
const OPERATOR = user.identifier("user-019f5530-8000-7000-8000-000000000403");

test.each(TEST_DIALECTS)(
    "declare the plans as the platform's products, each plan's fee including as much usage and each plan's grants, dedicated tenancy from Plus up, the same however often applied, on %s",
    async (dialect) => {
        // serve finance with the plans' features and SKUs and observability's meters
        const finance = await FinanceFixture.open(dialect, [
            [
                database.package,
                [
                    database,
                    storage,
                    compute,
                    network,
                    tenancy,
                    support,
                    customDomains,
                    ...UNIT_SKUS,
                ],
            ],
            [databaseBytes.package, METERS],
        ]);
        onTestFinished(() => finance.close());
        await finance.operate(OPERATOR);

        // declare the platform's seller and the plans twice, the second changing nothing
        const definition = {
            sellers: {
                platform: { provider: "stripe", country: "AT", creditPrices: CREDIT_PRICES },
            },
            products: PLANS,
        } as const;
        await finance.declare(definition);
        const again = await finance.declare(definition);

        // read the declared products, their fees and their grants
        const records = finance.test.database;
        const products = await records.select().from(product.table);
        const named = new Map(products.map((row) => [row.id, row.name.source]));
        const prices = await records.select().from(price.table);
        const grants = await records.select().from(productFeature.table);

        expect({
            again: again.steps,
            products: products
                .map((row) => [row.name.source, row.description.source])
                .toSorted((left, right) => left.join().localeCompare(right.join())),
            fees: prices
                .filter((row) => row.currency === "CAD" || row.currency === "USD")
                .map((row) => [
                    named.get(row.parentId),
                    row.currency,
                    row.unitAmount,
                    row.includedUsage,
                ])
                .toSorted((left, right) =>
                    JSON.stringify(left).localeCompare(JSON.stringify(right)),
                ),
            prices: prices.length,
            free: grants
                .filter((row) => named.get(row.parentId) === "Free")
                .toSorted((left, right) => left.feature.localeCompare(right.feature))
                .map((row) => [row.feature, row.value]),
            tenancy: grants
                .filter((row) => row.feature === "tenancy")
                .map((row) => [named.get(row.parentId), row.value])
                .toSorted((left, right) =>
                    JSON.stringify(left).localeCompare(JSON.stringify(right)),
                ),
        }).toEqual({
            again: [],
            products: [
                ["Free", "Meet Destack."],
                ["Go", "More room to build."],
                ["Plus", "More power, on machines of your own."],
                ["Pro", "The most out of Destack."],
            ],
            fees: [
                ["Go", "CAD", 1100, 1100],
                ["Go", "USD", 800, 800],
                ["Plus", "CAD", 2800, 2800],
                ["Plus", "USD", 2000, 2000],
                ["Pro", "CAD", 14_000, 14_000],
                ["Pro", "USD", 10_000, 10_000],
            ],
            prices: 3 * 5,
            free: [
                ["compute", 45_000],
                ["database", 1_000_000_000],
                ["network", 100_000_000_000],
                ["storage", 10_000_000_000],
                ["support", "community"],
                ["tenancy", ["shared", "own"]],
            ],
            tenancy: [
                ["Free", ["shared", "own"]],
                ["Go", ["shared", "own"]],
                ["Plus", ["shared", "dedicated", "own"]],
                ["Pro", ["shared", "dedicated", "own"]],
            ],
        });
    },
);
