# @destack/finance

Sell products with features, bill accounts for subscriptions and usage, and derive their entitlements.

## Overview

`@destack/finance` is Stripe Billing driven by declarations: features packages sell, products stacks sell, and entitlements derived from subscriptions and usage.

Packages declare what they sell:

- `defineFeature`: a feature products grant: access, a fixed value, or metered usage up to a limit.
- `defineSku`: a provider's list price per unit of what a meter measures.

Stacks sell products:

- `defineCatalog`: products with recurring prices and feature grants.

Finance bills accounts:

- `subscription`: an account's purchase of a product.
- `checkout-session`: a provider's payment page for one purchase.
- `charge`: usage rated into money at the paying account's home.
- `PaymentProvider`: the provider finance bills through, such as Stripe.

Declare a meter and two features, and sell a product granting them:

```ts
import { defineCatalog, defineFeature } from "@destack/finance/declare";
import { t } from "@destack/locale";
import { schema } from "@destack/schema";
import { defineMeter } from "@destack/telemetry/declare";

// declare a meter of API requests
export const requests = defineMeter({
    name: "api.requests",
    description: t`Requests to the API.`,
    aggregation: "sum",
    unit: "request",
    attributes: {},
});

// declare the features products grant
export const calls = defineFeature({
    name: "api.calls",
    description: t`API requests per billing period.`,
    kind: "metered",
    meters: [requests],
    reset: "period",
});
export const support = defineFeature({
    name: "support",
    description: t`The support tier.`,
    kind: "static",
    value: schema.enum(["community", "priority"]),
});

// sell a product granting both
export const catalog = defineCatalog({
    go: {
        name: t`Go`,
        description: t`More room to build.`,
        isDefault: false,
        prices: {
            eur: {
                currency: "EUR",
                unitAmount: 900,
                includedUsage: 900,
                recurring: { interval: "month", intervalCount: 1, usage: "licensed" },
            },
        },
        productFeatures: { calls: calls.grant(10_000), support: support.grant("priority") },
    },
});
```

## Reference

### defineFeature

`defineFeature({ name, description, kind, … })` declares a feature, and `grant(value)` sets a product's value of it.

| Kind        | What it grants                 | Fields                                     |
| ----------- | ------------------------------ | ------------------------------------------ |
| `"boolean"` | access alone                   |                                            |
| `"static"`  | a fixed value                  | `value`: the value's schema                |
| `"metered"` | usage up to a limit per period | `meters`, `reset`: `"period"` or `"never"` |

### defineSku

`defineSku(definition)` declares a list price, in minor units per `pricingUnit`, with its `source` and `observedAt`.

### defineCatalog

`defineCatalog(products)` declares a stack's products by name.
Each has a `name`, `description`, `isDefault`, `prices` by key with `currency`, `unitAmount` and `recurring`, and `productFeatures` as grants.

## Examples

### Price a SKU

A SKU prices what a meter measures, such as R2 storage.

```ts
import { defineSku } from "@destack/finance/declare";
import { t } from "@destack/locale";
import { defineMeter } from "@destack/telemetry/declare";

// declare a meter of stored bytes
const stored = defineMeter({
    name: "storage.bytes",
    description: t`Bytes stored, averaged over the month.`,
    aggregation: "average",
    unit: "byte",
    attributes: {},
});

// price R2 storage per GB-month
export const storage = defineSku({
    name: "cloudflare.r2.storage",
    description: t`R2 standard storage, averaged over the month.`,
    provider: "cloudflare",
    service: "R2",
    category: "Storage",
    meter: stored,
    attributes: {},
    pricingUnit: "GB-month",
    unitSize: 1_000_000_000,
    currency: "USD",
    unitAmount: "1.5",
    source: "https://developers.cloudflare.com/r2/pricing/",
    observedAt: "2026-10-06",
});
```

### Check an entitlement

An account's entitlements are `entitlement` objects of `@destack/account`, one per feature.

```ts
import type accountPackage from "@destack/account/package";
import type { ClientOf } from "@destack/object/client";
import type { PackageId } from "@destack/package";

/** Read the account's limit of API calls, absent when unlimited. */
export async function callLimit(
    client: ClientOf<typeof accountPackage>,
    accountId: string,
    packageId: PackageId,
) {
    // read the account's entitlement to the calls feature
    const where = { packageId, feature: "api.calls" };
    const { items } = await client.entitlement.list({ accountId, where });

    return items[0]?.limit;
}
```

### Start a checkout

A checkout session sends the buyer to the provider's payment page, and the subscription follows once they paid.

```ts
import type financePackage from "@destack/finance/package";
import type { ClientOf } from "@destack/object/client";
import type { Identifier } from "@destack/schema";
import type { ObjectReference } from "@destack/sync";

/** Open a checkout for a product, answering the payment page. */
export async function checkout(
    client: ClientOf<typeof financePackage>,
    spaceId: Identifier<"space">,
    accountId: Identifier<"account">,
    product: ObjectReference,
    requestId: string,
) {
    return await client.checkoutSession.create({
        spaceId,
        accountId,
        requestId,
        productPackageId: product.packageId,
        productType: product.type,
        productScope: product.scope,
        productId: product.id,
        currency: "EUR",
        successUrl: "https://example.com/paid",
        cancelUrl: "https://example.com/plans",
    });
}
```

### Test billing

`TestFinance` runs finance over a test database, with Stripe in process.

```ts
import { TestFinance } from "@destack/finance/test";

// open finance, derive the entitlements, and close it
const finance = await TestFinance.open([]);
await finance.entitle();
await finance.close();
```

## Comparison

### Stripe Billing

Products, prices, subscriptions and checkout follow Stripe Billing.

| Stripe Billing      | `@destack/finance`         | Difference                                                   |
| ------------------- | -------------------------- | ------------------------------------------------------------ |
| products and prices | `defineCatalog`            | Declared in a stack.                                         |
| entitlements        | features                   | Packages declare features, and products grant them.          |
| meters              | `defineMeter`, `defineSku` | Usage is rated at the paying account's home, at list prices. |
