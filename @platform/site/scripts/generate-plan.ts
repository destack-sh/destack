import { existsSync, mkdirSync, readFileSync, renameSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { formatSource } from "@destack/check";
import { PLUS } from "@platform/pricing";

/** The repository root. */
const repositoryDirectory = resolve(dirname(fileURLToPath(import.meta.url)), "../../..");
/** The generated plan module. */
const generatedFile = join(
    repositoryDirectory,
    "@platform/site/src/view/content/generated/plan.ts",
);
/** Whether this run checks the generated file instead of writing it. */
const isCheck = process.argv.includes("--check");

// read the plan the homepage bills a destacked team on, in whole dollars a month
/** The Plus plan's price in US dollars. */
const price = PLUS.prices.find((candidate) => candidate.currency === "USD");
if (price === undefined) {
    throw new Error("the Plus plan lacks a USD price");
}
/** The plan the site shows. */
const plan = { name: `Destack ${PLUS.name}`, monthly: price.unitAmount / 100 };

// render the generated module
/** The generated plan module source. */
const generatedSource = await formatSource(
    generatedFile,
    `/** The plan the homepage bills a destacked team on, in US dollars a month. */\n` +
        `export const plan = ${JSON.stringify(plan)} as const;\n`,
);

// check or replace the module
if (isCheck) {
    if (!existsSync(generatedFile) || readFileSync(generatedFile, "utf8") !== generatedSource) {
        throw new Error("generated plan is out of date, run `just @platform/site/generate`");
    }
} else {
    mkdirSync(dirname(generatedFile), { recursive: true });
    const temporaryFile = `${generatedFile}.${process.pid}.tmp`;
    writeFileSync(temporaryFile, generatedSource);
    renameSync(temporaryFile, generatedFile);
}
