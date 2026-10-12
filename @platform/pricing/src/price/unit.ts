import { CREDITS, type Sku } from "@destack/finance";
import { defineSku } from "@destack/finance/declare";
import * as meters from "@destack/observability/meter";
import { PricingModel } from "./model.ts";

/** The page publishing Destack's unit prices with their cost basis. */
const SOURCE = "https://destack.sh/pricing";

/** The day the unit prices were set. */
const OBSERVED_AT = "2026-10-12";

/** The units in a million, the quantity operations are priced per. */
const MILLION = 1_000_000;

/** The bytes in a GB, taken as 10^9 as Cloudflare and AWS take it. */
const GIGABYTE = 1_000_000_000;

/** The GB-seconds in a GB-hour, the unit memory is priced per. */
const GIGABYTE_HOUR_SECONDS = 3600;

/** The resources people see Destack's usage as, each unit's FOCUS service. */
export const RESOURCES = ["Compute", "Database", "Storage", "Network"] as const;

/** A resource people see Destack's usage as. */
export type Resource = (typeof RESOURCES)[number];

/** Memory, 0.06 credits per GB-hour held. */
export const memoryUnit = defineSku({
    name: "destack.compute.memory",
    description: "Memory held while installations run.",
    provider: "destack",
    service: "Compute",
    category: "Compute",
    meter: meters.memory.reference,
    pricingUnit: "GB-hour",
    unitSize: GIGABYTE_HOUR_SECONDS,
    currency: CREDITS,
    unitAmount: "6",
    source: SOURCE,
    observedAt: OBSERVED_AT,
});

/** CPU time, 0.03 credits per million CPU milliseconds. */
export const cpuUnit = defineSku({
    name: "destack.compute.cpu",
    description: "CPU time spent on requests.",
    provider: "destack",
    service: "Compute",
    category: "Compute",
    meter: meters.cpu.reference,
    pricingUnit: "million CPU milliseconds",
    unitSize: MILLION,
    currency: CREDITS,
    unitAmount: "3",
    source: SOURCE,
    observedAt: OBSERVED_AT,
});

/** Requests, 0.60 credits per million. */
export const requestsUnit = defineSku({
    name: "destack.compute.requests",
    description: "Requests served.",
    provider: "destack",
    service: "Compute",
    category: "Compute",
    meter: meters.requests.reference,
    pricingUnit: "million requests",
    unitSize: MILLION,
    currency: CREDITS,
    unitAmount: "60",
    source: SOURCE,
    observedAt: OBSERVED_AT,
});

/** Database bytes, 0.25 credits per GB-month. */
export const databaseBytesUnit = defineSku({
    name: "destack.database.bytes",
    description: "Database storage, kept on average over the month.",
    provider: "destack",
    service: "Database",
    category: "Databases",
    meter: meters.databaseBytes.reference,
    pricingUnit: "GB-month",
    unitSize: GIGABYTE,
    currency: CREDITS,
    unitAmount: "25",
    source: SOURCE,
    observedAt: OBSERVED_AT,
});

/** Database rows read, 0.00125 credits per million. */
export const rowsReadUnit = defineSku({
    name: "destack.database.rows.read",
    description: "Database rows read.",
    provider: "destack",
    service: "Database",
    category: "Databases",
    meter: meters.rowsRead.reference,
    pricingUnit: "million rows",
    unitSize: MILLION,
    currency: CREDITS,
    unitAmount: "0.125",
    source: SOURCE,
    observedAt: OBSERVED_AT,
});

/** Database rows written, 1.25 credits per million. */
export const rowsWrittenUnit = defineSku({
    name: "destack.database.rows.written",
    description: "Database rows written.",
    provider: "destack",
    service: "Database",
    category: "Databases",
    meter: meters.rowsWritten.reference,
    pricingUnit: "million rows",
    unitSize: MILLION,
    currency: CREDITS,
    unitAmount: "125",
    source: SOURCE,
    observedAt: OBSERVED_AT,
});

/** Storage bytes, 0.02 credits per GB-month. */
export const storageBytesUnit = defineSku({
    name: "destack.storage.bytes",
    description: "Bucket storage, kept on average over the month.",
    provider: "destack",
    service: "Storage",
    category: "Storage",
    meter: meters.storageBytes.reference,
    pricingUnit: "GB-month",
    unitSize: GIGABYTE,
    currency: CREDITS,
    unitAmount: "2",
    source: SOURCE,
    observedAt: OBSERVED_AT,
});

/** Storage writes and lists, 6 credits per million. */
export const storageWritesUnit = defineSku({
    name: "destack.storage.writes",
    description: "Bucket writes and lists.",
    provider: "destack",
    service: "Storage",
    category: "Storage",
    meter: meters.storageWrites.reference,
    pricingUnit: "million operations",
    unitSize: MILLION,
    currency: CREDITS,
    unitAmount: "600",
    source: SOURCE,
    observedAt: OBSERVED_AT,
});

/** Storage reads, 0.50 credits per million. */
export const storageReadsUnit = defineSku({
    name: "destack.storage.reads",
    description: "Bucket reads.",
    provider: "destack",
    service: "Storage",
    category: "Storage",
    meter: meters.storageReads.reference,
    pricingUnit: "million operations",
    unitSize: MILLION,
    currency: CREDITS,
    unitAmount: "50",
    source: SOURCE,
    observedAt: OBSERVED_AT,
});

/** Egress, free. */
export const egressUnit = defineSku({
    name: "destack.network.egress",
    description: "Bytes served out of Destack.",
    provider: "destack",
    service: "Network",
    category: "Networking",
    meter: meters.egress.reference,
    pricingUnit: "GB",
    unitSize: GIGABYTE,
    currency: CREDITS,
    unitAmount: "0",
    source: SOURCE,
    observedAt: OBSERVED_AT,
});

/** The units every cloud machine rates its spaces' usage by, one per meter, by resource. */
export const UNIT_SKUS: readonly Sku[] = [
    // compute
    memoryUnit,
    cpuUnit,
    requestsUnit,

    // database
    databaseBytesUnit,
    rowsReadUnit,
    rowsWrittenUnit,

    // storage
    storageBytesUnit,
    storageWritesUnit,
    storageReadsUnit,

    // network
    egressUnit,
];

/** Destack's pricing model, each meter at its one unit. */
export const UNIT_PRICING = new PricingModel(
    UNIT_SKUS.map((sku) => [sku.definition.meter, [[sku, 1]]]),
);
