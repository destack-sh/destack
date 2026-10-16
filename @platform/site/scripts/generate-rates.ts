import { existsSync, mkdirSync, readFileSync, renameSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { formatSource } from "@destack/check";
import {
    cpuUnit,
    CREDIT_PRICES,
    databaseBytesUnit,
    memoryUnit,
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

/** The minor units of credits in a credit. */
const CREDIT_MINOR_UNITS = 100;

/** The bytes in a gigabyte. */
const GIGABYTE = 1_000_000_000;

/** The repository root. */
const repositoryDirectory = resolve(dirname(fileURLToPath(import.meta.url)), "../../..");
/** The generated rates module. */
const generatedFile = join(
    repositoryDirectory,
    "@platform/site/src/view/content/generated/rates.ts",
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

// render the generated module
/** The generated rates module source. */
const generatedSource = await formatSource(
    generatedFile,
    `/** The homepage's usage rates in US dollars, per GB-month, GB-hour or million. */\n` +
        `export const rates = ${JSON.stringify(rates)} as const;\n`,
);

/** Return a unit's price in US dollars for a quantity of its meter, such as a gigabyte of bytes. */
function rateOf(unit: typeof memoryUnit, quantity: number): number {
    const credits = Number(unit.definition.unitAmount) / CREDIT_MINOR_UNITS;

    return roundOf(credits * (quantity / unit.definition.unitSize) * Number(CREDIT_PRICES.USD));
}

/** Round a dollar rate to the thousandth of a cent, past the noise of binary fractions. */
function roundOf(amount: number): number {
    return Math.round(amount * 100_000) / 100_000;
}

// check or replace the module
if (isCheck) {
    if (!existsSync(generatedFile) || readFileSync(generatedFile, "utf8") !== generatedSource) {
        throw new Error("generated rates are out of date, run `just @platform/site/generate`");
    }
} else {
    mkdirSync(dirname(generatedFile), { recursive: true });
    const temporaryFile = `${generatedFile}.${process.pid}.tmp`;
    writeFileSync(temporaryFile, generatedSource);
    renameSync(temporaryFile, generatedFile);
}
