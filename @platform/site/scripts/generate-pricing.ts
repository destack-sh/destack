import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { formatSource, Toolchain } from "@destack/check";
import { BunEnvironment } from "@destack/environment/bun";
import { present } from "@destack/schema";
import {
    bucketReadUnit,
    bucketUsageUnit,
    bucketWriteUnit,
    computeOperationUnit,
    computeTimeUnit,
    computeUsageUnit,
    CREDIT_PRICES,
    CURRENCIES,
    databaseReadUnit,
    databaseUsageUnit,
    databaseWriteUnit,
    ENTERPRISE,
    inferenceCostUnit,
    networkEgressUnit,
    networkIngressUnit,
    MACHINES,
    PLANS,
} from "@platform/pricing";

/** A unit SKU of Destack's price list. */
type Unit = typeof computeUsageUnit;

/** The CPU milliseconds in a CPU-hour, which CPU time is shown per. */
const CPU_HOUR_MILLISECONDS = 3_600_000;

/** The minutes in an hour, which machines are also shown per. */
const MINUTES_PER_HOUR = 60;

/** The GB-seconds in a GB-hour. */
const SECONDS_PER_HOUR = 3600;

/** The units in a million, the quantity the site prices counted meters per. */
const MILLION = 1_000_000;

/** The minor units of credits in a credit, and of US dollars in a dollar. */
const MINOR_UNITS = 100;

/** The bytes in a gigabyte. */
const GIGABYTE = 1_000_000_000;

/** A month the free plan's included usage covers, in GB, GB-hours and millions of requests. */
const FREE_MONTH = { database: 1, files: 10, compute: 10, requests: 1 } as const;

/** The repository root. */
const repositoryDirectory = resolve(dirname(fileURLToPath(import.meta.url)), "../../..");
/** The generated pricing module. */
const generatedFile = join(
    repositoryDirectory,
    "@platform/site/src/view/content/generated/pricing.ts",
);
/** The machine the generator runs on. */
const environment = BunEnvironment.system;
/** Whether this run checks the generated file instead of writing it. */
const isCheck = environment.process.arguments.includes("--check");

// price usage at Destack's own unit prices, in US dollars
/** The site's usage rates in US dollars, per GB-month, GB-hour or million. */
const rates = {
    database: rateOf(databaseUsageUnit, GIGABYTE),
    files: rateOf(bucketUsageUnit, GIGABYTE),
    memory: rateOf(computeUsageUnit, SECONDS_PER_HOUR),
    cpu: rateOf(computeTimeUnit, MILLION),
    requests: rateOf(computeOperationUnit, MILLION),
    rowsRead: rateOf(databaseReadUnit, MILLION),
    rowsWritten: rateOf(databaseWriteUnit, MILLION),
    fileReads: rateOf(bucketReadUnit, MILLION),
    fileWrites: rateOf(bucketWriteUnit, MILLION),
};

// price each unit as the page lists it, in US dollars
/** Each unit by resource: what it is, per what, and its price. */
const units = [
    unitOf("Compute", "Memory", "GB-hour", computeUsageUnit, SECONDS_PER_HOUR),
    unitOf("Compute", "CPU time", "CPU-hour", computeTimeUnit, CPU_HOUR_MILLISECONDS),
    unitOf("Compute", "Requests", "million", computeOperationUnit, MILLION),
    unitOf("Database", "Storage", "GB-month", databaseUsageUnit, GIGABYTE),
    unitOf("Database", "Writes", "million rows", databaseWriteUnit, MILLION),
    unitOf("Database", "Reads", "million rows", databaseReadUnit, MILLION),
    unitOf("Files", "Storage", "GB-month", bucketUsageUnit, GIGABYTE),
    unitOf("Files", "Writes", "million", bucketWriteUnit, MILLION),
    unitOf("Files", "Reads", "million", bucketReadUnit, MILLION),
    unitOf("Network", "Ingress", "GB", networkIngressUnit, GIGABYTE),
    unitOf("Network", "Egress", "GB", networkEgressUnit, GIGABYTE),
    unitOf("Inference", "Model calls", "US dollar at list price", inferenceCostUnit, 1),
];

// read each plan's name, fee in each currency, support and gates
/** The plans, cheapest first. */
const catalogPlans = Object.entries(PLANS).map(([key, plan]) => {
    // require the list of places each plan may run in
    const tenancies = plan.productFeatures.tenancy.value;
    if (!Array.isArray(tenancies)) {
        throw new TypeError(`plan ${key} grants no tenancy list`);
    }

    return {
        key,
        name: plan.name.source,
        fees: Object.fromEntries(
            CURRENCIES.map((currency) => [
                currency,
                "prices" in plan
                    ? present(plan.prices[currency.toLowerCase()], `${key} price in ${currency}`)
                          .unitAmount / MINOR_UNITS
                    : 0,
            ]),
        ),
        support: plan.productFeatures.support.value,
        isUncapped: !("overage" in plan && plan.overage === "capped"),
        isCustomDomains: "custom-domains" in plan.productFeatures,
        isCustomRetention: "custom-retention" in plan.productFeatures,
        isEarlyAccess: "early-access" in plan.productFeatures,
        isDedicated: tenancies.includes("dedicated"),
        isContract: false,
    };
});

// follow them with the enterprise contract: all of the last plan, with its own fee and support
/** The plans, then the enterprise contract. */
const plans = [
    ...catalogPlans,
    {
        ...present(catalogPlans.at(-1), "last plan"),
        key: "enterprise",
        name: ENTERPRISE.name.source,
        fees: Object.fromEntries(
            CURRENCIES.map((currency) => [currency, ENTERPRISE.commitment[currency] / MINOR_UNITS]),
        ),
        support: ENTERPRISE.support,
        isContract: true,
    },
];

// read a credit's price in each currency, which converts the rates
/** A credit's price in each currency, in its major unit per US dollar of rate. */
const creditPrices = Object.fromEntries(
    CURRENCIES.map((currency) => [
        currency,
        Number(CREDIT_PRICES[currency]) / Number(CREDIT_PRICES.USD),
    ]),
);

// read the free plan's included usage per account, and require the month it covers to cost exactly that
/** The free plan's monthly usage per account, and a month it covers. */
const allowance = { usage: dollarsOf(PLANS.free.includedUsage), ...FREE_MONTH };
/** What the month the free plan covers costs at the unit prices, in US dollars. */
const covered = roundOf(
    FREE_MONTH.database * rates.database +
        FREE_MONTH.files * rates.files +
        FREE_MONTH.compute * rates.memory +
        FREE_MONTH.requests * rates.requests,
);
if (covered !== allowance.usage) {
    throw new Error(`the free month costs ${covered}, not the plan's ${allowance.usage}`);
}

// read each machine size's vCPUs, memory, disk, and price per minute and hour in US dollars
/** The dedicated machines, smallest first. */
const machines = Object.entries(MACHINES).map(([size, machine]) => ({
    size,
    vcpus: machine.vcpus,
    memory: machine.memory,
    disk: machine.disk,
    hourly: dollarsOf(machine.hourly),
    minutely: dollarsOf(machine.hourly / MINUTES_PER_HOUR),
}));

// render the generated module
/** The generated pricing module source. */
const generatedSource = await formatSource(
    generatedFile,
    `/** The usage rates in US dollars, per GB-month, GB-hour or million. */\n` +
        `export const rates = ${JSON.stringify(rates)} as const;\n\n` +
        `/** Each unit by resource, with its price in US dollars. */\n` +
        `export const units = ${JSON.stringify(units)} as const;\n\n` +
        `/** The currencies Destack bills in. */\n` +
        `export const currencies = ${JSON.stringify(CURRENCIES)} as const;\n\n` +
        `/** A US dollar of rate in each currency. */\n` +
        `export const creditPrices = ${JSON.stringify(creditPrices)} as const;\n\n` +
        `/** The plans, cheapest first, then the enterprise contract, each fee a month in each currency. */\n` +
        `export const plans = ${JSON.stringify(plans)} as const;\n\n` +
        `/** The free plan's usage per account a month in US dollars, and a month it covers. */\n` +
        `export const allowance = ${JSON.stringify(allowance)} as const;\n\n` +
        `/** The dedicated machines, smallest first, priced in US dollars. */\n` +
        `export const machines = ${JSON.stringify(machines)} as const;\n`,
    Toolchain.workspace(),
);

/** Return a unit's price in US dollars for a quantity of its meter, such as a gigabyte of bytes. */
function rateOf(unit: Unit, quantity: number): number {
    const credits = Number(unit.definition.unitAmount) / MINOR_UNITS;

    return roundOf(credits * (quantity / unit.definition.unitSize) * Number(CREDIT_PRICES.USD));
}

/** Return a unit as the page lists it, priced for a quantity of its meter. */
function unitOf(resource: string, name: string, per: string, unit: Unit, quantity: number) {
    return { resource, name, per, price: rateOf(unit, quantity) };
}

/** Return credit cents in US dollars. */
function dollarsOf(cents: number): number {
    return roundOf((cents / MINOR_UNITS) * Number(CREDIT_PRICES.USD));
}

/** Round a dollar rate to the thousandth of a cent, past the noise of binary fractions. */
function roundOf(amount: number): number {
    return Math.round(amount * 100_000) / 100_000;
}

// check or replace the module
/** The file system the module is written to. */
const { files } = environment;
if (isCheck) {
    const isCurrent =
        (await files.exists(generatedFile)) &&
        (await files.readTextFile(generatedFile)) === generatedSource;
    if (!isCurrent) {
        throw new Error("generated pricing is out of date, run `just @platform/site/generate`");
    }
} else {
    await files.mkdir(dirname(generatedFile), { recursive: true });
    const temporaryFile = `${generatedFile}.${environment.process.id}.tmp`;
    await files.writeTextFile(temporaryFile, generatedSource);
    await files.rename(temporaryFile, generatedFile);
}
