import { existsSync, mkdirSync, readFileSync, renameSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { formatSource } from "@destack/check";
import { durableObjectDuration, r2Storage } from "@platform/pricing/cloudflare";

/** The markup over provider cost the homepage assumes, a typical reseller's 30 to 40 percent. */
const MARKUP = 1.35;

/** The seconds in an hour. */
const SECONDS_PER_HOUR = 3600;

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

// price usage at the provider's list prices, in cents, with the markup on top
/** The usage rates the site shows in US dollars: storage a GB-month and compute a GB-hour. */
const rates = {
    storage: roundOf(
        (Number(r2Storage.definition.unitAmount) / 100) *
            (GIGABYTE / r2Storage.definition.unitSize) *
            MARKUP,
    ),
    compute: roundOf(
        (Number(durableObjectDuration.definition.unitAmount) /
            100 /
            durableObjectDuration.definition.unitSize) *
            SECONDS_PER_HOUR *
            MARKUP,
    ),
};

// render the generated module
/** The generated rates module source. */
const generatedSource = await formatSource(
    generatedFile,
    `/** The homepage's usage rates in US dollars: storage a GB-month and compute a GB-hour. */\n` +
        `export const rates = ${JSON.stringify(rates)} as const;\n`,
);

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
