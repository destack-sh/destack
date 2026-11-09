import { Sku } from "@destack/finance/declare";
import * as meters from "@destack/telemetry/meter";
import { PricingModel } from "../price/model.ts";

/** The package keeping the vendors' list prices as data. */
const OWNER = import.meta.destack.package;

/** The page publishing Durable Objects' prices. */
const DURABLE_OBJECTS = "https://developers.cloudflare.com/durable-objects/platform/pricing/";

/** The page publishing R2's prices. */
const R2 = "https://developers.cloudflare.com/r2/pricing/";

/** The page publishing Workers' prices. */
const WORKERS = "https://developers.cloudflare.com/workers/platform/pricing/";

/** The day the prices were read at their sources. */
const OBSERVED_AT = "2026-10-12";

/** The units in a million, the quantity Cloudflare prices most units per. */
const MILLION = 1_000_000;

/** The bytes in a GB, which Cloudflare takes as 10^9, as its 128 MB / 1 GB example shows. */
const GIGABYTE = 1_000_000_000;

/** The memory Cloudflare bills an active Durable Object at, in GB: 128 MB / 1 GB. */
export const DURABLE_OBJECT_GIGABYTES = 0.128;

/** Durable Objects duration, $12.50 per million GB-seconds. */
export const durableObjectDuration = new Sku(OWNER, {
    name: "cloudflare.durableObjectDuration",
    description: "Durable Objects duration, by wall-clock time at the object's billed memory.",
    provider: "cloudflare",
    service: "Durable Objects",
    category: "Compute",
    meter: meters.computeUsage.reference,
    attributes: {},
    pricingUnit: "million GB-seconds",
    unitSize: MILLION,
    currency: "USD",
    unitAmount: "1250",
    source: DURABLE_OBJECTS,
    observedAt: OBSERVED_AT,
});

/** Durable Objects requests, $0.15 per million. */
export const durableObjectRequests = new Sku(OWNER, {
    name: "cloudflare.durableObjectRequests",
    description: "Durable Objects requests, RPC sessions, WebSocket messages and alarms.",
    provider: "cloudflare",
    service: "Durable Objects",
    category: "Compute",
    meter: meters.computeOperation.reference,
    attributes: {},
    pricingUnit: "million requests",
    unitSize: MILLION,
    currency: "USD",
    unitAmount: "15",
    source: DURABLE_OBJECTS,
    observedAt: OBSERVED_AT,
});

/** Durable Objects SQLite rows read, $0.001 per million. */
export const durableObjectRowsRead = new Sku(OWNER, {
    name: "cloudflare.durableObjectRowsRead",
    description: "Durable Objects SQLite rows read.",
    provider: "cloudflare",
    service: "Durable Objects",
    category: "Databases",
    meter: meters.databaseOperation.reference,
    attributes: { direction: "read" },
    pricingUnit: "million rows read",
    unitSize: MILLION,
    currency: "USD",
    unitAmount: "0.1",
    source: DURABLE_OBJECTS,
    observedAt: OBSERVED_AT,
});

/** Durable Objects SQLite rows written, $1.00 per million. */
export const durableObjectRowsWritten = new Sku(OWNER, {
    name: "cloudflare.durableObjectRowsWritten",
    description: "Durable Objects SQLite rows written.",
    provider: "cloudflare",
    service: "Durable Objects",
    category: "Databases",
    meter: meters.databaseOperation.reference,
    attributes: { direction: "write" },
    pricingUnit: "million rows written",
    unitSize: MILLION,
    currency: "USD",
    unitAmount: "100",
    source: DURABLE_OBJECTS,
    observedAt: OBSERVED_AT,
});

/** Durable Objects SQLite storage, $0.20 per GB-month. */
export const durableObjectStorage = new Sku(OWNER, {
    name: "cloudflare.durableObjectStorage",
    description: "Durable Objects SQLite stored data.",
    provider: "cloudflare",
    service: "Durable Objects",
    category: "Databases",
    meter: meters.databaseUsage.reference,
    attributes: {},
    pricingUnit: "GB-month",
    unitSize: GIGABYTE,
    currency: "USD",
    unitAmount: "20",
    source: DURABLE_OBJECTS,
    observedAt: OBSERVED_AT,
});

/** R2 Standard storage, $0.015 per GB-month. */
export const r2Storage = new Sku(OWNER, {
    name: "cloudflare.r2Storage",
    description: "R2 Standard storage.",
    provider: "cloudflare",
    service: "R2",
    category: "Storage",
    meter: meters.bucketUsage.reference,
    attributes: {},
    pricingUnit: "GB-month",
    unitSize: GIGABYTE,
    currency: "USD",
    unitAmount: "1.5",
    source: R2,
    observedAt: OBSERVED_AT,
});

/** R2 Standard Class A operations, writes and lists, $4.50 per million. */
export const r2ClassA = new Sku(OWNER, {
    name: "cloudflare.r2ClassA",
    description: "R2 Standard Class A operations: writes and lists.",
    provider: "cloudflare",
    service: "R2",
    category: "Storage",
    meter: meters.bucketOperation.reference,
    attributes: { direction: "write" },
    pricingUnit: "million Class A operations",
    unitSize: MILLION,
    currency: "USD",
    unitAmount: "450",
    source: R2,
    observedAt: OBSERVED_AT,
});

/** R2 Standard Class B operations, reads, $0.36 per million. */
export const r2ClassB = new Sku(OWNER, {
    name: "cloudflare.r2ClassB",
    description: "R2 Standard Class B operations: reads.",
    provider: "cloudflare",
    service: "R2",
    category: "Storage",
    meter: meters.bucketOperation.reference,
    attributes: { direction: "read" },
    pricingUnit: "million Class B operations",
    unitSize: MILLION,
    currency: "USD",
    unitAmount: "36",
    source: R2,
    observedAt: OBSERVED_AT,
});

/** R2 egress to the internet, free. */
export const r2Egress = new Sku(OWNER, {
    name: "cloudflare.r2Egress",
    description: "R2 data transfer to the internet.",
    provider: "cloudflare",
    service: "R2",
    category: "Networking",
    meter: meters.networkTransfer.reference,
    attributes: { direction: "egress" },
    pricingUnit: "GB",
    unitSize: GIGABYTE,
    currency: "USD",
    unitAmount: "0",
    source: R2,
    observedAt: OBSERVED_AT,
});

/** Workers Standard requests, $0.30 per million. */
export const workerRequests = new Sku(OWNER, {
    name: "cloudflare.workerRequests",
    description: "Workers Standard requests.",
    provider: "cloudflare",
    service: "Workers",
    category: "Compute",
    meter: meters.computeOperation.reference,
    attributes: {},
    pricingUnit: "million requests",
    unitSize: MILLION,
    currency: "USD",
    unitAmount: "30",
    source: WORKERS,
    observedAt: OBSERVED_AT,
});

/** Workers Standard CPU time, $0.02 per million CPU milliseconds. */
export const workerCpu = new Sku(OWNER, {
    name: "cloudflare.workerCpu",
    description: "Workers Standard CPU time.",
    provider: "cloudflare",
    service: "Workers",
    category: "Compute",
    meter: meters.computeTime.reference,
    attributes: {},
    pricingUnit: "million CPU milliseconds",
    unitSize: MILLION,
    currency: "USD",
    unitAmount: "2",
    source: WORKERS,
    observedAt: OBSERVED_AT,
});

/** Cloudflare's SKUs Destack runs on. */
export const CLOUDFLARE_SKUS: readonly Sku[] = [
    durableObjectDuration,
    durableObjectRequests,
    durableObjectRowsRead,
    durableObjectRowsWritten,
    durableObjectStorage,
    r2Storage,
    r2ClassA,
    r2ClassB,
    r2Egress,
    workerRequests,
    workerCpu,
];

/** Cloudflare's cost per unit of each meter, as the SKUs a machine's work takes. */
export const CLOUDFLARE_PRICING = new PricingModel([
    // compute
    [meters.computeUsage, {}, [[durableObjectDuration, 1]]],
    [meters.computeTime, {}, [[workerCpu, 1]]],

    // take one Worker request and one Durable Object request per request
    [
        meters.computeOperation,
        {},
        [
            [workerRequests, 1],
            [durableObjectRequests, 1],
        ],
    ],

    // database
    [meters.databaseUsage, {}, [[durableObjectStorage, 1]]],
    [meters.databaseOperation, { direction: "read" }, [[durableObjectRowsRead, 1]]],
    [meters.databaseOperation, { direction: "write" }, [[durableObjectRowsWritten, 1]]],

    // bucket
    [meters.bucketUsage, {}, [[r2Storage, 1]]],
    [meters.bucketOperation, { direction: "write" }, [[r2ClassA, 1]]],
    [meters.bucketOperation, { direction: "read" }, [[r2ClassB, 1]]],

    // network: Cloudflare bills neither direction
    [meters.networkTransfer, { direction: "ingress" }, []],
    [meters.networkTransfer, { direction: "egress" }, [[r2Egress, 1]]],
]);
