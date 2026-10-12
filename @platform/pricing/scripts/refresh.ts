import { readFileSync, writeFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { Cost } from "@destack/finance";
import { schema } from "@destack/schema";
import { VENDOR_SKUS, VENDORS } from "../src/vendor/vendor.ts";

/** The AWS Price List bulk API's offer file of a service in US East (N. Virginia). */
const AWS_OFFER =
    "https://pricing.us-east-1.amazonaws.com/offers/v1.0/aws/{offer}/current/us-east-1/index.json";

/** The Azure Retail Prices API. */
const AZURE_PRICES = "https://prices.azure.com/api/retail/prices";

/** The region Azure's prices are read in: East US. */
const AZURE_REGION = "eastus";

/** The Google Cloud Billing Catalog API's services. */
const GCP_SERVICES = "https://cloudbilling.googleapis.com/v1/services";

/** The environment variable holding the API key the Cloud Billing Catalog API requires. */
const GCP_KEY = "GOOGLE_CLOUD_API_KEY";

/** The most SKUs the Cloud Billing Catalog API returns per page. */
const GCP_PAGE_SIZE = 5000;

/** The digits of the nanos Google writes a price's fraction in. */
const NANO_DIGITS = 9;

/** The decimals a catalog's floating-point price is written with, as a decimal amount keeps. */
const DECIMALS = 12;

/** The cents in a dollar. */
const CENTS = 100n;

/** Where a SKU's list price is read: a catalog row, or a Cloudflare docs page's table row. */
type Lookup =
    | {
          /** An AWS Price List offer's product. */
          readonly kind: "aws";
          /** The offer, such as AmazonS3. */
          readonly offer: string;
          /** The product's usage type. */
          readonly usageType: string;
          /** The unit its price dimensions are per. */
          readonly unit: string;
      }
    | {
          /** An Azure retail price meter in East US. */
          readonly kind: "azure";
          /** The service, such as Storage. */
          readonly serviceName: string;
          /** The product, such as General Block Blob v2. */
          readonly productName: string;
          /** The SKU, such as Hot LRS. */
          readonly skuName: string;
          /** The meter, such as Hot LRS Data Stored. */
          readonly meterName: string;
          /** The unit of measure its prices are per. */
          readonly unit: string;
      }
    | {
          /** A Google Cloud SKU. */
          readonly kind: "gcp";
          /** The service's ID. */
          readonly service: string;
          /** The SKU's ID. */
          readonly sku: string;
          /** The usage unit its prices are per. */
          readonly unit: string;
      }
    | {
          /** A Cloudflare docs page's table row. */
          readonly kind: "cloudflare";
          /** The page's path under the docs. */
          readonly page: string;
          /** The pattern capturing the price in dollars, or Free, in the page's Markdown. */
          readonly pattern: RegExp;
      };

/** Where each vendor SKU's price is read, by the SKU's name. */
const LOOKUPS: Readonly<Record<string, Lookup>> = {
    // cloudflare: the docs pages' Markdown, as Cloudflare publishes no catalog
    "cloudflare.durableObjectDuration": {
        kind: "cloudflare",
        page: "durable-objects/platform/pricing",
        pattern: /^\| Duration[^|]*\|[^|]*\|[^|]*\+ \$([\d.]+)\/million GB-s/gmu,
    },
    "cloudflare.durableObjectRequests": {
        kind: "cloudflare",
        page: "durable-objects/platform/pricing",
        pattern: /^\| Requests \|[^|]*\|[^|]*\+ \$([\d.]+)\/million/gmu,
    },
    "cloudflare.durableObjectRowsRead": {
        kind: "cloudflare",
        page: "durable-objects/platform/pricing",
        pattern: /^\| Rows reads[^|]*\|[^|]*\|[^|]*\+ \$([\d.]+) \/ million rows/gmu,
    },
    "cloudflare.durableObjectRowsWritten": {
        kind: "cloudflare",
        page: "durable-objects/platform/pricing",
        pattern: /^\| Rows written[^|]*\|[^|]*\|[^|]*\+ \$([\d.]+) \/ million rows/gmu,
    },
    "cloudflare.durableObjectStorage": {
        kind: "cloudflare",
        page: "durable-objects/platform/pricing",
        pattern: /^\| SQL Stored data[^|]*\|[^|]*\|[^|]*\+ \$([\d.]+)\/ GB-month/gmu,
    },
    "cloudflare.r2Storage": {
        kind: "cloudflare",
        page: "r2/pricing",
        pattern: /^\| Storage \| \$([\d.]+) \/ GB-month/gmu,
    },
    "cloudflare.r2ClassA": {
        kind: "cloudflare",
        page: "r2/pricing",
        pattern: /^\| Class A Operations \| \$([\d.]+) \/ million requests/gmu,
    },
    "cloudflare.r2ClassB": {
        kind: "cloudflare",
        page: "r2/pricing",
        pattern: /^\| Class B Operations \| \$([\d.]+) \/ million requests/gmu,
    },
    "cloudflare.r2Egress": {
        kind: "cloudflare",
        page: "r2/pricing",
        pattern: /^\| Egress \(data transfer to Internet\) \| (Free)/gmu,
    },
    "cloudflare.workerRequests": {
        kind: "cloudflare",
        page: "workers/platform/pricing",
        pattern: /^\| \*\*Standard\*\* \|[^|]*\+\$([\d.]+) per additional million \|/gmu,
    },
    "cloudflare.workerCpu": {
        kind: "cloudflare",
        page: "workers/platform/pricing",
        pattern: /\+\$([\d.]+) per additional million CPU milliseconds/gmu,
    },

    // aws: the Price List bulk API's offer files
    "aws.lambdaDuration": {
        kind: "aws",
        offer: "AWSLambda",
        usageType: "Lambda-GB-Second",
        unit: "Lambda-GB-Second",
    },
    "aws.lambdaRequests": {
        kind: "aws",
        offer: "AWSLambda",
        usageType: "Request",
        unit: "Requests",
    },
    "aws.dynamodbStorage": {
        kind: "aws",
        offer: "AmazonDynamoDB",
        usageType: "TimedStorage-ByteHrs",
        unit: "GB-Mo",
    },
    "aws.dynamodbReads": {
        kind: "aws",
        offer: "AmazonDynamoDB",
        usageType: "ReadRequestUnits",
        unit: "ReadRequestUnits",
    },
    "aws.dynamodbWrites": {
        kind: "aws",
        offer: "AmazonDynamoDB",
        usageType: "WriteRequestUnits",
        unit: "WriteRequestUnits",
    },
    "aws.s3Storage": {
        kind: "aws",
        offer: "AmazonS3",
        usageType: "TimedStorage-ByteHrs",
        unit: "GB-Mo",
    },
    "aws.s3Writes": {
        kind: "aws",
        offer: "AmazonS3",
        usageType: "Requests-Tier1",
        unit: "Requests",
    },
    "aws.s3Reads": {
        kind: "aws",
        offer: "AmazonS3",
        usageType: "Requests-Tier2",
        unit: "Requests",
    },
    "aws.egress": {
        kind: "aws",
        offer: "AWSDataTransfer",
        usageType: "DataTransfer-Out-Bytes",
        unit: "GB",
    },

    // azure: the Retail Prices API
    "azure.functionsDuration": {
        kind: "azure",
        serviceName: "Functions",
        productName: "Functions",
        skuName: "Standard",
        meterName: "Standard Execution Time",
        unit: "1 GB Second",
    },
    "azure.functionsExecutions": {
        kind: "azure",
        serviceName: "Functions",
        productName: "Functions",
        skuName: "Standard",
        meterName: "Standard Total Executions",
        unit: "10",
    },
    "azure.flexDuration": {
        kind: "azure",
        serviceName: "Functions",
        productName: "Flex Consumption",
        skuName: "On Demand",
        meterName: "On Demand Execution Time",
        unit: "1 GB Second",
    },
    "azure.flexExecutions": {
        kind: "azure",
        serviceName: "Functions",
        productName: "Flex Consumption",
        skuName: "On Demand",
        meterName: "On Demand Total Executions",
        unit: "10",
    },
    "azure.cosmosStorage": {
        kind: "azure",
        serviceName: "Azure Cosmos DB",
        productName: "Azure Cosmos DB",
        skuName: "RUs",
        meterName: "Data Stored",
        unit: "1 GB/Month",
    },
    "azure.blobStorage": {
        kind: "azure",
        serviceName: "Storage",
        productName: "General Block Blob v2",
        skuName: "Hot LRS",
        meterName: "Hot LRS Data Stored",
        unit: "1 GB/Month",
    },
    "azure.blobWrites": {
        kind: "azure",
        serviceName: "Storage",
        productName: "General Block Blob v2",
        skuName: "Hot LRS",
        meterName: "Hot LRS Write Operations",
        unit: "10K",
    },
    "azure.blobReads": {
        kind: "azure",
        serviceName: "Storage",
        productName: "General Block Blob v2",
        skuName: "Hot LRS",
        meterName: "Hot Read Operations",
        unit: "10K",
    },
    "azure.egress": {
        kind: "azure",
        serviceName: "Bandwidth",
        productName: "Rtn Preference: MGN",
        skuName: "Standard",
        meterName: "Standard Data Transfer Out",
        unit: "1 GB",
    },
    "azure.internetEgress": {
        kind: "azure",
        serviceName: "Bandwidth",
        productName: "Bandwidth - Routing Preference: Internet",
        skuName: "Standard",
        meterName: "Standard Data Transfer Out",
        unit: "1 GB",
    },

    // gcp: the Cloud Billing Catalog API, its IDs as Google's SKU group pages list them
    "gcp.storage": {
        kind: "gcp",
        service: "95FF-2EF5-5EA1",
        sku: "E5F0-6A5D-7BAD",
        unit: "GiBy.h",
    },
    "gcp.storageWrites": {
        kind: "gcp",
        service: "95FF-2EF5-5EA1",
        sku: "4DBF-185F-A415",
        unit: "count",
    },
    "gcp.storageReads": {
        kind: "gcp",
        service: "95FF-2EF5-5EA1",
        sku: "7870-010B-2763",
        unit: "count",
    },
    "gcp.firestoreStorage": {
        kind: "gcp",
        service: "EE2C-7FAC-5E08",
        sku: "140B-ADDF-6A12",
        unit: "GiBy.h",
    },
    "gcp.runRequests": {
        kind: "gcp",
        service: "152E-C115-5142",
        sku: "2DA5-55D3-E679",
        unit: "count",
    },
    "gcp.egress": {
        kind: "gcp",
        service: "6F81-5844-456A",
        sku: "F274-1692-F213",
        unit: "GiBy",
    },
};

/** A catalog's tier: where it starts, and its price in dollars. */
interface Tier {
    /** The usage the tier starts at. */
    readonly start: number;
    /** The price per unit in dollars, as a decimal. */
    readonly price: string;
}

/** An AWS Price List offer file, as far as the refresh reads it, its other fields kept. */
const AwsOffer = schema.looseObject({
    /** The products by SKU. */
    products: schema.record(
        schema.string(),
        schema.looseObject({
            /** The product's SKU. */
            sku: schema.string(),
            /** Its attributes, such as its usage type. */
            attributes: schema.record(schema.string(), schema.string()),
        }),
    ),
    /** The terms, on-demand ones by product SKU and offer term code. */
    terms: schema.looseObject({
        /** The on-demand terms. */
        OnDemand: schema.record(
            schema.string(),
            schema.record(
                schema.string(),
                schema.looseObject({
                    /** The term's price dimensions, one per tier. */
                    priceDimensions: schema.record(
                        schema.string(),
                        schema.looseObject({
                            /** The unit the price is per. */
                            unit: schema.string(),
                            /** The usage the tier starts at. */
                            beginRange: schema.string(),
                            /** The price per unit by currency. */
                            pricePerUnit: schema.looseObject({ USD: schema.string() }),
                        }),
                    ),
                }),
            ),
        ),
    }),
});

/** A page of Azure retail prices, as far as the refresh reads it. */
const AzurePage = schema.looseObject({
    /** The page's items. */
    Items: schema.array(
        schema.looseObject({
            /** The unit the price is per, such as 1 GB/Month. */
            unitOfMeasure: schema.string(),
            /** The usage the tier starts at. */
            tierMinimumUnits: schema.number(),
            /** The list price in dollars. */
            retailPrice: schema.number(),
        }),
    ),
    /** The next page, null on the last. */
    NextPageLink: schema.string().nullable(),
});

/** A page of a Google Cloud service's SKUs, as far as the refresh reads it. */
const GcpPage = schema.looseObject({
    /** The page's SKUs. */
    skus: schema.array(
        schema.looseObject({
            /** The SKU's ID. */
            skuId: schema.string(),
            /** Its pricing, the current one first. */
            pricingInfo: schema.array(
                schema.looseObject({
                    /** How its price grows with usage. */
                    pricingExpression: schema.looseObject({
                        /** The unit the prices are per, such as GiBy.mo. */
                        usageUnit: schema.string(),
                        /** The tiers' prices. */
                        tieredRates: schema.array(
                            schema.looseObject({
                                /** The usage the tier starts at. */
                                startUsageAmount: schema.number(),
                                /** The price per unit, in whole dollars and nanos. */
                                unitPrice: schema.looseObject({
                                    units: schema.string(),
                                    nanos: schema.number().int(),
                                }),
                            }),
                        ),
                    }),
                }),
            ),
        }),
    ),
    /** The next page's token, empty on the last. */
    nextPageToken: schema.string(),
});

/** The JSON documents read so far, by URL, each read once. */
const documents = new Map<string, Promise<unknown>>();

/** The pages read so far, by URL, each read once. */
const pages = new Map<string, Promise<string>>();

// read the vendors to refresh, every vendor by default
/** The vendors named on the command line. */
const vendors = schema.array(schema.enum(VENDORS)).parse(process.argv.slice(2));
/** The vendors to refresh. */
const refreshed = vendors.length === 0 ? VENDORS : vendors;

// read each vendor's prices, then rewrite its module's amounts and its day
/** The day the prices are read, as YYYY-MM-DD. */
const today = new Date().toISOString().slice(0, 10);
for (const vendor of refreshed) {
    // read every SKU's price in cents before writing anything
    const prices = await Promise.all(
        VENDOR_SKUS[vendor].map(async (sku) => {
            // find where the SKU's price is published
            const lookup = LOOKUPS[sku.name];
            if (lookup === undefined) {
                throw new Error(`sku ${sku.name} has no catalog lookup`);
            }

            // read it in dollars and write it in cents
            const dollars = await read(sku.name, lookup);

            return { sku, cents: Cost.of(Cost.scaled(dollars) * CENTS) };
        }),
    );

    // rewrite each amount and the day the prices were read
    const file = fileURLToPath(new URL(`../src/vendor/${vendor}.ts`, import.meta.url));
    let source = readFileSync(file, "utf8");
    for (const { sku, cents } of prices) {
        source = replaced(
            source,
            new RegExp(`(name: "${sku.name}",[^}]*?unitAmount: ")([^"]*)(")`, "u"),
            cents,
            sku.name,
        );
        if (cents !== sku.definition.unitAmount) {
            process.stdout.write(`${sku.name}: ${sku.definition.unitAmount} -> ${cents}\n`);
        }
    }
    source = replaced(source, /(const OBSERVED_AT = ")([^"]*)(")/u, today, vendor);
    writeFileSync(file, source);
}

/** Read a SKU's list price in dollars from its vendor's catalog. */
async function read(name: string, lookup: Lookup): Promise<string> {
    // read the catalog's tiers, or the page's one price
    if (lookup.kind === "aws") {
        return paid(name, await awsTiers(name, lookup));
    } else if (lookup.kind === "azure") {
        return paid(name, await azureTiers(name, lookup));
    } else if (lookup.kind === "gcp") {
        return paid(name, await gcpTiers(name, lookup));
    }

    return cloudflarePrice(name, lookup);
}

/** Read an AWS product's tiers from its offer file, refusing a product found never or twice. */
async function awsTiers(
    name: string,
    lookup: Extract<Lookup, { kind: "aws" }>,
): Promise<readonly Tier[]> {
    // find the one product of the usage type
    const offer = AwsOffer.parse(await json(AWS_OFFER.replace("{offer}", lookup.offer)));
    const products = Object.values(offer.products).filter(
        (product) => product.attributes["usagetype"] === lookup.usageType,
    );
    const [product] = products;
    if (product === undefined || products.length > 1) {
        throw new Error(`sku ${name} matches ${products.length} ${lookup.offer} products`);
    }

    // read its on-demand price dimensions, each in the expected unit
    const terms = Object.values(offer.terms.OnDemand[product.sku] ?? {});
    const dimensions = terms.flatMap((term) => Object.values(term.priceDimensions));
    for (const dimension of dimensions) {
        if (dimension.unit !== lookup.unit) {
            throw new Error(`sku ${name} is priced per ${dimension.unit}, not ${lookup.unit}`);
        }
    }

    return dimensions.map((dimension) => ({
        start: Number(dimension.beginRange),
        price: dimension.pricePerUnit.USD,
    }));
}

/** Read an Azure meter's tiers in East US, following every page. */
async function azureTiers(
    name: string,
    lookup: Extract<Lookup, { kind: "azure" }>,
): Promise<readonly Tier[]> {
    // filter the meter by its keys
    const filter = [
        `serviceName eq '${lookup.serviceName}'`,
        `productName eq '${lookup.productName}'`,
        `skuName eq '${lookup.skuName}'`,
        `meterName eq '${lookup.meterName}'`,
        `armRegionName eq '${AZURE_REGION}'`,
        `type eq 'Consumption'`,
    ].join(" and ");
    let next: string | null = `${AZURE_PRICES}?$filter=${encodeURIComponent(filter)}`;

    // read every page's items, each in the expected unit
    const items: schema.Infer<typeof AzurePage>["Items"][number][] = [];
    while (next !== null) {
        const page = AzurePage.parse(await json(next));
        items.push(...page.Items);
        next = page.NextPageLink;
    }
    for (const item of items) {
        if (item.unitOfMeasure !== lookup.unit) {
            throw new Error(`sku ${name} is priced per ${item.unitOfMeasure}, not ${lookup.unit}`);
        }
    }

    return items.map((item) => ({
        start: item.tierMinimumUnits,
        price: item.retailPrice.toFixed(DECIMALS),
    }));
}

/** Read a Google Cloud SKU's tiers from its service's catalog, which requires an API key. */
async function gcpTiers(
    name: string,
    lookup: Extract<Lookup, { kind: "gcp" }>,
): Promise<readonly Tier[]> {
    // require the key the catalog takes
    const key = process.env[GCP_KEY];
    if (key === undefined || key === "") {
        throw new Error(`set ${GCP_KEY} to read Google's Cloud Billing Catalog`);
    }

    // read the service's pages until the SKU is found
    let token = "";
    let found: schema.Infer<typeof GcpPage>["skus"][number] | undefined;
    do {
        const query = new URLSearchParams({
            key,
            currencyCode: "USD",
            pageSize: String(GCP_PAGE_SIZE),
            pageToken: token,
        });
        const page = GcpPage.parse(
            await json(`${GCP_SERVICES}/${lookup.service}/skus?${query.toString()}`),
        );
        found = page.skus.find((sku) => sku.skuId === lookup.sku);
        token = page.nextPageToken;
    } while (found === undefined && token !== "");

    // read its current pricing expression, in the expected unit
    const expression = found?.pricingInfo[0]?.pricingExpression;
    if (expression === undefined) {
        throw new Error(`sku ${name} is missing from service ${lookup.service}`);
    }
    if (expression.usageUnit !== lookup.unit) {
        throw new Error(`sku ${name} is priced per ${expression.usageUnit}, not ${lookup.unit}`);
    }

    return expression.tieredRates.map((rate) => ({
        start: rate.startUsageAmount,
        price: `${rate.unitPrice.units}.${String(rate.unitPrice.nanos).padStart(NANO_DIGITS, "0")}`,
    }));
}

/** Read a Cloudflare price from its docs page, refusing a pattern matching never or differently. */
async function cloudflarePrice(
    name: string,
    lookup: Extract<Lookup, { kind: "cloudflare" }>,
): Promise<string> {
    // read every match's captured price, a free one as zero
    const page = await text(`https://developers.cloudflare.com/${lookup.page}/index.md`);
    const prices = new Set(
        [...page.matchAll(lookup.pattern)].map((match) => (match[1] === "Free" ? "0" : match[1])),
    );
    const [price] = prices;
    if (price === undefined || prices.size > 1) {
        throw new Error(`sku ${name} reads ${prices.size} prices on ${lookup.page}`);
    }

    return price;
}

/** Pick the first paid tier, the price past any free usage, refusing a SKU with none. */
function paid(name: string, tiers: readonly Tier[]): string {
    // order the tiers by where they start
    const ordered = tiers.toSorted((left, right) => left.start - right.start);
    const tier = ordered.find((each) => Cost.scaled(each.price) > 0n);
    if (tier === undefined) {
        throw new Error(`sku ${name} has no paid tier`);
    }

    return tier.price;
}

/** Replace a pattern's one match's middle group, refusing a source it misses. */
function replaced(source: string, pattern: RegExp, value: string, name: string): string {
    if (!pattern.test(source)) {
        throw new Error(`${name} has no amount to rewrite`);
    }

    return source.replace(
        pattern,
        (_match, before: string, _value: string, after: string) => `${before}${value}${after}`,
    );
}

/** Fetch a URL's JSON once, refusing a failed response. */
function json(url: string): Promise<unknown> {
    const document = documents.get(url) ?? fetched(url).then((response) => response.json());
    documents.set(url, document);

    return document;
}

/** Fetch a URL's text once, refusing a failed response. */
function text(url: string): Promise<string> {
    const page = pages.get(url) ?? fetched(url).then((response) => response.text());
    pages.set(url, page);

    return page;
}

/** Fetch a URL, refusing a failed response. */
async function fetched(url: string): Promise<Response> {
    const response = await fetch(url);
    if (!response.ok) {
        throw new Error(`${new URL(url).host}${new URL(url).pathname} answered ${response.status}`);
    }

    return response;
}
