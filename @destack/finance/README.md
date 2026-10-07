# @destack/finance

Sell products with features, rate usage at SKU list prices into charges, bill accounts through a payment provider, and derive their entitlements.

## Declarations

`defineFeature` declares what a product grants: access, a static value or metered usage up to a limit.

```ts
import { defineFeature, defineMeter } from "@destack/finance/declare";
import { schema } from "@destack/schema";

export const requests = defineMeter({
    name: "api.requests",
    description: "Requests to the storage API.",
    aggregation: "sum",
    unit: "request",
});
export const sync = defineFeature({ name: "sync", description: "Sync files.", kind: "boolean" });
export const support = defineFeature({
    name: "support",
    description: "The support tier.",
    kind: "static",
    value: schema.enum(["community", "priority"]),
});
export const calls = defineFeature({
    name: "api.calls",
    description: "API requests per billing period.",
    kind: "metered",
    meters: [requests.reference],
    reset: "period",
});
```

### SKUs

`defineSku` declares a provider's list price per pricing unit of the usage a meter measures, named after FOCUS.

```ts
import { defineSku } from "@destack/finance/declare";
import { bucketStorage } from "@destack/observability/meter";

export const storage = defineSku({
    name: "cloudflare.r2.storage",
    description: "R2 standard storage, kept on average over the month.",
    provider: "cloudflare",
    service: "R2", // FOCUS ServiceName
    category: "Storage", // FOCUS ServiceCategory
    meter: bucketStorage.reference,
    pricingUnit: "GB-month", // FOCUS PricingUnit
    unitSize: 1_000_000_000,
    currency: "USD",
    unitAmount: "1.5", // cents
    source: "https://developers.cloudflare.com/r2/pricing/",
    observedAt: "2026-10-06",
});
```

## Objects

Each finance object lives in a seller's or a buyer's account, and `@destack/finance/object` exports their declarations.

```ts
import {
    charge,
    checkoutSession,
    customer,
    entitlement,
    invoice,
    price,
    product,
    seller,
    subscription,
} from "@destack/finance/object";
```

## Selling

`ProductDefinition.publish` keeps a seller's product by name with its fixed prices, each with the usage its fee includes, and exactly its feature grants.

```ts
import { connect, ProductDefinition } from "@destack/finance/client";

await ProductDefinition.publish(connect({ url, fetch }), sellerAccountId, {
    name: "Go",
    description: "9 a month of usage at list prices, and more as you use it.",
    prices: [
        {
            lookupKey: "go-eur",
            currency: "EUR",
            unitAmount: 900,
            includedUsage: 900,
            recurring: { interval: "month", intervalCount: 1, usage: "licensed" },
        },
    ],
    features: [calls.grant(null), support.grant("priority")], // unlimited calls
});
```

### Metered prices

`Charge.meter` rates a seller's own `metered` price into one usage line per period, billed at the close beside a licensed fee of at least 0.

```ts
await client.price.create({
    accountId,
    requestId: RequestId.create(),
    parentId: pro.id,
    currency: "EUR",
    unitAmountDecimal: "0.5", // cents
    recurring: { interval: "month", intervalCount: 1, usage: "metered", meter: requests.reference },
});
```

## Rating

`Rate.usage` prices a quantity of a SKU's meter at its list price converted into the billing currency and rounded up, billed within a subscription the provider bills.

```ts
import { Rate } from "@destack/finance";

const rated = Rate.usage(storage, 500e9, true, "1.4");
// { pricingQuantity: 500, listUnitPrice: "2.1", listCost: "1050", billedCost: "1050", skuPriceId: "…@2026-10-06" }
```

### Price book

`seller.creditPrices` prices a credit in each currency the seller bills, so SKUs listed in credits (`x-credits`) rate in each payer's own currency.

```ts
await client.seller.create({
    accountId,
    requestId,
    country: "AT",
    creditPrices: { USD: "1", CAD: "1.4", EUR: "1" },
});
```

### Charges

A `charge` is one FOCUS-shaped line per account, period, resource and SKU, rated again as each use of the SKU arrives.

```ts
const { items } = await client.charge.list({
    accountId,
    where: { resourceId: `/spaces/${spaceId}` },
});
// [{ chargeCategory: "Usage", skuId, skuMeter: "bucket.bytes", consumedQuantity, pricingQuantity: 2.5,
//    pricingUnit: "GB-month", listUnitPrice: "1.5", listCost: "3.75", billedCost: "3.75", status: "open", … }]
```

### Sellers

`Seller.publisher` reads the seller of a SKU's package, whose subscription the provider bills bills the buyer's usage, and a line without one bills 0 at its list cost.

```ts
const seller = await Seller.publisher(database, sku.package); // the platform account's seller
```

### Default products

A product marked `isDefault`, without prices, grants its features to every account holding no subscription to its seller, its metered usage counted by calendar month.

```ts
await ProductDefinition.publish(finance, sellerAccountId, {
    name: "Free",
    description,
    isDefault: true,
    prices: [],
    features,
});
```

## Closing

`subscription.closePeriod` adds the period's usage lines and the included usage drawn on them to the provider's draft invoice of the renewal.

```ts
await call.invoke(subscription).closePeriod({ id, periodStart, periodEnd, invoice });
// invoice lines: usage +1200, included usage -900
```

## Checkout

A `checkout-session` subscribes its account to a product's recurring prices in one currency.

```ts
const session = await client.checkoutSession.create({
    accountId,
    requestId: RequestId.create(),
    product: { scope: sellerAccountId, id: productId },
    currency: "CHF",
});
// { status: "open", url }
```

### Plan changes

`subscription.change` moves a subscription to another product's prices at the provider, or for the seller's default product closes its period onto the final invoice and cancels it.

```ts
await client.subscription.change({
    accountId,
    requestId: RequestId.create(),
    id: subscriptionId,
    product: { scope: sellerAccountId, id: freeId },
    currency: "CHF",
});
```

## Billing

`subscription.sync` moves a subscription to the `ProviderSubscription` its provider reports, refusing a transition its state forbids.

```ts
await call.invoke(subscription).sync(reported); // status "past_due"
```

## Offers

A subscription item copies its price's `terms` and its product's `grants` when created, and a subscription the usage its prices include (`includedUsage`).

```ts
const { terms, grants } = await Price.offer(database, { scope: shopId, id: priceId });
```

## Entitlements

`EntitlementController` derives one `entitlement` per grant of each granting subscription item and default product, with its usage and its state.

```ts
const { items } = await client.entitlement.list({ accountId, where: { feature: "storage" } });
// [{ kind: "metered", limit: 1000000000, usage: 830000000, state: "near", resetAt: null, … }]
```

### Usage limits

`usageLimit` tells the account's readers of billing once a period when an entitlement's state moves to near, over or blocked.

```ts
import { usageLimit } from "@destack/finance/object";

usageLimit.payload; // { feature, state, usage, limit }
```

### Spending limits

A subscription's `spendingLimit` caps what its usage may cost past its included usage each period, starting at 0 and unlimited at null, and a line past it bills 0 at its list cost.

```ts
await client.subscription.update({ accountId, requestId, id, spendingLimit: 5000 });
```

## Resolution

`Entitlement.resolve` combines the entitlements of one feature across sources.

```ts
const storage = Entitlement.resolve(items, { packageId, name: "storage" });
// { kind: "metered", limit: 1000000000, usage: 830000000, resetAt: null, state: "near" }
```

## Usage

A package measuring a use appends a `usage` event in its own scope, which routes a copy to the paying account's finance, whose `admitUsage` admits the workloads serving the account.

```ts
import { usage } from "@destack/finance/declare";

await events.append(usage, [
    {
        scope: spaceId,
        id: `${meter}:${spaceId}:${to}`,
        time: from * 1000,
        keys: {
            meter: "package-…/db.bytes",
            sku: "package-…/destack.storage.database",
            installation: null,
            quantity: 830000000 * (HOUR / MONTH),
            level: 830000000,
        },
        data: { unit: "byte", from, to },
    },
]);
```

### Rating

`UsageController` rates each account's usage past its `UsageReader` slot into charges, entitlements and allowances.

```ts
const after = await UsageReader.of(accountId).after(database);
await new MeterUsage(events).read(accountId, meter, { start, end }); // 830000000
```

## Cells

`entitlementShape` copies an account's entitlements to the cells serving its spaces, decided for the cell where they live.

```ts
entitlementShape.subscription({
    name: entitlementShape.copy(accountId),
    scope: accountId,
    below: accountId,
    parameters: {},
});
```

### Allowances

A feature declaring an `allowance` derives the account's `allowance` of [@destack/account](../account/README.md) on that resource from its entitlements.

```ts
defineFeature({ name: "storage", kind: "metered", allowance: "storage", meters, reset: "never" });
```

## Payers

`Customer.payer` returns the account that pays an account's charges.

```ts
const paying = await Customer.payer(database, accountId);
```

## Providers

`PaymentProvider` is the contract finance bills through: sellers' connected accounts, customers, licensed prices, checkouts, plan changes, invoice lines and webhooks.

```ts
import type { PaymentProvider } from "@destack/finance/provider";
```

### Stripe

`StripeProvider` bills through Stripe: Connect Express sellers paid by destination charges, licensed subscription items, usage as invoice items, Checkout taxed by Stripe Tax, and signed webhooks at `/finance/webhooks/stripe`.

```ts
import { StripeProvider } from "@destack/finance/stripe";

const provider = new StripeProvider({
    key: stripeSecretKey,
    webhookSecret: stripeWebhookSecret,
    fetch,
    pages: {
        successUrl: "https://destack.app/-/billing",
        cancelUrl: "https://destack.app/-/plans",
    },
});
```

## Service

`implementFinance` serves the finance objects over a residency's database.

```ts
import { implementFinance } from "@destack/finance/server";

const finance = implementFinance({
    database,
    identity,
    callKey,
    registry: Registry.of(directory, fetch),
    tag: "latest",
    provider,
    report,
});
```

### Catalogs

`Catalog.tagged` reads the features, meters and SKUs of each package's release under a tag, which finance checks grants, events and charges against.

```ts
import { Catalog } from "@destack/finance";

const catalog = Catalog.tagged(Registry.of(directory, fetch), "latest");
const sku = (await catalog(packageId)).sku(reference);
```

## Client

`connect` returns a client of the finance service's objects.

```ts
import { connect } from "@destack/finance/client";

const finance = connect({ url, headers: { authorization } });
await finance.subscription.list({ accountId });
```

## Workload

`financeWorkload` serves the finance objects once per residency, billing through the provider its process binds.

```ts
import { Registry } from "@destack/forge/client";
import { LATEST_TAG } from "@destack/forge/object";
import { financeConfiguration, financeWorkload } from "@destack/finance/workload";

resources.bind(financeConfiguration, {
    registry: Registry.of(directory, fetch),
    tag: LATEST_TAG,
    provider,
});
```

## Tables

`financeDatabase` holds `financeTables` and copies of the residency's accounts and organisations, and of the machines and keys its callers' tokens name.

```ts
import { financeDatabase, financeTables } from "@destack/finance/stack";
```

## Tests

`FinanceFixture` serves finance over a test database to people and workloads by name, and `StripeFixture` is an in-process Stripe after stripe-mock.

```ts
import { FinanceFixture, StripeFixture } from "@destack/finance/test";

const finance = await FinanceFixture.open("sqlite", [[plan.package, [storage, storageSku]]], {
    provider,
    clock,
});
await finance.account(accountId, "acme", ownerId);
finance.workload("cell-eu");
const stripe = new StripeFixture(Date.now());
await stripe.deliver(stripe.renew(subscriptionId), endpoint, url);
await stripe.deliver([{ type: "invoice.paid", object: stripe.pay(invoiceId) }], endpoint, url);
```
