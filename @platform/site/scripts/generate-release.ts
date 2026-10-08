import { existsSync, mkdirSync, readFileSync, renameSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { formatSource } from "@destack/check";
import { schema } from "@destack/schema";

/** The repository root. */
const repositoryDirectory = resolve(dirname(fileURLToPath(import.meta.url)), "../../..");
/** The generated release module. */
const generatedFile = join(
    repositoryDirectory,
    "@platform/site/src/view/content/generated/release.ts",
);
/** The public release document. */
const publicFile = join(repositoryDirectory, "@platform/site/public/release.json");
/** Whether this run checks the generated files instead of writing them. */
const isCheck = process.argv.includes("--check");

// read the repository's calendar version and the release's stability
/** The release version. */
const { version } = schema
    .looseObject({ version: schema.string().regex(/^\d{4}\.(?:[1-9]|1[0-2])\.\d+$/u) })
    .parse(JSON.parse(readFileSync(join(repositoryDirectory, "package.json"), "utf8")));
/** The release stability. */
const { stability } = schema
    .object({ stability: schema.enum(["experimental", "alpha", "beta", "stable"]) })
    .parse(
        JSON.parse(
            readFileSync(join(repositoryDirectory, "@platform/release/config.json"), "utf8"),
        ),
    );

// render both generated representations
/** The release the site shows. */
const release = { version, stability };
/** The generated release module source. */
const generatedSource = await formatSource(
    generatedFile,
    `/** The current Destack release. */\n` +
        `export const release = ${JSON.stringify(release)} as const;\n`,
);
/** The public release document source. */
const publicSource = await formatSource(publicFile, `${JSON.stringify(release)}\n`);

// check or replace both representations together
if (isCheck) {
    requireGeneratedFile(generatedFile, generatedSource);
    requireGeneratedFile(publicFile, publicSource);
} else {
    writeGeneratedFile(generatedFile, generatedSource);
    writeGeneratedFile(publicFile, publicSource);
}

/** Require one generated file to match its expected contents. */
function requireGeneratedFile(file: string, source: string) {
    if (!existsSync(file)) {
        throw new Error(`missing generated release file: ${file}`);
    }

    const current = readFileSync(file, "utf8");
    if (current !== source) {
        throw new Error(
            "generated release metadata is out of date, run `just @platform/site/generate`",
        );
    }
}

/** Atomically replace one generated file. */
function writeGeneratedFile(file: string, source: string) {
    // write a temporary file beside the target and rename it into place
    mkdirSync(dirname(file), { recursive: true });
    const temporaryFile = `${file}.${process.pid}.tmp`;
    writeFileSync(temporaryFile, source);
    renameSync(temporaryFile, file);
}
