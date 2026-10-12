import { Sku } from "@destack/finance";
import * as meters from "@destack/observability/meter";
import { PricingModel } from "../price/model.ts";

/** The package keeping the vendors' list prices as data. */
const OWNER = import.meta.destack.package;

/** The page publishing Lambda's prices. */
const LAMBDA = "https://aws.amazon.com/lambda/pricing/";

/** The page publishing DynamoDB's on-demand prices. */
const DYNAMODB = "https://aws.amazon.com/dynamodb/pricing/on-demand/";

/** The page publishing S3's prices. */
const S3 = "https://aws.amazon.com/s3/pricing/";

/** The page publishing data transfer prices. */
const DATA_TRANSFER = "https://aws.amazon.com/ec2/pricing/on-demand/";

/** The day the prices were read at their sources. */
const OBSERVED_AT = "2026-10-12";

/** The region the prices apply in: US East (N. Virginia). */
const REGION = "us-east-1";

/** The bytes in a GB, which AWS publishes without a definition, taken as 10^9. */
const GIGABYTE = 1_000_000_000;

/** Lambda duration on x86, the first 6 billion GB-seconds a month, $0.0000166667 per GB-second. */
export const lambdaDuration = new Sku(OWNER, {
    name: "aws.lambdaDuration",
    description: "Lambda duration on x86, the first 6 billion GB-seconds a month.",
    provider: "aws",
    region: REGION,
    service: "AWS Lambda",
    category: "Compute",
    meter: meters.memory.reference,
    pricingUnit: "GB-second",
    unitSize: 1,
    currency: "USD",
    unitAmount: "0.00166667",
    source: LAMBDA,
    observedAt: OBSERVED_AT,
});

/** Lambda requests, $0.0000002 each. */
export const lambdaRequests = new Sku(OWNER, {
    name: "aws.lambdaRequests",
    description: "Lambda requests.",
    provider: "aws",
    region: REGION,
    service: "AWS Lambda",
    category: "Compute",
    meter: meters.requests.reference,
    pricingUnit: "request",
    unitSize: 1,
    currency: "USD",
    unitAmount: "0.00002",
    source: LAMBDA,
    observedAt: OBSERVED_AT,
});

/** DynamoDB Standard table storage past the free 25 GB-months, $0.25 per GB-month. */
export const dynamodbStorage = new Sku(OWNER, {
    name: "aws.dynamodbStorage",
    description: "DynamoDB Standard table class indexed storage.",
    provider: "aws",
    region: REGION,
    service: "Amazon DynamoDB",
    category: "Databases",
    meter: meters.databaseBytes.reference,
    pricingUnit: "GB-month",
    unitSize: GIGABYTE,
    currency: "USD",
    unitAmount: "25",
    source: DYNAMODB,
    observedAt: OBSERVED_AT,
});

/** DynamoDB on-demand reads, $0.125 per million read request units of up to 4 KB each. */
export const dynamodbReads = new Sku(OWNER, {
    name: "aws.dynamodbReads",
    description: "DynamoDB on-demand read request units.",
    provider: "aws",
    region: REGION,
    service: "Amazon DynamoDB",
    category: "Databases",
    meter: meters.rowsRead.reference,
    pricingUnit: "read request unit",
    unitSize: 1,
    currency: "USD",
    unitAmount: "0.0000125",
    source: DYNAMODB,
    observedAt: OBSERVED_AT,
});

/** DynamoDB on-demand writes, $0.625 per million write request units of up to 1 KB each. */
export const dynamodbWrites = new Sku(OWNER, {
    name: "aws.dynamodbWrites",
    description: "DynamoDB on-demand write request units.",
    provider: "aws",
    region: REGION,
    service: "Amazon DynamoDB",
    category: "Databases",
    meter: meters.rowsWritten.reference,
    pricingUnit: "write request unit",
    unitSize: 1,
    currency: "USD",
    unitAmount: "0.0000625",
    source: DYNAMODB,
    observedAt: OBSERVED_AT,
});

/** S3 Standard storage, the first 50 TB a month, $0.023 per GB-month. */
export const s3Storage = new Sku(OWNER, {
    name: "aws.s3Storage",
    description: "S3 Standard storage, the first 50 TB a month.",
    provider: "aws",
    region: REGION,
    service: "Amazon S3",
    category: "Storage",
    meter: meters.storageBytes.reference,
    pricingUnit: "GB-month",
    unitSize: GIGABYTE,
    currency: "USD",
    unitAmount: "2.3",
    source: S3,
    observedAt: OBSERVED_AT,
});

/** S3 Standard PUT, COPY, POST and LIST requests, $0.000005 each. */
export const s3Writes = new Sku(OWNER, {
    name: "aws.s3Writes",
    description: "S3 Standard PUT, COPY, POST and LIST requests.",
    provider: "aws",
    region: REGION,
    service: "Amazon S3",
    category: "Storage",
    meter: meters.storageWrites.reference,
    pricingUnit: "request",
    unitSize: 1,
    currency: "USD",
    unitAmount: "0.0005",
    source: S3,
    observedAt: OBSERVED_AT,
});

/** S3 Standard GET and all other requests, $0.0000004 each. */
export const s3Reads = new Sku(OWNER, {
    name: "aws.s3Reads",
    description: "S3 Standard GET and all other requests.",
    provider: "aws",
    region: REGION,
    service: "Amazon S3",
    category: "Storage",
    meter: meters.storageReads.reference,
    pricingUnit: "request",
    unitSize: 1,
    currency: "USD",
    unitAmount: "0.00004",
    source: S3,
    observedAt: OBSERVED_AT,
});

/** Data transfer out to the internet, the first 10 TB a month, $0.09 per GB. */
export const egress = new Sku(OWNER, {
    name: "aws.egress",
    description: "Data transfer out to the internet, the first 10 TB a month.",
    provider: "aws",
    region: REGION,
    service: "AWS Data Transfer",
    category: "Networking",
    meter: meters.egress.reference,
    pricingUnit: "GB",
    unitSize: GIGABYTE,
    currency: "USD",
    unitAmount: "9",
    source: DATA_TRANSFER,
    observedAt: OBSERVED_AT,
});

/** AWS's SKUs Destack's prices compare with. */
export const AWS_SKUS: readonly Sku[] = [
    lambdaDuration,
    lambdaRequests,
    dynamodbStorage,
    dynamodbReads,
    dynamodbWrites,
    s3Storage,
    s3Writes,
    s3Reads,
    egress,
];

/** AWS's price per unit of each meter, at Lambda, DynamoDB, S3 and data transfer. */
export const AWS_PRICING = new PricingModel([
    // compute
    [meters.memory.reference, [[lambdaDuration, 1]]],
    [meters.requests.reference, [[lambdaRequests, 1]]],

    // database
    [meters.databaseBytes.reference, [[dynamodbStorage, 1]]],
    [meters.rowsRead.reference, [[dynamodbReads, 1]]],
    [meters.rowsWritten.reference, [[dynamodbWrites, 1]]],

    // storage
    [meters.storageBytes.reference, [[s3Storage, 1]]],
    [meters.storageWrites.reference, [[s3Writes, 1]]],
    [meters.storageReads.reference, [[s3Reads, 1]]],

    // network
    [meters.egress.reference, [[egress, 1]]],
]);
