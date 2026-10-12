import { defineMeter } from "@destack/finance/declare";
import type { Meter } from "@destack/finance";

/** The memory a space's installations keep while they run, in GB-seconds. */
export const memory = defineMeter({
    name: "compute.memory",
    description: "Memory held while installations run, in GB-seconds.",
    aggregation: "sum",
    unit: "GB-second",
});

/** The CPU time a space's installations spend on requests, in milliseconds. */
export const cpu = defineMeter({
    name: "compute.cpu",
    description: "CPU time spent on requests, in milliseconds.",
    aggregation: "sum",
    unit: "CPU millisecond",
});

/** The requests a space's installations serve. */
export const requests = defineMeter({
    name: "compute.requests",
    description: "Requests served.",
    aggregation: "sum",
    unit: "request",
});

/** The bytes a space keeps in its databases, averaged over the month. */
export const databaseBytes = defineMeter({
    name: "database.bytes",
    description: "Bytes kept in databases, averaged over the month.",
    aggregation: "average",
    unit: "byte",
});

/** The database rows a space's installations read. */
export const rowsRead = defineMeter({
    name: "database.rows.read",
    description: "Database rows read.",
    aggregation: "sum",
    unit: "row",
});

/** The database rows a space's installations write. */
export const rowsWritten = defineMeter({
    name: "database.rows.written",
    description: "Database rows written.",
    aggregation: "sum",
    unit: "row",
});

/** The bytes a space keeps in its buckets, telemetry segments included, averaged over the month. */
export const storageBytes = defineMeter({
    name: "storage.bytes",
    description: "Bytes kept in buckets, averaged over the month.",
    aggregation: "average",
    unit: "byte",
});

/** The writes and lists of a space's buckets. */
export const storageWrites = defineMeter({
    name: "storage.writes",
    description: "Bucket writes and lists.",
    aggregation: "sum",
    unit: "operation",
});

/** The reads of a space's buckets. */
export const storageReads = defineMeter({
    name: "storage.reads",
    description: "Bucket reads.",
    aggregation: "sum",
    unit: "operation",
});

/** The bytes a space's installations send out of their machine's network. */
export const egress = defineMeter({
    name: "network.egress",
    description: "Bytes sent out of the machine's network.",
    aggregation: "sum",
    unit: "byte",
});

/** The hours a dedicated machine stays awake, billed per machine. */
export const machineHours = defineMeter({
    name: "machine.hours",
    description: "Hours a dedicated machine stays awake.",
    aggregation: "sum",
    unit: "hour",
});

/** The meters every machine measures of the spaces it serves, by resource. */
export const METERS: readonly Meter[] = [
    // compute
    memory,
    cpu,
    requests,

    // database
    databaseBytes,
    rowsRead,
    rowsWritten,

    // storage
    storageBytes,
    storageWrites,
    storageReads,

    // network
    egress,
];
