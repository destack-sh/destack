# @platform/pricing

Destack's plans, units, price book and plan features, and the list prices of the vendors it runs on.

## Plans

`PLANS` declares Free, Go, Plus and Pro as products, each fee a month per member including the same amount of usage, Free a monthly usage per member, and Free and Go capped at their usage.

```ts
import { CREDIT_PRICES, PLANS } from "@platform/pricing";

export const platform = defineStack({
    sellers: { platform: { provider: "stripe", country: "AT", creditPrices: CREDIT_PRICES } },
    products: PLANS, // { free: { name: t`Free`, … }, go, plus, pro }
});

// free: every account's default, 2.50 credits of usage per member a month, email support
// go:   8 USD, EUR, CHF or GBP, 11 CAD, 13 AUD per member a month, capped: custom retention, early access
// plus: 20, 28 or 32 per member a month, then pay as you go: custom domains
// pro:  50, 70 or 80 per member a month: dedicated machines, priority support
```

## Enterprise

`ENTERPRISE` declares the contract organisations sign past Pro: all of Pro, uptime and support SLAs, and a monthly commitment spent on usage.

```ts
import { ENTERPRISE } from "@platform/pricing";

ENTERPRISE.commitment.USD; // 100_000, 1000 USD a month
ENTERPRISE.support; // "sla"
```

## Price book

`CREDIT_PRICES` prices a credit in each billing currency, and `Fee.conversion` converts a list price's currency to US cents.

```ts
import { CREDIT_PRICES, Fee } from "@platform/pricing";

CREDIT_PRICES.CAD; // "1.4"
Fee.conversion("x-credits"); // "1"
```

## Machines

`MACHINES` sizes the dedicated machines, each billed per awake second through `MACHINE_SKUS` at an hourly price that makes 500 awake hours about twice its share of a Hetzner server.

```ts
import { MACHINE_HOST, MACHINE_SKUS, MACHINES } from "@platform/pricing";

MACHINES.small; // { vcpus: 2, memory: 4, disk: 80, hourly: 8 } credit cents
MACHINE_SKUS.small.definition.unitAmount; // "8" per hour of 3600 seconds
MACHINE_HOST.quote(MACHINES.small); // { name: "AX42-1", monthly: 1906.51, … }
```

## Units

`UNIT_SKUS` prices each meter at each of its attribute values in credits at twice its Cloudflare cost, the same on every host and in every region, transfer free both ways, and inference at 1.25 credits per US dollar of its provider's list price.

```ts
import { computeUsageUnit, databaseWriteUnit, inferenceCostUnit, RESOURCES } from "@platform/pricing";

RESOURCES; // ["Compute", "Database", "Files", "Network", "Inference"]
computeUsageUnit.definition.unitAmount; // "9" per GB-hour
databaseWriteUnit.definition.attributes; // { direction: "write" }
databaseWriteUnit.meterKey; // "<telemetry>/database.operation direction=write"
inferenceCostUnit.definition.unitAmount; // "125" per US dollar
```

### Pricing models

`PricingModel` maps each meter at its attribute values to a provider's SKUs, none where the provider bills nothing, and `price` rates a quantity in US cents.

```ts
import { UNIT_PRICING } from "@platform/pricing";
import { CLOUDFLARE_PRICING } from "@platform/pricing/cloudflare";
import { computeOperation } from "@destack/telemetry/meter";

const requests = computeOperation.key({});
CLOUDFLARE_PRICING.components(requests); // [[workerRequests, 1], [durableObjectRequests, 1]]
CLOUDFLARE_PRICING.price(requests, 1_000_000); // 45 US cents
UNIT_PRICING.price(requests, 1_000_000); // 90 US cents
```

## Vendors

`CLOUDFLARE_SKUS` lists Cloudflare's published prices as FOCUS SKUs with their sources, and `CLOUDFLARE_PRICING` prices each meter with them.

```ts
import { CLOUDFLARE_PRICING, r2Storage } from "@platform/pricing/cloudflare";
import { networkTransfer } from "@destack/telemetry/meter";

r2Storage.definition.unitAmount; // "1.5" US cents per GB-month
CLOUDFLARE_PRICING.price(networkTransfer.key({ direction: "ingress" }), 1e12); // 0 US cents
```

### Refresh

`just refresh` reads Cloudflare's docs pages and rewrites the module's prices and date.

```sh
just refresh
```

### Machine vendors

`hetzner` lists Hetzner's prices and quotes the cheapest share of a server with the given resources.

```ts
import { hetzner } from "@platform/pricing/hetzner";

hetzner.quote({ vcpus: 16, memory: 32, disk: 640 }); // { name: "AX102-1", monthly: 15725.83, … }
```

## Features

Each feature is an entitlement a plan grants: the database, bucket, compute and network its spaces use, where they may run, and what a plan unlocks, such as custom retention past each setting's default.

```ts
import { bucket, customDomains, database, tenancy } from "@platform/pricing";

database.grant(null); // unlimited, within the plan's included usage or its spending limit
bucket.grant(null);
tenancy.grant(["shared", "dedicated", "local"]);
customDomains.grant(null);
```
