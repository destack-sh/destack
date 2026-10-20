# @platform/pricing

Declare Destack's plans as products, its units by resource, its price book, the features its plans grant, and the list prices of the vendors it runs on.

## Plans

`PLANS` lists Free, every account's default product, and Go, Plus and Pro at round local fees, 8, 20 and 100 a month in USD, EUR, CHF and GBP, 11, 28 and 140 in CAD, and 12, 30 and 150 in AUD, named and described as messages, each fee including as much usage; Go lifts the caps and adds early access, Plus adds custom domains and email support, and Pro adds dedicated machines and priority support, every plan keeping events for longer.

```ts
import { CREDIT_PRICES, PLANS } from "@platform/pricing";

export const platform = defineStack({
    sellers: { platform: { provider: "stripe", country: "AT", creditPrices: CREDIT_PRICES } },
    products: PLANS, // { free: { name: t`Free`, … }, go, plus, pro }
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

`MACHINES` sizes Destack's dedicated machines, which sleep when idle, at 8, 16, 32 and 64 credit cents per awake hour capped at 40, 80, 160 and 320 credits a month, about twice their share of a Hetzner server, and `MACHINE_SKUS` rate each size's `machine.hours` as usage like every other unit.

```ts
import { MACHINE_HOST, MACHINE_SKUS, MACHINES } from "@platform/pricing";

MACHINES.small; // { vcpus: 2, memory: 4, disk: 80, hourly: 8, monthly: 4000 } credit cents
MACHINE_SKUS.small.definition.unitAmount; // "8" per hour
MACHINE_HOST.quote(MACHINES.small); // { name: "AX42-1", monthly: 1906.51, … }
```

## Units

`UNIT_SKUS` are Destack's SKUs in credits, one per generic meter, each at twice its Cloudflare cost, egress free, priced alike on every host and in every region, each under the resource people see as its FOCUS service: Compute, Database, Storage or Network.

```ts
import { memoryUnit, RESOURCES, requestsUnit, rowsWrittenUnit } from "@platform/pricing";

RESOURCES; // ["Compute", "Database", "Storage", "Network"]
memoryUnit.definition.unitAmount; // "9" per GB-hour
requestsUnit.definition.unitAmount; // "90" per million requests
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
UNIT_PRICING.price(requests.reference, 1_000_000); // 90 US cents
```

## Vendors

`CLOUDFLARE_SKUS` keeps Cloudflare's list prices as FOCUS SKUs in their published pricing units, each on the meter its pricing unit counts, with their sources and the day they were read, and `CLOUDFLARE_PRICING` prices each meter over them, the cost every unit doubles.

```ts
import { CLOUDFLARE_PRICING, r2Storage } from "@platform/pricing/cloudflare";
import { requests } from "@destack/observability/meter";

r2Storage.definition.unitAmount; // "1.5" US cents per GB-month
CLOUDFLARE_PRICING.price(requests.reference, 1_000_000); // 45 US cents
```

### Refresh

`just refresh` reads Cloudflare's docs pages and rewrites the module's amounts and day.

```sh
just refresh
```

### Machine vendors

`hetzner` keeps Hetzner's list prices with their source and the day they were read, and quotes the cheapest share of a server with some resources, the cost every machine cap doubles.

```ts
import { hetzner } from "@platform/pricing/hetzner";

hetzner.quote({ vcpus: 16, memory: 32, disk: 640 }); // { name: "AX102-1", monthly: 15725.83, … }
```

## Features

`database` and `storage` count the bytes kept in databases and buckets, `compute` the GB-seconds and `network` the bytes sent out each period, each an entitlement a machine enforces, `retention` the days events stay, and `tenancy`, `support`, `customDomains` and `earlyAccess` what a plan unlocks.

```ts
import { compute, database, network, retention, tenancy } from "@platform/pricing";

database.grant(1_000_000_000);
network.grant(100_000_000_000);
tenancy.grant(["shared", "dedicated", "own"]);
retention.grant(90);
```
