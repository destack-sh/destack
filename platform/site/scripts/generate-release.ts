import { existsSync, mkdirSync, readFileSync, renameSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { formatSource } from "@destack/check";

const repositoryDirectory = resolve(dirname(fileURLToPath(import.meta.url)), "../../..");
const generatedFile = join(repositoryDirectory, "platform/site/src/generated/release.ts");
const publicFile = join(repositoryDirectory, "platform/site/public/release.json");
const isCheck = process.argv.includes("--check");

// read the repository version and release configuration
const manifestFile = join(repositoryDirectory, "package.json");
const manifest = JSON.parse(readFileSync(manifestFile, "utf8"));
const { version } = manifest;
const configurationFile = join(repositoryDirectory, "dev/release/config.json");
const { stability } = JSON.parse(readFileSync(configurationFile, "utf8"));

// require one canonical calendar version
if (!/^\d{4}\.(?:[1-9]|1[0-2])\.\d+$/.test(version)) {
    throw new Error(`invalid calendar version: ${version}`);
}

// require one canonical stability
if (!["experimental", "alpha", "beta", "stable"].includes(stability)) {
    throw new Error(`invalid release stability: ${stability}`);
}

// render both generated representations
const release = { version, stability };
const generatedSource = await formatSource(
    generatedFile,
    `/** The current Destack release. */\n` +
        `export const release = ${JSON.stringify(release)} as const;\n`,
);
const publicSource = `${JSON.stringify(release)}\n`;

// check or replace both representations together
if (isCheck) {
    requireGeneratedFile(generatedFile, generatedSource);
    requireGeneratedFile(publicFile, publicSource);
} else {
    writeGeneratedFile(generatedFile, generatedSource);
    writeGeneratedFile(publicFile, publicSource);
}

/// Require one generated file to match its expected contents.
function requireGeneratedFile(file: string, source: string) {
    if (!existsSync(file)) {
        throw new Error(`missing generated release file: ${file}`);
    }

    const current = readFileSync(file, "utf8");
    if (current !== source) {
        throw new Error(
            "generated release metadata is out of date, run `just platform/site/generate`",
        );
    }
}

/// Atomically replace one generated file.
function writeGeneratedFile(file: string, source: string) {
    mkdirSync(dirname(file), { recursive: true });
    const temporaryFile = `${file}.${process.pid}.tmp`;

    writeFileSync(temporaryFile, source);
    renameSync(temporaryFile, file);
}
