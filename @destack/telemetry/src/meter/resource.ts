import { defineMeter } from "../declare/meter.ts";
import type { Meter } from "./meter.ts";

/** The directions of an operation on stored data. */
const ACCESS = ["read", "write"];

/** The directions bytes cross a machine's network in. */
const FLOW = ["ingress", "egress"];

/** The CPU time a space's installations spend, in milliseconds. */
export const computeTime = defineMeter({
    name: "compute.time",
    description: "CPU time spent, in milliseconds.",
    aggregation: "sum",
    unit: "CPU millisecond",
    attributes: {},
});

/** The memory a space's installations hold while awake, in GB-seconds. */
export const computeUsage = defineMeter({
    name: "compute.usage",
    description: "Memory held while installations are awake, in GB-seconds.",
    aggregation: "sum",
    unit: "GB-second",
    attributes: {},
});

/** The requests a space's installations serve. */
export const computeOperation = defineMeter({
    name: "compute.operation",
    description: "Requests served.",
    aggregation: "sum",
    unit: "request",
    attributes: {},
});

/** The bytes a space keeps in its databases, averaged over the month. */
export const databaseUsage = defineMeter({
    name: "database.usage",
    description: "Bytes kept in databases, averaged over the month.",
    aggregation: "average",
    unit: "byte",
    attributes: {},
});

/** The database rows a space's installations read and write. */
export const databaseOperation = defineMeter({
    name: "database.operation",
    description: "Database rows read and written, deletes counting as writes.",
    aggregation: "sum",
    unit: "row",
    attributes: { direction: ACCESS },
});

/** The bytes a space keeps in its buckets, telemetry segments included, averaged over the month. */
export const bucketUsage = defineMeter({
    name: "bucket.usage",
    description: "Bytes kept in buckets, averaged over the month.",
    aggregation: "average",
    unit: "byte",
    attributes: {},
});

/** The reads, writes and lists of a space's buckets, lists counting as writes. */
export const bucketOperation = defineMeter({
    name: "bucket.operation",
    description: "Bucket reads and writes, lists counting as writes.",
    aggregation: "sum",
    unit: "operation",
    attributes: { direction: ACCESS },
});

/** The bytes a space's installations receive and send across their machine's network. */
export const networkTransfer = defineMeter({
    name: "network.transfer",
    description: "Bytes received into and sent out of the machine's network.",
    aggregation: "sum",
    unit: "byte",
    attributes: { direction: FLOW },
});

/** The seconds a dedicated machine stays awake, billed per machine at its size's price. */
export const machineTime = defineMeter({
    name: "machine.time",
    description: "Seconds a dedicated machine stays awake.",
    aggregation: "sum",
    unit: "second",
    attributes: {},
});

/** The list cost of the models a space's installations call through Destack, in US dollars. */
export const inferenceCost = defineMeter({
    name: "inference.cost",
    description: "Model calls at their provider's list price, in US dollars.",
    aggregation: "sum",
    unit: "US dollar",
    attributes: {},
});

/** The meters every machine measures of the spaces it serves, by resource. */
export const METERS: readonly Meter[] = [
    // compute
    computeTime,
    computeUsage,
    computeOperation,

    // database
    databaseUsage,
    databaseOperation,

    // bucket
    bucketUsage,
    bucketOperation,

    // network
    networkTransfer,
];
