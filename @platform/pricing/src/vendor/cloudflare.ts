import type { ProviderCost } from "../plan/plan.ts";

/** The page publishing Durable Objects' prices. */
const DURABLE_OBJECTS = "https://developers.cloudflare.com/durable-objects/platform/pricing/";

/** The page publishing R2's prices. */
const R2 = "https://developers.cloudflare.com/r2/pricing/";

/** The page publishing Workers' prices. */
const WORKERS = "https://developers.cloudflare.com/workers/platform/pricing/";

/** The day the prices were read at their sources. */
const OBSERVED_AT = "2026-10-06";

/** The memory Cloudflare bills every active Durable Object at, in gigabytes. */
export const DURABLE_OBJECT_GIGABYTES = 0.125;

/** Cloudflare's list prices Destack's units and operation prices are set from and its bill is reconciled against, in fractional US cents per pricing unit. */
export const CLOUDFLARE_COSTS = {
    /** Durable Objects duration, $12.50 per million GB-seconds. */
    durableObjectDuration: {
        provider: "cloudflare",
        service: "Durable Objects",
        pricingUnit: "GB-second",
        unitSize: 1,
        unitAmount: "0.00125",
        source: DURABLE_OBJECTS,
        observedAt: OBSERVED_AT,
    },
    /** Durable Objects SQLite storage, $0.20 per GB-month, a GB taken as 10^9 bytes. */
    durableObjectStorage: {
        provider: "cloudflare",
        service: "Durable Objects",
        pricingUnit: "GB-month",
        unitSize: 1_000_000_000,
        unitAmount: "20",
        source: DURABLE_OBJECTS,
        observedAt: OBSERVED_AT,
    },
    /** Durable Objects SQLite rows read, $0.001 per million. */
    durableObjectRowsRead: {
        provider: "cloudflare",
        service: "Durable Objects",
        pricingUnit: "row read",
        unitSize: 1,
        unitAmount: "0.0000001",
        source: DURABLE_OBJECTS,
        observedAt: OBSERVED_AT,
    },
    /** Durable Objects SQLite rows written, $1.00 per million. */
    durableObjectRowsWritten: {
        provider: "cloudflare",
        service: "Durable Objects",
        pricingUnit: "row written",
        unitSize: 1,
        unitAmount: "0.0001",
        source: DURABLE_OBJECTS,
        observedAt: OBSERVED_AT,
    },
    /** Durable Objects requests, $0.15 per million. */
    durableObjectRequests: {
        provider: "cloudflare",
        service: "Durable Objects",
        pricingUnit: "request",
        unitSize: 1,
        unitAmount: "0.000015",
        source: DURABLE_OBJECTS,
        observedAt: OBSERVED_AT,
    },
    /** R2 standard storage, $0.015 per GB-month. */
    r2Storage: {
        provider: "cloudflare",
        service: "R2",
        pricingUnit: "GB-month",
        unitSize: 1_000_000_000,
        unitAmount: "1.5",
        source: R2,
        observedAt: OBSERVED_AT,
    },
    /** R2 Class A operations, writes and lists, $4.50 per million. */
    r2ClassA: {
        provider: "cloudflare",
        service: "R2",
        pricingUnit: "Class A operation",
        unitSize: 1,
        unitAmount: "0.00045",
        source: R2,
        observedAt: OBSERVED_AT,
    },
    /** R2 Class B operations, reads, $0.36 per million. */
    r2ClassB: {
        provider: "cloudflare",
        service: "R2",
        pricingUnit: "Class B operation",
        unitSize: 1,
        unitAmount: "0.000036",
        source: R2,
        observedAt: OBSERVED_AT,
    },
    /** R2 egress to the internet, free. */
    r2Egress: {
        provider: "cloudflare",
        service: "R2",
        pricingUnit: "GB",
        unitSize: 1_000_000_000,
        unitAmount: "0",
        source: R2,
        observedAt: OBSERVED_AT,
    },
    /** Workers requests, $0.30 per million. */
    workerRequests: {
        provider: "cloudflare",
        service: "Workers",
        pricingUnit: "request",
        unitSize: 1,
        unitAmount: "0.00003",
        source: WORKERS,
        observedAt: OBSERVED_AT,
    },
} as const satisfies Readonly<Record<string, ProviderCost>>;
