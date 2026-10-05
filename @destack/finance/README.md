# @destack/finance

Sell products with features, bill accounts for subscriptions and purchases, and derive their entitlements.

## Declarations

`defineFeature` declares what a product grants: access, a static value or metered usage up to a limit.

```ts
import { defineFeature, defineMeter } from "@destack/finance/declare";
import { schema } from "@destack/schema";

export const requests = defineMeter({
    name: "api.requests",
    description: "Requests to the storage API.",
    aggregation: "sum", // or count, max, last
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
    meter: requests.reference,
    reset: "period", // or never
});
```

## Objects

Each finance object lives in a seller's or a buyer's account.

```text
account
├── customer { email, name, address?, taxIds, providerId? }                       one per account
├── seller { provider, providerId?, status, chargesEnabled, payoutsEnabled, country }   one per account
├── product { name, description, active }
│   ├── product-feature { packageId, feature, value? }
│   └── price { currency, unitAmount?, type, recurring?, billingScheme, tiers?, lookupKey?, active }
├── subscription { seller, status, currentPeriodStart, currentPeriodEnd, cancelAtPeriodEnd, trialEnd?, canceledAt?, providerId? }
│   └── subscription-item { price, quantity, terms, grants }
├── purchase { seller, price, quantity, terms, grants, amount, status, paidAt?, providerId? }
├── entitlement { packageId, feature, kind, value?, limit?, usage?, resetAt?, source, sourceId }
├── meter-event { eventId, source, packageId, meter, value, time }               unique per source and eventId
└── invoice { number, status, currency, subtotal, tax, total, amountPaid, periodStart?, periodEnd?, hostedUrl?, pdfUrl?, issuedAt, providerId }
```

## Selling

A `product-feature` grants a feature of one of the seller's packages to the buyers of a product.

```ts
const pro = await client.product.create({
    accountId,
    requestId: RequestId.create(),
    name: "Pro",
    description,
});
await client.price.create({
    accountId,
    requestId: RequestId.create(),
    parentId: pro.id,
    currency: "EUR", // ISO 4217, or x-<custom> such as x-credits
    unitAmount: 900, // minor units
    type: "recurring",
    recurring: { interval: "month", intervalCount: 1, usage: "licensed" },
});
for (const [feature, value] of [
    [calls, 1000],
    [sync, null], // access alone
] as const) {
    await client.productFeature.create({
        accountId,
        requestId: RequestId.create(),
        parentId: pro.id,
        packageId: feature.package.id,
        feature: feature.name,
        value,
    });
}
// a metered grant without a value is unlimited, and its usage still counts
```

## Billing

The payment provider creates the subscriptions and purchases of an account's `customer`.

```text
subscription  trial      incomplete                                      -> trialing
              activate   incomplete, trialing, past_due, unpaid, paused  -> active
              fail       trialing, active                                -> past_due
              lapse      past_due                                        -> unpaid
              pause      trialing, active                                -> paused
              expire     incomplete                                      -> incomplete_expired
              cancel     every state but incomplete_expired and canceled  -> canceled, at canceledAt
purchase      pay        pending                                         -> paid, at paidAt
              fail       pending                                         -> failed
              refund     paid                                            -> refunded
```

## Snapshots

A subscription item or purchase copies its price's `terms` and its product's `grants` when created.

```ts
const { terms, grants } = await Price.snapshot(database, { scope: shopId, id: priceId });
await server.executeAsSystem(
    subscriptionItem,
    "create",
    [
        {
            scope: accountId,
            input: { parentId, price: { scope: shopId, id: priceId }, terms, grants },
        },
    ],
    Date.now(),
);
```

## Entitlements

The customer's controller derives one `entitlement` per grant of each active subscription and paid purchase.

```ts
const { items } = await client.entitlement.list({ accountId, where: { feature: "api.calls" } });
// [{ kind: "metered", limit: 1000, usage: 30, resetAt: currentPeriodEnd, source: "subscription", ... }]
```

## Resolution

`Entitlement.resolve` combines the entitlements of one feature across sources.

```ts
const calls = Entitlement.resolve(items, { packageId, name: "api.calls" });
// { kind: "metered", limit: 1500, usage: 30, resetAt: currentPeriodEnd }, or null without a grant
```

## Meter events

A `meter-event` is unique per CloudEvents `source` and `eventId`.

```ts
await server.executeAsSystem(
    meterEvent,
    "create",
    [
        {
            scope: accountId,
            input: { eventId: event.id, source: event.source, packageId, meter, value, time },
        },
    ],
    Date.now(),
);
```

## Payers

`Customer.payer` returns the account that pays an account's charges.

```ts
const paying = await Customer.payer(database, accountId);
```

## Service

`implementFinance` serves the finance objects over a residency's database.

```ts
import { implementFinance } from "@destack/finance/server";

const finance = implementFinance({ database, identity, callKey, releases, report });
```

## Client

`connect` returns a client of the finance service's objects.

```ts
import { connect } from "@destack/finance/client";

const finance = connect({ url, headers: { authorization } });
await finance.customer.list({ accountId });
```

## Workload

`financeWorkload` serves the finance objects once per residency.

```ts
import { Registry } from "@destack/forge/client";
import { financeConfiguration, financeWorkload } from "@destack/finance/workload";

resources.bind(financeConfiguration, {
    registry: Registry.of(directory, fetch),
});
```

## Tables

`financeDatabase` holds `financeTables` and copies of the residency's accounts and organisations.

```ts
import { financeDatabase, financeTables } from "@destack/finance/stack";
```
