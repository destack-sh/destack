import { defineMeter } from "@destack/finance/declare";
import type { Meter } from "@destack/finance";

/** The bytes a space keeps in its databases, averaged over the month. */
export const databaseStorage = defineMeter({
    name: "db.bytes",
    description: "Bytes kept in databases, averaged over the month.",
    aggregation: "average",
    unit: "byte",
});

/** The bytes a space keeps in its buckets, telemetry segments included, averaged over the month. */
export const bucketStorage = defineMeter({
    name: "bucket.bytes",
    description: "Bytes kept in buckets, averaged over the month.",
    aggregation: "average",
    unit: "byte",
});

/** The memory a space's installations hold while they run, in GB-seconds, as compute SKUs bill it. */
export const compute = defineMeter({
    name: "compute",
    description: "Memory held while installations run, in GB-seconds.",
    aggregation: "sum",
    unit: "GB-second",
});

/** The bytes a space's installations send out of their provider's network. */
export const egress = defineMeter({
    name: "egress.bytes",
    description: "Bytes sent out of the provider's network.",
    aggregation: "sum",
    unit: "byte",
});

/** The requests and storage reads and writes of a space, each weighted by its SKU, as request units weigh operations. */
export const operations = defineMeter({
    name: "operations",
    description:
        "Requests and storage reads and writes, at their prices in millionths of a credit.",
    aggregation: "sum",
    unit: "microcredit",
});

/** The meters every machine measures of the spaces it serves. */
export const METERS: readonly Meter[] = [
    databaseStorage,
    bucketStorage,
    compute,
    egress,
    operations,
];
