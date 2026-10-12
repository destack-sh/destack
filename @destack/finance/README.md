# @destack/finance

`@destack/finance` is Stripe Billing's `product`, `price`, `customer`, `subscription`, `invoice` and `checkout-session` with Stripe Entitlements' features, `defineMeter` is a Stripe billing meter, `defineSku` and `charge` are FinOps FOCUS's SKU prices and cost rows, and `StripeProvider` bills through Stripe Connect, Checkout and Tax.

```ts
// a Stripe billing meter
defineMeter({ name: "api.requests", description, aggregation: "sum", unit: "request" });
// a Stripe Entitlements feature
defineFeature({
    name: "api.calls",
    description,
    kind: "metered",
    meters: [requests.reference],
    reset: "period",
});
// Stripe's recurring.usage_type
recurring: { interval: "month", intervalCount: 1, usage: "licensed" };
// a Stripe Checkout Session, kept in the residency's home and owned by the buying account
await client.checkoutSession.create({ spaceId: home, accountId, requestId, product, currency: "CHF" });
// FOCUS ServiceName, ServiceCategory, PricingUnit
defineSku({
    name: "cloudflare.r2.storage",
    service: "R2",
    category: "Storage",
    pricingUnit: "GB-month",
    ...price,
});
// a FOCUS row: pricingQuantity, listUnitPrice, listCost, billedCost
Rate.usage(storage, 500e9, true, "1.4");
```

## Products

`ProductDefinition.publish` keeps a seller's product by name in the residency's home with its fixed prices, each with the usage its fee includes, and exactly its feature grants.

```ts
import { createClient } from "@destack/service/client";
import { ProductDefinition } from "@destack/finance/client";
import { financeService } from "@destack/finance/service";

await ProductDefinition.publish(
    createClient(financeService, { url, fetch }),
    home,
    sellerAccountId,
    {
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
        features: [calls.grant(null), support.grant("priority")],
    },
);
```

## SKUs

`defineSku` declares a provider's list price per pricing unit of the usage a meter measures.

```ts
export const storage = defineSku({
    name: "cloudflare.r2.storage",
    description: "R2 standard storage, kept on average over the month.",
    provider: "cloudflare",
    service: "R2",
    category: "Storage",
    meter: bucketStorage.reference,
    pricingUnit: "GB-month",
    unitSize: 1_000_000_000,
    currency: "USD",
    unitAmount: "1.5", // cents
    source: "https://developers.cloudflare.com/r2/pricing/",
    observedAt: "2026-10-06",
});
```

## Usage

A package appends `usage` events in its own scope, and `Metering` rates them past each paying account's slot into its charges and entitlements.

```ts
await events.append(usage, [
    {
        scope: spaceId,
        id,
        time: EventTime.of(from),
        keys: { meter, sku, installation: null, quantity, level },
        data: { unit: "byte", from, to },
    },
]);
// [{ kind: "metered", limit, usage, state: "near", … }]
const { items } = await client.entitlement.list({
    spaceId: home,
    where: { accountId, feature: "storage" },
});
```

## Service

`implementFinance` serves the finance objects in the residency's home over its database and usage events, billing through a `PaymentProvider`.

```ts
import { implementFinance } from "@destack/finance/server";
import { StripeProvider } from "@destack/finance/stripe";

const provider = new StripeProvider({
    key: stripeSecretKey,
    webhookSecret: stripeWebhookSecret,
    fetch,
    pages,
    clock,
});
const finance = implementFinance({
    database,
    accounts,
    installation,
    callKey,
    registry,
    tag: "latest",
    provider,
    events,
    report,
});
```

## Installation

Finance installs in each residency's home: its database and `usage` bucket bind as resources, the `provider` and `tag` settings choose its payment provider and the releases it sells, and it reads releases through the registry its machine mounts.

```ts
install(
    financePackage,
    {
        main: "finance",
        usage: "usage",
        "stripe-secret-key": "stripe-secret-key",
        "stripe-webhook": "stripe-webhook",
    },
    { alias: "finance" },
);
settings: {
    provider: { setting: provider, value: { kind: "stripe", pages }, mode: "set" },
    tag: { setting: tag, value: "dev", mode: "set" },
};
```

## Tests

`FinanceFixture` serves finance over a test database with its home copied, and `StripeFixture` is an in-process Stripe.

```ts
const finance = await FinanceFixture.open("sqlite", [[plan.package, [storage, storageSku]]], {
    provider,
    clock,
    report,
});
const stripe = new StripeFixture(Date.now());
await stripe.deliver([{ type: "invoice.paid", object: stripe.pay(invoiceId) }], endpoint, url);
```
