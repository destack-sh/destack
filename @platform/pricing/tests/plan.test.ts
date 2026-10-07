import { account, user } from "@destack/account/object";
import { TEST_DIALECTS } from "@destack/db/test";
import { Cost, MeterReference } from "@destack/finance";
import { ProductDefinition } from "@destack/finance/client";
import { FinanceFixture } from "@destack/finance/test";
import { databaseStorage, METERS } from "@destack/observability/meter";
import { present } from "@destack/schema";
import { RequestId } from "@destack/service/request";
import { expect, onTestFinished, refusal, test } from "@destack/test";
import {
    capacityClasses,
    compute,
    customDomains,
    CREDIT_PRICES,
    members,
    OPERATION_COSTS,
    OPERATION_PRICES,
    PLANS,
    UNIT_COSTS,
    UNIT_SKUS,
    storage,
    support,
} from "../src/index.ts";

/** Read a price's margin over what it costs, in basis points. */
function margin(price: bigint, paid: bigint): number {
    return Number(((price - paid) * 10_000n) / price);
}

/** The platform's account, which sells the plans. */
const PLATFORM = account.identifier("account-019f5530-8000-7000-8000-000000000401");

/** The account buying a plan. */
const BUYER = account.identifier("account-019f5530-8000-7000-8000-000000000402");

/** The operator owning the platform's account. */
const OPERATOR = user.identifier("user-019f5530-8000-7000-8000-000000000403");

/** The person owning the buying account. */
const ADA = user.identifier("user-019f5530-8000-7000-8000-000000000404");

test("rate each platform meter by exactly one unit, every priced unit and operation 20 to 25 percent above its cost at the US dollar's credit price", () => {
    // count the units rating each meter
    const rated = METERS.map(
        (meter) =>
            UNIT_SKUS.filter(
                (sku) =>
                    MeterReference.key(sku.definition.meter) ===
                    MeterReference.key(meter.reference),
            ).length,
    );

    // read each priced unit's margin over its cost per meter unit, a credit-cent at the dollar's price, in basis points
    const dollar = Cost.scaled(CREDIT_PRICES.USD);
    const units = UNIT_COSTS.filter(({ unit }) => unit.definition.unitAmount !== "0").map(
        ({ unit, cost }) =>
            margin(
                (Cost.scaled(unit.definition.unitAmount) * dollar * BigInt(cost.unitSize)) /
                    Cost.whole(1),
                Cost.scaled(cost.unitAmount) * BigInt(unit.definition.unitSize),
            ),
    );

    // read each operation's margin, its price in millionths of a credit against its cost in cents
    const operations = OPERATION_COSTS.map(({ operation, cost }) =>
        margin(
            BigInt(Math.round(OPERATION_PRICES[operation] * 1_000_000)),
            (Cost.scaled(cost.unitAmount) * 10_000n * 1_000_000n) / Cost.whole(1),
        ),
    );

    expect({
        rated,
        margins: [...units, ...operations].every((each) => each >= 2000 && each <= 2500),
    }).toEqual({ rated: METERS.map(() => 1), margins: true });
});

test.each(TEST_DIALECTS)(
    "publish the plans as the platform account's products once, start the free plan at once and refuse a paid one without a payment provider on %s",
    async (dialect) => {
        // serve finance with the plans' features and SKUs and observability's meters, the platform and a buyer
        const finance = await FinanceFixture.open(dialect, [
            [
                storage.package,
                [storage, compute, capacityClasses, support, customDomains, members, ...UNIT_SKUS],
            ],
            [databaseStorage.package, METERS],
        ]);
        onTestFinished(() => finance.close());
        await finance.account(PLATFORM, "platform", OPERATOR);
        await finance.account(BUYER, "ada", ADA);

        // publish the plans twice, the second keeping everything
        const operator = finance.client(OPERATOR);
        await operator.seller.create({
            accountId: PLATFORM,
            requestId: RequestId.create(),
            country: "AT",
            creditPrices: CREDIT_PRICES,
        });
        for (const plan of [...PLANS, ...PLANS]) {
            await ProductDefinition.publish(operator, PLATFORM, plan);
        }
        const { items: products } = await operator.product.list({
            accountId: PLATFORM,
            orderBy: { name: "asc" },
        });
        const { items: prices } = await operator.price.list({ accountId: PLATFORM });

        // start Ada on the free plan, and refuse Go
        const ada = finance.client(ADA);
        await ada.customer.create({
            accountId: BUYER,
            requestId: RequestId.create(),
            email: "ada@example.com",
            name: "Ada",
        });
        const plan = (name: string) =>
            present(
                products.find((each) => each.name === name),
                name,
            );
        const started = await ada.checkoutSession.create({
            accountId: BUYER,
            requestId: RequestId.create(),
            product: { scope: PLATFORM, id: plan("Free").id },
            currency: "CHF",
        });
        const refused = await refusal(
            ada.checkoutSession.create({
                accountId: BUYER,
                requestId: RequestId.create(),
                product: { scope: PLATFORM, id: plan("Go").id },
                currency: "CHF",
            }),
        );
        await finance.control("customer");
        const { items: entitlements } = await ada.entitlement.list({
            accountId: BUYER,
            orderBy: { feature: "asc" },
        });
        const derived = await ada.allowance.list({
            accountId: BUYER,
            orderBy: { resource: "asc" },
        });

        expect({
            products: products.map((each) => each.name),
            fees: prices
                .filter((each) => each.currency === "CAD" || each.currency === "USD")
                .map((each) => [each.lookupKey, each.unitAmount, each.includedUsage])
                .toSorted(([left], [right]) => String(left).localeCompare(String(right))),
            prices: prices.length,
            started: started.status,
            refused,
            entitlements: entitlements.map((each) => [
                each.feature,
                each.value ?? each.limit,
                each.state,
            ]),
            derived: derived.items.map((row) => [row.resource, row.limit, row.values]),
        }).toEqual({
            products: ["Free", "Go", "Plus", "Pro"],
            fees: [
                ["free-cad", 0, 0],
                ["free-usd", 0, 0],
                ["go-cad", 1100, 1100],
                ["go-usd", 800, 800],
                ["plus-cad", 2800, 2800],
                ["plus-usd", 2000, 2000],
                ["pro-cad", 14_000, 14_000],
                ["pro-usd", 10_000, 10_000],
            ],
            prices: 4 * 5,
            started: "complete",
            refused: [
                "PRECONDITION_FAILED",
                "this universe takes no payments, so it sells only what charges nothing",
            ],
            entitlements: [
                ["capacityClasses", ["elastic", "machine"], null],
                ["compute", 45_000, "within"],
                ["members", 10, null],
                ["storage", 1_000_000_000, "within"],
                ["support", "community", null],
            ],
            derived: [
                ["capacity", null, ["elastic", "machine"]],
                ["compute", 45_000, null],
                ["membership", 10, null],
                ["storage", 1_000_000_000, null],
            ],
        });
    },
);
