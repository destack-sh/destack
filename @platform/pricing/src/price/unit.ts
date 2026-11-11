import { CREDITS, type Sku } from "@destack/finance/declare";
import { defineSku } from "@destack/finance/declare";
import * as meters from "@destack/telemetry/meter";
import type { Meter } from "@destack/telemetry/meter";
import { PricingModel } from "./model.ts";
import { t } from "@destack/locale";

/** The page publishing Destack's unit prices with their cost basis. */
const SOURCE = "https://destack.sh/pricing";

/** The day the unit prices were set. */
const OBSERVED_AT = "2026-11-09";

/** The units in a million, the quantity operations are priced per. */
const MILLION = 1_000_000;

/** The bytes in a GB, taken as 10^9 as Cloudflare and AWS take it. */
const GIGABYTE = 1_000_000_000;

/** The GB-seconds in a GB-hour, the unit memory is priced per. */
const GIGABYTE_HOUR_SECONDS = 3600;

/** The resources people see Destack's usage as, each unit's FOCUS service. */
export const RESOURCES = ["Compute", "Database", "Files", "Network", "Inference"] as const;

/** A resource people see Destack's usage as. */
export type Resource = (typeof RESOURCES)[number];

/** The fields every unit shares: Destack sells it in credits, published on the pricing page. */
const UNIT = {
    provider: "destack",
    currency: CREDITS,
    source: SOURCE,
    observedAt: OBSERVED_AT,
} as const;

/** CPU time, 0.04 credits per million CPU milliseconds, double its cost on Cloudflare. */
export const computeTimeUnit = defineSku({
    ...UNIT,
    name: "destack.compute.time",
    description: t`CPU time installations spend.`,
    service: "Compute",
    category: "Compute",
    meter: meters.computeTime,
    attributes: {},
    pricingUnit: "million CPU milliseconds",
    unitSize: MILLION,
    unitAmount: "4",
});

/** Memory, 0.09 credits per GB-hour held while awake, double its cost on Cloudflare. */
export const computeUsageUnit = defineSku({
    ...UNIT,
    name: "destack.compute.usage",
    description: t`Memory held while installations are awake.`,
    service: "Compute",
    category: "Compute",
    meter: meters.computeUsage,
    attributes: {},
    pricingUnit: "GB-hour",
    unitSize: GIGABYTE_HOUR_SECONDS,
    unitAmount: "9",
});

/** Requests, 0.90 credits per million, double their cost on Cloudflare. */
export const computeOperationUnit = defineSku({
    ...UNIT,
    name: "destack.compute.operation",
    description: t`Requests served.`,
    service: "Compute",
    category: "Compute",
    meter: meters.computeOperation,
    attributes: {},
    pricingUnit: "million requests",
    unitSize: MILLION,
    unitAmount: "90",
});

/** Database bytes, 0.40 credits per GB-month, double their cost on Cloudflare. */
export const databaseUsageUnit = defineSku({
    ...UNIT,
    name: "destack.database.usage",
    description: t`Database storage, kept on average over the month.`,
    service: "Database",
    category: "Databases",
    meter: meters.databaseUsage,
    attributes: {},
    pricingUnit: "GB-month",
    unitSize: GIGABYTE,
    unitAmount: "40",
});

/** Database rows read, 0.002 credits per million, double their cost on Cloudflare. */
export const databaseReadUnit = defineSku({
    ...UNIT,
    name: "destack.database.read",
    description: t`Database rows read, every row a query scans.`,
    service: "Database",
    category: "Databases",
    meter: meters.databaseOperation,
    attributes: { direction: "read" },
    pricingUnit: "million rows",
    unitSize: MILLION,
    unitAmount: "0.2",
});

/** Database rows written, 2 credits per million, double their cost on Cloudflare. */
export const databaseWriteUnit = defineSku({
    ...UNIT,
    name: "destack.database.write",
    description: t`Database rows written, deletes included.`,
    service: "Database",
    category: "Databases",
    meter: meters.databaseOperation,
    attributes: { direction: "write" },
    pricingUnit: "million rows",
    unitSize: MILLION,
    unitAmount: "200",
});

/** File bytes, 0.03 credits per GB-month, double their cost on Cloudflare. */
export const bucketUsageUnit = defineSku({
    ...UNIT,
    name: "destack.bucket.usage",
    description: t`File storage, kept on average over the month.`,
    service: "Files",
    category: "Storage",
    meter: meters.bucketUsage,
    attributes: {},
    pricingUnit: "GB-month",
    unitSize: GIGABYTE,
    unitAmount: "3",
});

/** File reads, 0.72 credits per million, double their cost on Cloudflare. */
export const bucketReadUnit = defineSku({
    ...UNIT,
    name: "destack.bucket.read",
    description: t`File reads.`,
    service: "Files",
    category: "Storage",
    meter: meters.bucketOperation,
    attributes: { direction: "read" },
    pricingUnit: "million operations",
    unitSize: MILLION,
    unitAmount: "72",
});

/** File writes and lists, 9 credits per million, double their cost on Cloudflare. */
export const bucketWriteUnit = defineSku({
    ...UNIT,
    name: "destack.bucket.write",
    description: t`File writes and lists.`,
    service: "Files",
    category: "Storage",
    meter: meters.bucketOperation,
    attributes: { direction: "write" },
    pricingUnit: "million operations",
    unitSize: MILLION,
    unitAmount: "900",
});

/** Ingress, free. */
export const networkIngressUnit = defineSku({
    ...UNIT,
    name: "destack.network.ingress",
    description: t`Bytes received into Destack.`,
    service: "Network",
    category: "Networking",
    meter: meters.networkTransfer,
    attributes: { direction: "ingress" },
    pricingUnit: "GB",
    unitSize: GIGABYTE,
    unitAmount: "0",
});

/** Egress, free. */
export const networkEgressUnit = defineSku({
    ...UNIT,
    name: "destack.network.egress",
    description: t`Bytes sent out of Destack.`,
    service: "Network",
    category: "Networking",
    meter: meters.networkTransfer,
    attributes: { direction: "egress" },
    pricingUnit: "GB",
    unitSize: GIGABYTE,
    unitAmount: "0",
});

/** Model calls, 1.25 credits per US dollar of their provider's list price. */
export const inferenceCostUnit = defineSku({
    ...UNIT,
    name: "destack.inference.cost",
    description: t`Model calls through Destack, at their provider's list price.`,
    service: "Inference",
    category: "AI and Machine Learning",
    meter: meters.inferenceCost,
    attributes: {},
    pricingUnit: "US dollar",
    unitSize: 1,
    unitAmount: "125",
});

/** Each unit beside the meter it prices, by resource. */
const UNITS: readonly (readonly [Meter, Sku])[] = [
    // compute
    [meters.computeTime, computeTimeUnit],
    [meters.computeUsage, computeUsageUnit],
    [meters.computeOperation, computeOperationUnit],

    // database
    [meters.databaseUsage, databaseUsageUnit],
    [meters.databaseOperation, databaseReadUnit],
    [meters.databaseOperation, databaseWriteUnit],

    // bucket
    [meters.bucketUsage, bucketUsageUnit],
    [meters.bucketOperation, bucketReadUnit],
    [meters.bucketOperation, bucketWriteUnit],

    // network
    [meters.networkTransfer, networkIngressUnit],
    [meters.networkTransfer, networkEgressUnit],

    // inference
    [meters.inferenceCost, inferenceCostUnit],
];

/** The units every cloud machine rates its spaces' usage by, one per meter and attribute values. */
export const UNIT_SKUS: readonly Sku[] = UNITS.map(([, sku]) => sku);

/** Destack's pricing model, each meter at its one unit. */
export const UNIT_PRICING = new PricingModel(
    UNITS.map(([meter, sku]) => [meter, sku.definition.attributes, [[sku, 1]]]),
);
