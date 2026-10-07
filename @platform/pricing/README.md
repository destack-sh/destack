# @platform/pricing

Declare Destack's plans as products, the units and operations it sells usage by in credits with their cost basis, its price book, and the features its plans grant.

## Plans

`PLANS` lists Free, Go, Plus and Pro at 0, 8, 20 and 100 a month as finance products, priced in USD, CAD, EUR, CHF and GBP, each fee including as much usage.

```ts
import { ProductDefinition } from "@destack/finance/client";
import { PLANS } from "@platform/pricing";

for (const plan of PLANS) {
    await ProductDefinition.publish(finance, platformAccountId, plan);
}
```

## Price book

`CREDIT_PRICES` prices a credit in each currency Destack bills, kept by the platform seller.

```ts
import { CREDIT_PRICES } from "@platform/pricing";

await finance.seller.create({ accountId, requestId, country: "AT", creditPrices: CREDIT_PRICES });
CREDIT_PRICES.CAD; // "1.4"
```

## Units

`UNIT_SKUS` are Destack's SKUs in credits, one per platform meter, priced alike on every host and in every region.

```ts
import { computeUnit, databaseStorageUnit, UNIT_SKUS } from "@platform/pricing";

databaseStorageUnit.unitAmount; // "25" hundredths of a credit per GB-month
computeUnit.unitAmount; // "6" per GB-hour
```

### Operations

`OPERATION_PRICES` prices each operation a cell counts in millionths of a credit, which the `operations` meter adds up.

```ts
import { Reading } from "@destack/observability/meter";
import { OPERATION_PRICES } from "@platform/pricing";

Reading.operations(requests, OPERATION_PRICES.workerRequest); // 0.375 each
```

### Cost basis

`UNIT_COSTS` and `OPERATION_COSTS` name the provider cost each unit's and operation's price is set 20 to 25 percent above.

```ts
import { OPERATION_COSTS, UNIT_COSTS } from "@platform/pricing";

UNIT_COSTS[0]; // { unit: databaseStorageUnit, cost: { provider: "cloudflare", service: "Durable Objects", pricingUnit: "GB-month", unitAmount: "20", … } }
OPERATION_COSTS[0]; // { operation: "workerRequest", cost: { service: "Workers", pricingUnit: "request", unitAmount: "0.00003", … } }
```

## Cloudflare

`CLOUDFLARE_COSTS` keeps Cloudflare's list prices with their sources, in US cents per pricing unit, one operation per unit.

```ts
import { CLOUDFLARE_COSTS, DURABLE_OBJECT_GIGABYTES } from "@platform/pricing/cloudflare";

CLOUDFLARE_COSTS.durableObjectRowsWritten.unitAmount; // "0.0001"
const readings = await Reading.compute(records, from, to, DURABLE_OBJECT_GIGABYTES);
```

## Features

`storage` counts the bytes kept in databases and buckets together, `compute` the GB-seconds run each period, and `capacityClasses`, `support`, `customDomains` and `members` what a plan unlocks.

```ts
import { capacityClasses, compute, members, storage } from "@platform/pricing";

storage.grant(1_000_000_000);
capacityClasses.grant(["elastic", "reserved", "machine"]);
members.grant(null); // unlimited
```
