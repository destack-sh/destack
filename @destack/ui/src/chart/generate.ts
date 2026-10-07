import { mkdir, readFile, rm, writeFile } from "node:fs/promises";
import { dirname } from "node:path";
import { schema } from "@destack/schema";

/** The first line of each generated module. */
const HEADER = "// generate with `bun run generate` from @tanstack/charts' entry points";

/** The entry points binding other frameworks, which charts in views never use. */
const FRAMEWORKS = /^(alpine|angular|lit|octane|preact|react|react-native|solid|svelte|vue)(\/|$)/u;

/** The directory of the generated modules. */
const MODULES = new URL("module/", import.meta.url);

/** The part of a package manifest naming its entry points. */
const Manifest = schema.looseObject({ exports: schema.record(schema.string(), schema.unknown()) });

/** The upstream manifest. */
const manifest = Manifest.parse(
    JSON.parse(
        await readFile(new URL(import.meta.resolve("@tanstack/charts/package.json")), "utf8"),
    ),
);

/** The upstream entry points beside the root, which the chart module itself re-exports. */
const entries = Object.keys(manifest.exports)
    .filter((key) => key.startsWith("./") && key !== "./package.json")
    .map((key) => key.slice(2))
    .filter((entry) => !FRAMEWORKS.test(entry))
    .toSorted();

// write one module re-exporting each entry point
await rm(MODULES, { recursive: true, force: true });
for (const entry of entries) {
    const file = new URL(`${entry}.ts`, MODULES);
    await mkdir(dirname(file.pathname), { recursive: true });
    await writeFile(file, `${HEADER}\n\nexport * from "@tanstack/charts/${entry}";\n`);
}
