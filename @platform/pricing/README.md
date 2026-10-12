# @platform/pricing

Declare Destack's plans as products, its units by resource, its price book, the features its plans grant, and its vendors' list prices.

## Plans

`PLANS` lists Free, every account's default product, and Go, Plus and Pro at round local fees, 8, 20 and 100 a month in USD, EUR, CHF and GBP and 11, 28 and 140 in CAD, named and described as messages, each fee including as much usage, Plus and Pro unlocking dedicated machines, which bill as usage.

```ts
import { CREDIT_PRICES, PLANS } from "@platform/pricing";

export const platform = defineStack({
    sellers: { platform: { provider: "stripe", country: "AT", creditPrices: CREDIT_PRICES } },
    products: PLANS, // { free: { name: t`Free`, description: t`Meet Destack.`, … }, go, plus, pro }
});
```

## Price book

`CREDIT_PRICES` prices a credit in each currency Destack bills, kept by the platform seller, and `Fee.conversion` reads a list price's currency in US cents.

```ts
import { CREDIT_PRICES, Fee } from "@platform/pricing";

CREDIT_PRICES.CAD; // "1.4"
Fee.conversion("x-credits"); // "1"
```

## Machines

`MACHINES` sizes Destack's dedicated machines, which sleep when idle, at 5, 10, 20 and 40 credit cents per awake hour capped at 25, 50, 100 and 200 credits a month, and `MACHINE_SKUS` rate each size's `machine.hours` as usage like every other unit; `just compare` prints each price and cap beside its share of a Hetzner server and the equivalent on Render, Fly.io and Railway.

```ts
import { MACHINE_COMPARABLES, MACHINE_HOST, MACHINE_SKUS, MACHINES } from "@platform/pricing";

MACHINES.small; // { vcpus: 2, memory: 4, disk: 80, hourly: 5, monthly: 2500 } credit cents
MACHINE_SKUS.small.definition.unitAmount; // "5" per hour
MACHINE_HOST.quote(MACHINES.small); // { name: "AX42-1", monthly: 1906.51, … }
MACHINE_COMPARABLES.map((vendor) => vendor.quote(MACHINES.small)?.monthly); // [10500, 7800, 9200]
```

## Units

`UNIT_SKUS` are Destack's SKUs in credits, one per generic meter, priced alike on every host and in every region, each under the resource people see as its FOCUS service: Compute, Database, Storage or Network.

```ts
import { memoryUnit, RESOURCES, requestsUnit, rowsWrittenUnit } from "@platform/pricing";

RESOURCES; // ["Compute", "Database", "Storage", "Network"]
memoryUnit.definition.unitAmount; // "6" per GB-hour
requestsUnit.definition.unitAmount; // "60" per million requests
rowsWrittenUnit.definition.service; // "Database"
```

### Pricing models

`PricingModel` keeps a provider's SKUs and their quantities per unit of each generic meter, and `PricingModel.price` prices a meter's quantity in US cents; `UNIT_PRICING` is Destack's, each meter at its one unit.

```ts
import { UNIT_PRICING } from "@platform/pricing";
import { CLOUDFLARE_PRICING } from "@platform/pricing/cloudflare";
import { requests } from "@destack/observability/meter";

CLOUDFLARE_PRICING.components(requests.reference); // [[workerRequests, 1], [durableObjectRequests, 1]]
CLOUDFLARE_PRICING.price(requests.reference, 1_000_000); // 45 US cents
UNIT_PRICING.price(requests.reference, 1_000_000); // 60 US cents
```

### Typical app

`TYPICAL_APP` prices a typical app's month at the units, 128 MB busy two hours a day for 10,000 requests a day, for the pricing page.

```ts
import { TYPICAL_APP } from "@platform/pricing";

TYPICAL_APP; // 123 US cents
```

### Workloads

`WORKLOADS` keeps a month of each resource, which `Workload.price` prices at a pricing model: at the units, at Cloudflare's cost and at each hyperscaler.

```ts
import { UNIT_PRICING, VENDOR_PRICING, WORKLOADS } from "@platform/pricing";

WORKLOADS.Database.price(UNIT_PRICING); // 151.25 US cents
WORKLOADS.Database.price(VENDOR_PRICING.aws); // 212.5 US cents at DynamoDB
```

## Vendors

`VENDOR_SKUS` keeps each cloud's list prices as FOCUS SKUs in their published pricing units, as data the package's catalog leaves out, each on the meter its pricing unit counts, with their sources and the day they were read, one module per vendor; `VENDOR_PRICING` keeps each vendor's pricing model over them.

```ts
import { DURABLE_OBJECT_GIGABYTES, r2Storage } from "@platform/pricing/cloudflare";
import { AWS_PRICING, dynamodbWrites } from "@platform/pricing/aws";
import { rowsWritten } from "@destack/observability/meter";

r2Storage.definition.unitAmount; // "1.5" US cents per GB-month
dynamodbWrites.definition.meter; // rowsWritten
AWS_PRICING.components(rowsWritten.reference); // [[dynamodbWrites, 1]]
```

### Machine vendors

`hetzner`, `render`, `fly` and `railway` keep each machine vendor's list prices with their sources and the day they were read, and quote the cheapest machine with some resources.

```ts
import { hetzner } from "@platform/pricing/hetzner";

hetzner.quote({ vcpus: 16, memory: 32, disk: 640 }); // { name: "AX102-1", monthly: 15725.83, … }
```

### Refresh

`just refresh` reads every vendor's catalog, the first paid tier of each SKU, and rewrites the modules' amounts and days; Google's catalog takes `GOOGLE_CLOUD_API_KEY`.

```sh
just refresh aws azure cloudflare
GOOGLE_CLOUD_API_KEY=… just refresh gcp
```

### Compare

`just compare` prints each unit's price beside its Cloudflare cost and margin and each hyperscaler's price on its meter, then each machine's hourly price and monthly cap beside its host's cost and its comparables' prices, then each resource's month at every vendor pricing it.

```sh
just compare
# Compute  destack.compute.requests                                      million requests  0.600000
#          cloudflare.workerRequests + cloudflare.durableObjectRequests  million requests  0.450000  cloudflare  +25%
#          aws.lambdaRequests                                            million requests  0.200000  aws         -67%
```

## Features

`database` and `storage` count the bytes kept in databases and buckets, `compute` the GB-seconds and `network` the bytes sent out each period, each an allowance a machine enforces, and `tenancy`, `support` and `customDomains` what a plan unlocks.

```ts
import { compute, database, network, tenancy } from "@platform/pricing";

database.grant(1_000_000_000);
network.grant(100_000_000_000);
tenancy.grant(["shared", "dedicated", "own"]);
```
