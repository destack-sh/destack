import { existsSync, mkdirSync, readFileSync, renameSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { formatSource } from "@destack/check";
import { present } from "@destack/schema";
import {
    cpuUnit,
    CREDIT_PRICES,
    CURRENCIES,
    databaseBytesUnit,
    MACHINES,
    memoryUnit,
    PLANS,
    requestsUnit,
    rowsReadUnit,
    rowsWrittenUnit,
    storageBytesUnit,
    storageReadsUnit,
    storageWritesUnit,
} from "@platform/pricing";

/** The GB-seconds in a GB-hour. */
const SECONDS_PER_HOUR = 3600;

/** The units in a million, the quantity the site prices counted meters per. */
const MILLION = 1_000_000;

/** The minor units of credits in a credit, and of US dollars in a dollar. */
const MINOR_UNITS = 100;

/** The bytes in a gigabyte. */
const GIGABYTE = 1_000_000_000;

/** The repository root. */
const repositoryDirectory = resolve(dirname(fileURLToPath(import.meta.url)), "../../..");
/** The generated pricing module. */
const generatedFile = join(
    repositoryDirectory,
    "@platform/site/src/view/content/generated/pricing.ts",
);
/** Whether this run checks the generated file instead of writing it. */
const isCheck = process.argv.includes("--check");

// price usage at Destack's own unit prices, in US dollars
/** The site's usage rates in US dollars, per GB-month, GB-hour or million. */
const rates = {
    database: rateOf(databaseBytesUnit, GIGABYTE),
    files: rateOf(storageBytesUnit, GIGABYTE),
    memory: rateOf(memoryUnit, SECONDS_PER_HOUR),
    cpu: rateOf(cpuUnit, MILLION),
    requests: rateOf(requestsUnit, MILLION),
    rowsRead: rateOf(rowsReadUnit, MILLION),
    rowsWritten: rateOf(rowsWrittenUnit, MILLION),
    fileReads: rateOf(storageReadsUnit, MILLION),
    fileWrites: rateOf(storageWritesUnit, MILLION),
};

// read each plan's name, fee in each currency, support, retention and gates
/** The plans, cheapest first. */
const plans = Object.entries(PLANS).map(([key, plan]) => {
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
        retention: plan.productFeatures.retention.value,
        isUncapped: plan.productFeatures.database.value === null,
        isCustomDomains: "custom-domains" in plan.productFeatures,
        isEarlyAccess: "early-access" in plan.productFeatures,
        isDedicated: tenancies.includes("dedicated"),
    };
});

// read a credit's price in each currency, which converts the rates
/** A credit's price in each currency, in its major unit per US dollar of rate. */
const creditPrices = Object.fromEntries(
    CURRENCIES.map((currency) => [
        currency,
        Number(CREDIT_PRICES[currency]) / Number(CREDIT_PRICES.USD),
    ]),
);

// read the free plan's allowance in GB and GB-hours
/** The free plan's monthly allowance. */
const allowance = {
    database: limitOf(PLANS.free.productFeatures.database.value, "database") / GIGABYTE,
    files: limitOf(PLANS.free.productFeatures.storage.value, "storage") / GIGABYTE,
    compute: limitOf(PLANS.free.productFeatures.compute.value, "compute") / SECONDS_PER_HOUR,
    egress: limitOf(PLANS.free.productFeatures.network.value, "network") / GIGABYTE,
};

// read each machine size's vCPUs, memory, hourly price and monthly cap in US dollars
/** The dedicated machines, smallest first. */
const machines = Object.entries(MACHINES).map(([size, machine]) => ({
    size,
    vcpus: machine.vcpus,
    memory: machine.memory,
    hourly: dollarsOf(machine.hourly),
    monthly: dollarsOf(machine.monthly),
}));

// render the generated module
/** The generated pricing module source. */
const generatedSource = await formatSource(
    generatedFile,
    `/** The usage rates in US dollars, per GB-month, GB-hour or million. */\n` +
        `export const rates = ${JSON.stringify(rates)} as const;\n\n` +
        `/** The currencies Destack bills in. */\n` +
        `export const currencies = ${JSON.stringify(CURRENCIES)} as const;\n\n` +
        `/** A US dollar of rate in each currency. */\n` +
        `export const creditPrices = ${JSON.stringify(creditPrices)} as const;\n\n` +
        `/** The plans, cheapest first, each fee a month in each currency. */\n` +
        `export const plans = ${JSON.stringify(plans)} as const;\n\n` +
        `/** The free plan's monthly allowance, in GB and GB-hours. */\n` +
        `export const allowance = ${JSON.stringify(allowance)} as const;\n\n` +
        `/** The dedicated machines, smallest first, priced in US dollars. */\n` +
        `export const machines = ${JSON.stringify(machines)} as const;\n`,
);

/** Return the limit a free plan grant sets, refusing a grant without one. */
function limitOf(value: unknown, feature: string): number {
    if (typeof value !== "number") {
        throw new TypeError(`the free plan sets no ${feature} limit`);
    }

    return value;
}

/** Return a unit's price in US dollars for a quantity of its meter, such as a gigabyte of bytes. */
function rateOf(unit: typeof memoryUnit, quantity: number): number {
    const credits = Number(unit.definition.unitAmount) / MINOR_UNITS;

    return roundOf(credits * (quantity / unit.definition.unitSize) * Number(CREDIT_PRICES.USD));
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
if (isCheck) {
    if (!existsSync(generatedFile) || readFileSync(generatedFile, "utf8") !== generatedSource) {
        throw new Error("generated pricing is out of date, run `just @platform/site/generate`");
    }
} else {
    mkdirSync(dirname(generatedFile), { recursive: true });
    const temporaryFile = `${generatedFile}.${process.pid}.tmp`;
    writeFileSync(temporaryFile, generatedSource);
    renameSync(temporaryFile, generatedFile);
}
