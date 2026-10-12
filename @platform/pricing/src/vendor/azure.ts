import { Sku } from "@destack/finance";
import * as meters from "@destack/observability/meter";
import { PricingModel } from "../price/model.ts";

/** The package keeping the vendors' list prices as data. */
const OWNER = import.meta.destack.package;

/** The page publishing Functions' prices. */
const FUNCTIONS = "https://azure.microsoft.com/en-us/pricing/details/functions/";

/** The page publishing Cosmos DB serverless prices. */
const COSMOS_DB = "https://azure.microsoft.com/en-us/pricing/details/cosmos-db/serverless/";

/** The page publishing Blob Storage's prices. */
const BLOBS = "https://azure.microsoft.com/en-us/pricing/details/storage/blobs/";

/** The page publishing bandwidth prices. */
const BANDWIDTH = "https://azure.microsoft.com/en-us/pricing/details/bandwidth/";

/** The day the prices were read at their sources. */
const OBSERVED_AT = "2026-10-12";

/** The region the prices apply in: East US. */
const REGION = "eastus";

/** The bytes in a GB, which Azure publishes without a definition, taken as 10^9. */
const GIGABYTE = 1_000_000_000;

/** The executions Functions prices per. */
const EXECUTIONS = 10;

/** The operations Blob Storage prices per. */
const OPERATIONS = 10_000;

/** Functions Consumption plan execution time, $0.000016 per GB-second. */
export const functionsDuration = new Sku(OWNER, {
    name: "azure.functionsDuration",
    description: "Functions Consumption plan execution time.",
    provider: "azure",
    region: REGION,
    service: "Functions",
    category: "Compute",
    meter: meters.memory.reference,
    pricingUnit: "GB-second",
    unitSize: 1,
    currency: "USD",
    unitAmount: "0.0016",
    source: FUNCTIONS,
    observedAt: OBSERVED_AT,
});

/** Functions Consumption plan executions, $0.000002 per 10. */
export const functionsExecutions = new Sku(OWNER, {
    name: "azure.functionsExecutions",
    description: "Functions Consumption plan executions.",
    provider: "azure",
    region: REGION,
    service: "Functions",
    category: "Compute",
    meter: meters.requests.reference,
    pricingUnit: "10 executions",
    unitSize: EXECUTIONS,
    currency: "USD",
    unitAmount: "0.0002",
    source: FUNCTIONS,
    observedAt: OBSERVED_AT,
});

/** Functions Flex Consumption on-demand execution time, $0.000026 per GB-second. */
export const flexDuration = new Sku(OWNER, {
    name: "azure.flexDuration",
    description: "Functions Flex Consumption on-demand execution time.",
    provider: "azure",
    region: REGION,
    service: "Functions",
    category: "Compute",
    meter: meters.memory.reference,
    pricingUnit: "GB-second",
    unitSize: 1,
    currency: "USD",
    unitAmount: "0.0026",
    source: FUNCTIONS,
    observedAt: OBSERVED_AT,
});

/** Functions Flex Consumption on-demand executions, $0.000004 per 10. */
export const flexExecutions = new Sku(OWNER, {
    name: "azure.flexExecutions",
    description: "Functions Flex Consumption on-demand executions.",
    provider: "azure",
    region: REGION,
    service: "Functions",
    category: "Compute",
    meter: meters.requests.reference,
    pricingUnit: "10 executions",
    unitSize: EXECUTIONS,
    currency: "USD",
    unitAmount: "0.0004",
    source: FUNCTIONS,
    observedAt: OBSERVED_AT,
});

/** Cosmos DB data stored, $0.25 per GB-month. */
export const cosmosStorage = new Sku(OWNER, {
    name: "azure.cosmosStorage",
    description: "Cosmos DB for NoSQL data stored.",
    provider: "azure",
    region: REGION,
    service: "Azure Cosmos DB",
    category: "Databases",
    meter: meters.databaseBytes.reference,
    pricingUnit: "GB-month",
    unitSize: GIGABYTE,
    currency: "USD",
    unitAmount: "25",
    source: COSMOS_DB,
    observedAt: OBSERVED_AT,
});

/** Blob Storage Hot LRS data stored, the first 50 TB, $0.0208 per GB-month. */
export const blobStorage = new Sku(OWNER, {
    name: "azure.blobStorage",
    description: "Blob Storage General Purpose v2 Hot LRS data stored, the first 50 TB.",
    provider: "azure",
    region: REGION,
    service: "Storage",
    category: "Storage",
    meter: meters.storageBytes.reference,
    pricingUnit: "GB-month",
    unitSize: GIGABYTE,
    currency: "USD",
    unitAmount: "2.08",
    source: BLOBS,
    observedAt: OBSERVED_AT,
});

/** Blob Storage Hot LRS write operations, $0.05 per 10,000. */
export const blobWrites = new Sku(OWNER, {
    name: "azure.blobWrites",
    description: "Blob Storage General Purpose v2 Hot LRS write operations.",
    provider: "azure",
    region: REGION,
    service: "Storage",
    category: "Storage",
    meter: meters.storageWrites.reference,
    pricingUnit: "10K operations",
    unitSize: OPERATIONS,
    currency: "USD",
    unitAmount: "5",
    source: BLOBS,
    observedAt: OBSERVED_AT,
});

/** Blob Storage Hot read operations, $0.004 per 10,000. */
export const blobReads = new Sku(OWNER, {
    name: "azure.blobReads",
    description: "Blob Storage General Purpose v2 Hot read operations.",
    provider: "azure",
    region: REGION,
    service: "Storage",
    category: "Storage",
    meter: meters.storageReads.reference,
    pricingUnit: "10K operations",
    unitSize: OPERATIONS,
    currency: "USD",
    unitAmount: "0.4",
    source: BLOBS,
    observedAt: OBSERVED_AT,
});

/** Data transfer out over Microsoft's network past the free 100 GB, $0.087 per GB. */
export const egress = new Sku(OWNER, {
    name: "azure.egress",
    description: "Data transfer out over Microsoft's premium global network, past the free 100 GB.",
    provider: "azure",
    region: REGION,
    service: "Bandwidth",
    category: "Networking",
    meter: meters.egress.reference,
    pricingUnit: "GB",
    unitSize: GIGABYTE,
    currency: "USD",
    unitAmount: "8.7",
    source: BANDWIDTH,
    observedAt: OBSERVED_AT,
});

/** Data transfer out routed over the internet past the free 100 GB, $0.08 per GB. */
export const internetEgress = new Sku(OWNER, {
    name: "azure.internetEgress",
    description: "Data transfer out routed over the internet, past the free 100 GB.",
    provider: "azure",
    region: REGION,
    service: "Bandwidth",
    category: "Networking",
    meter: meters.egress.reference,
    pricingUnit: "GB",
    unitSize: GIGABYTE,
    currency: "USD",
    unitAmount: "8",
    source: BANDWIDTH,
    observedAt: OBSERVED_AT,
});

/** Azure's SKUs Destack's prices compare with. */
export const AZURE_SKUS: readonly Sku[] = [
    functionsDuration,
    functionsExecutions,
    flexDuration,
    flexExecutions,
    cosmosStorage,
    blobStorage,
    blobWrites,
    blobReads,
    egress,
    internetEgress,
];

/** Azure's price per unit of each meter, at Functions, Cosmos DB, Blob Storage and bandwidth. */
export const AZURE_PRICING = new PricingModel([
    // compute
    [meters.memory.reference, [[functionsDuration, 1]]],
    [meters.requests.reference, [[functionsExecutions, 1]]],

    // database
    [meters.databaseBytes.reference, [[cosmosStorage, 1]]],

    // storage
    [meters.storageBytes.reference, [[blobStorage, 1]]],
    [meters.storageWrites.reference, [[blobWrites, 1]]],
    [meters.storageReads.reference, [[blobReads, 1]]],

    // network
    [meters.egress.reference, [[egress, 1]]],
]);
