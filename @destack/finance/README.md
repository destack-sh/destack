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

A stack declares a seller's products by key, each with its name and description as messages, its recurring prices, each including as much usage as its fee, and its feature grants.

```ts
import { t } from "@destack/locale";

export const shop = defineStack({
    sellers: { platform: { provider: "stripe", country: "AT", creditPrices: { EUR: "1" } } },
    products: {
        go: {
            name: t`Go`,
            description: t`More room to build.`,
            prices: {
                eur: {
                    currency: "EUR",
                    unitAmount: 900,
                    includedUsage: 900,
                    recurring: { interval: "month", intervalCount: 1, usage: "licensed" },
                },
            },
            productFeatures: { calls: calls.grant(null), support: support.grant("priority") },
        },
    },
});
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
    meter: storageBytes.reference,
    pricingUnit: "GB-month",
    unitSize: 1_000_000_000,
    currency: "USD",
    unitAmount: "1.5", // cents
    source: "https://developers.cloudflare.com/r2/pricing/",
    observedAt: "2026-10-06",
});
```

`maximumQuantity` caps the pricing units each resource pays for per billing period, as hourly machines capped at a monthly price bill.

```ts
// 5 credit-cents an awake hour, at most 500 hours a period for each machine
defineSku({
    name: "destack.machine.small",
    pricingUnit: "hour",
    unitAmount: "5",
    maximumQuantity: 500,
    ...price,
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
