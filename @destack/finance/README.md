# @destack/finance

`@destack/finance` sells products with features, bills accounts for subscriptions and usage through a payment provider, and derives what each account may use.

```ts
defineFeature({
    name: "api.calls",
    description,
    kind: "metered",
    meters: [requests.reference],
    reset: "period",
});
await client.checkoutSession.create({
    spaceId: home,
    accountId,
    requestId,
    product,
    currency: "EUR",
});
const { items } = await client.entitlement.list({ spaceId: home, where: { accountId } });
```

## Products

A stack declares a seller and its products, each with recurring prices and feature grants.

```ts
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

`defineSku` declares a provider's list price per pricing unit of what a meter measures, optionally capped per billing period with `maximumQuantity`.

```ts
export const storage = defineSku({
    name: "cloudflare.r2.storage",
    description: "R2 standard storage, averaged over the month.",
    provider: "cloudflare",
    service: "R2",
    category: "Storage",
    meter: storageBytes.reference,
    pricingUnit: "GB-month",
    unitSize: 1_000_000_000,
    currency: "USD",
    unitAmount: "1.5",
    source: "https://developers.cloudflare.com/r2/pricing/",
    observedAt: "2026-10-06",
});
```

## Usage

A space's machine appends `usage` at the home of the account paying for it, and `Metering` rates it into charges and entitlements.

```ts
await finance.usage.append({
    accountId: payer,
    requestId,
    items: [
        { id, time, meter, sku, spaceId, machineId: null, installationId, quantity, level, data },
    ],
});
await finance.usage.series({
    accountId: payer,
    where: { spaceId, meter },
    fold: "sum",
    measure: "quantity",
    group: [],
    from,
    before,
});
```

## Server

Finance's server runs in each residency's home: it reads its `provider` and `tag` settings and its Stripe key, finds sellers through the account package, and resolves catalogs through forge.

```ts
import financeServer from "@destack/finance/server";

const extension = financeServer(context); // one installation's server, built from its context
```

## Installation

Finance installs in each residency's home with its Stripe key as a secret, and its `provider` and `tag` settings.

```ts
install(financePackage, { "stripe-secret-key": "stripe-secret-key" }, { name: "finance" });
settings: {
    provider: { setting: provider, value: "stripe", mode: "set" },
    tag: { setting: tag, value: "dev", mode: "set" },
};
```

## Tests

`FinanceFixture` runs finance over a test database with each package published as a forge release, and `StripeFixture` is an in-process Stripe.

```ts
const finance = await FinanceFixture.open("sqlite", [[plan.package, [storage, storageSku]]], {
    provider,
    clock,
});
await finance.append(payer, [{ id, space, meter, quantity, time }]);
await finance.publish(plan.package, [storage, storageSku]); // a later release, now the latest
const stripe = new StripeFixture(Date.now());
```
