import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { formatSource, Toolchain } from "@destack/check";
import { BunEnvironment } from "@destack/environment/bun";
import { schema } from "@destack/schema";

/** The repository root. */
const repositoryDirectory = resolve(dirname(fileURLToPath(import.meta.url)), "../../..");
/** The generated release module. */
const generatedFile = join(
    repositoryDirectory,
    "@platform/site/src/view/content/generated/release.ts",
);
/** The host the script reads and writes files on. */
const environment = BunEnvironment.system;
/** The file system the module is read from and written to. */
const { files } = environment;
/** Whether this run checks the generated files instead of writing them. */
const isCheck = environment.process.arguments.includes("--check");

// read the repository's calendar version and the release's stability
/** The release version. */
const { version } = schema
    .looseObject({ version: schema.string().regex(/^\d{4}\.(?:[1-9]|1[0-2])\.\d+$/u) })
    .parse(JSON.parse(await files.readTextFile(join(repositoryDirectory, "package.json"))));
/** The release stability. */
const { stability } = schema
    .object({ stability: schema.enum(["experimental", "alpha", "beta", "stable"]) })
    .parse(
        JSON.parse(
            await files.readTextFile(join(repositoryDirectory, "@platform/release/config.json")),
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
    Toolchain.workspace(),
);

// check or replace the generated module
if (isCheck) {
    await requireGeneratedFile(generatedFile, generatedSource);
} else {
    await writeGeneratedFile(generatedFile, generatedSource);
}

/** Require one generated file to match its expected contents. */
async function requireGeneratedFile(file: string, source: string) {
    if (!(await files.exists(file))) {
        throw new Error(`generated release file '${file}' not found`);
    }

    const current = await files.readTextFile(file);
    if (current !== source) {
        throw new Error(
            "generated release metadata is out of date, run `just @platform/site/generate`",
        );
    }
}

/** Atomically replace one generated file. */
async function writeGeneratedFile(file: string, source: string) {
    // write a temporary file beside the target and rename it into place
    await files.mkdir(dirname(file), { recursive: true });
    const temporaryFile = `${file}.${environment.process.id}.tmp`;
    await files.writeTextFile(temporaryFile, source);
    await files.rename(temporaryFile, file);
}
