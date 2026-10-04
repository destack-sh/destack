# @destack/finance

Sell products with features, bill accounts for subscriptions and purchases, and derive their entitlements.

## Objects

Every object lives in an account: a seller's account keeps its products, and a buyer's account keeps its customer, subscriptions, purchases, entitlements, meter events and invoices.

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

## Features

`defineFeature` declares what a product grants (access alone, a fixed value of a schema, or usage of a meter up to a limit), and later releases keep each feature's kind.

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

## Selling

A seller's `product-feature` grants a declared feature of a package `@<handle>/…` its account publishes, with a value its declaration accepts, and its prices bill the product once or each period.

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

The provider creates subscriptions of recurring prices and purchases of one-time prices for an account with a `customer` and moves them through their states as the system, and the account's managers hold `bill` over the rest.

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

A subscription item or purchase keeps its active price's `terms` and its active product's `grants` from its creation on, so derivation never reads the seller's rows, which may live in another residency.

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

The customer's controller derives one `entitlement` per grant of each item of a trialing, active or past due subscription and of each paid purchase, and counts metered usage from `meter-event`s within the subscription's current period, while a purchase grants only boolean and static features.

```ts
const { items } = await client.entitlement.list({ accountId, where: { feature: "api.calls" } });
// [{ kind: "metered", limit: 1000, usage: 30, resetAt: currentPeriodEnd, source: "subscription", ... }]
```

## Resolution

`Entitlement.resolve` combines a feature's rows across sources: any source grants access, the highest number or else the latest value wins, and metered limits add up, unlimited when any source is, against the usage within the period of the source resetting first.

```ts
const calls = Entitlement.resolve(items, { packageId, name: "api.calls" });
// { kind: "metered", limit: 1500, usage: 30, resetAt: currentPeriodEnd }, or null without a grant
```

## Meter events

A `meter-event` is unique per CloudEvents `source` and `eventId`, so a repeat is refused as a duplicate, and each event updates only the usage of the entitlements on its meter.

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

`Customer.payer` reads the account paying an account's charges from the copies: its organisation's `payer`, or else the account itself.

```ts
const paying = await Customer.payer(database, accountId);
```

## Service

`financeWorkload` serves the objects once per residency over `financeDatabase`, which copies the residency's accounts and organisations.

```ts
import { financeConfiguration, financeWorkload } from "@destack/finance/workload";

resources.bind(financeConfiguration, { release: (packageId) => openRelease(packageId) });
```
