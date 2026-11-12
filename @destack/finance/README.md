# @destack/finance

Sell products with features, bill accounts for subscriptions and usage, and derive their entitlements.

## Overview

A package declares the features it sells and the meters that measure them, and a stack sells products that grant those features through a payment provider.

- **Feature**: what a product grants: access alone, a fixed value or metered usage up to a limit; `grant` sets a product's value.
- **SKU**: a provider's list price per unit of what a meter measures, such as R2 storage per GB-month.
- **Product**: what a seller sells, with recurring prices and feature grants; `defineCatalog` declares products in a stack.
- **Subscription**: an account's purchase of a product, created through a checkout session and billed by the provider.
- **Charge**: usage rated into money at the home of the account paying for it, which entitlements follow.

```ts
import { defineCatalog, defineFeature } from "@destack/finance/declare";
import { t } from "@destack/locale";
import { schema } from "@destack/schema";
import { defineMeter } from "@destack/telemetry/declare";

export const requests = defineMeter({
    name: "api.requests",
    description: t`Requests to the API.`,
    aggregation: "sum",
    unit: "request",
    attributes: {},
});
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

## Examples

### Install finance

Finance installs in each residency's home with its Stripe key as a secret.

```sh
destack install @destack/finance
```

### Price a SKU

`defineSku` declares a provider's list price, in minor units, per pricing unit of what a meter measures.

```ts
import { defineSku } from "@destack/finance/declare";
import { t } from "@destack/locale";
import { defineMeter } from "@destack/telemetry/declare";

const stored = defineMeter({
    name: "storage.bytes",
    description: t`Bytes stored, averaged over the month.`,
    aggregation: "average",
    unit: "byte",
    attributes: {},
});
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

### Start a checkout

A checkout session sends the buyer to the provider's payment page, and the subscription follows once they paid.

```ts
import type financePackage from "@destack/finance/package";
import type { ClientOf } from "@destack/object/client";
import type { Identifier } from "@destack/schema";
import type { ObjectReference } from "@destack/sync";

async (
    client: ClientOf<typeof financePackage>,
    spaceId: Identifier<"space">,
    accountId: Identifier<"account">,
    product: ObjectReference,
    requestId: string,
) => {
    const session = await client.checkoutSession.create({
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
};
```

### Test billing

`TestFinance` runs finance over a test database, with each package published as a forge release and Stripe in process.

```ts
import { TestFinance } from "@destack/finance/test";

const finance = await TestFinance.open([]);
await finance.entitle();
await finance.close();
```

## Comparison

### Stripe Billing

Products, prices, subscriptions and checkout follow Stripe Billing, which finance mirrors; three things differ.

- A package declares the features it sells and the meters that measure them, so a product grants features and never hard-codes limits.
- Usage is rated at the home of the account paying for it, after the SKUs' list prices.
- Entitlements are derived from subscriptions and usage, and every package reads them the same way.
