import { testSimulation } from "@destack/test/world";
import type { Environment } from "@destack/environment";
import { PackageVersion } from "@destack/package";
import { schema } from "@destack/schema";
import { BuildWriter } from "@destack/package/manifest";
import { expect, test } from "@destack/test";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { build, type Plugin, type Rolldown } from "vite";
import { styleExtension } from "./build.ts";

/** The dependency whose build keeps plum's rules. */
const DEPENDENCY = PackageVersion.parse({
    id: "package-01996ab0-0000-7000-8000-0000000000d1",
    name: "@destack/style-dependency",
    version: "2026.9.0",
});

/** The package directory, where the fixture modules resolve `@destack/style`. */
const ROOT = fileURLToPath(new URL("../..", import.meta.url));

/** Serve modules by name: an entry, a lazy page with a stylesheet, and one-color StyleX styles. */
function fixtures(): Plugin {
    return {
        name: "fixtures",
        resolveId: (id) =>
            id.startsWith("/fixture/")
                ? `${ROOT}src/build${id}${id.endsWith(".css") ? "" : ".ts"}`
                : undefined,
        load: (id) => {
            // write the entry, which loads the sheet and a style and the page later, and one loading only the page
            if (id.endsWith("/fixture/entry.ts")) {
                return `import "virtual:stylex.css";\nimport "/fixture/teal";\nvoid import("/fixture/page");\n`;
            } else if (id.endsWith("/fixture/bare.ts")) {
                return `void import("/fixture/page");\n`;
            } else if (id.endsWith("/fixture/pair.ts")) {
                return `import "/fixture/teal";\nimport "/fixture/plum";\n`;
            } else if (id.endsWith("/fixture/solo.ts")) {
                return `import "virtual:stylex.css";\nimport "/fixture/teal";\n`;
            }
            // write two entries sharing the module that loads the sheet and a style
            else if (id.endsWith("/fixture/one.ts") || id.endsWith("/fixture/two.ts")) {
                return `import "/fixture/shared";\nvoid import("/fixture/page");\n`;
            } else if (id.endsWith("/fixture/shared.ts")) {
                return `import "virtual:stylex.css";\nimport "/fixture/teal";\n`;
            }
            // write the lazy page, with a stylesheet of its own and a style
            else if (id.endsWith("/fixture/page.ts")) {
                return `import "/fixture/page.css";\nimport "/fixture/plum";\n`;
            } else if (id.endsWith("/fixture/page.css")) {
                return ".page { margin: 0 }\n";
            }

            // write a module whose style stands under a constant another module defines
            else if (id.endsWith("/fixture/hover.ts")) {
                return `import * as style from "@destack/style";\nimport { media } from "../../style/media.stylex.ts";\nexport const styles = style.create({ text: { color: { default: "teal", ":hover": { default: null, [media.hover]: "plum" } } } });\n`;
            }

            // write a module whose one style sets the color its name gives
            const color = /\/fixture\/([a-z]+)\.ts$/u.exec(id)?.[1];
            if (color === undefined) {
                return undefined;
            }

            return `import * as style from "@destack/style";\nexport const styles = style.create({ text: { color: "${color}" } });\n`;
        },
    };
}

/** Build entries with styles compiled as a package that uses `@destack/style` does. */
async function bundle(
    environment: Environment<"files">,
    entry: string | readonly string[],
    dependencies: Readonly<Record<string, string>> = {},
): ReturnType<typeof build> {
    return await build({
        root: ROOT,
        configFile: false,
        logLevel: "silent",
        plugins: [
            fixtures(),
            styleExtension.transform({
                directory: ROOT,
                dependencies,
                runtime: "browser",
                server: false,
                mode: "release",
                environment,
            }),
        ],
        build: { write: false, minify: false, rolldownOptions: { input: [entry].flat() } },
    });
}

/** Build entries and read each stylesheet by the names of the chunks that load it. */
async function stylesheets(
    environment: Environment<"files">,
    entry: string | readonly string[],
    dependencies: Readonly<Record<string, string>> = {},
): Promise<Record<string, string>> {
    // build the entries
    const output = await bundle(environment, entry, dependencies);
    const results = Array.isArray(output) ? output : [output];
    const files = results.flatMap(
        (result): readonly (Rolldown.OutputChunk | Rolldown.OutputAsset)[] =>
            "output" in result ? result.output : [],
    );

    return Object.fromEntries(
        files
            .filter(
                (file): file is Rolldown.OutputAsset =>
                    file.type === "asset" && file.fileName.endsWith(".css"),
            )
            .map((file): [string, string] => [file.names.join(", "), String(file.source)]),
    );
}

test("write every rule into the build's own stylesheet beside the entry's, leaving lazy pages' stylesheets", async () => {
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    expect(await stylesheets(environment, "/fixture/entry")).toEqual({
        "page.css": ".page { margin: 0 }\n",
        "stylex.css":
            "@layer base;\n\n@layer priority1 {\n  .xjjgwud {\n    color: plum;\n  }\n\n  .xy1w8w {\n    color: teal;\n  }\n}\n",
    });
});

test("write every rule into one stylesheet for a chunk several entries share", async () => {
    // read the sheets whatever the bundler names the shared chunk
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    const sheets = await stylesheets(environment, ["/fixture/one", "/fixture/two"]);

    expect(Object.values(sheets).toSorted()).toEqual([
        ".page { margin: 0 }\n",
        "@layer base;\n\n@layer priority1 {\n  .xjjgwud {\n    color: plum;\n  }\n\n  .xy1w8w {\n    color: teal;\n  }\n}\n",
    ]);
});

test("keep a build's rules for the builds depending on it when no module imports the sheet", async () => {
    // read the rules file the bundle keeps
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    const result = await bundle(environment, "/fixture/bare");
    const outputs = (Array.isArray(result) ? result : [result]).flatMap((each) =>
        "output" in each ? each.output : [],
    );
    const rules = outputs.find((each) => each.fileName === "stylex.json");
    const source = rules?.type === "asset" ? String(rules.source) : undefined;

    expect(source).toEqual(
        '[["src/build/fixture/plum.ts",[["xjjgwud",{"ltr":".xjjgwud{color:plum}","rtl":null},3000]]]]',
    );
});

test("keep a build's rules sorted by module path, whatever order the modules compile in", async () => {
    // read the rules file of a bundle compiling teal before plum
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    const result = await bundle(environment, "/fixture/pair");
    const outputs = (Array.isArray(result) ? result : [result]).flatMap((each) =>
        "output" in each ? each.output : [],
    );
    const rules = outputs.find((each) => each.fileName === "stylex.json");
    const source = rules?.type === "asset" ? String(rules.source) : "";

    // list plum before teal, so equal sources keep equal bytes
    const modules = schema
        .array(schema.tuple([schema.string(), schema.unknown()]))
        .parse(JSON.parse(source))
        .map(([module]) => module);
    expect(modules).toEqual(["src/build/fixture/plum.ts", "src/build/fixture/teal.ts"]);
});

test("add the rules each dependency's build keeps to the stylesheet, ordered with the build's own", async () => {
    // keep a dependency's rules as its build keeps them
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    const { files } = environment;
    const dependency = await files.makeTemporaryDirectory("style-dependency-");
    const rules = JSON.stringify([
        ["src/plum.ts", [["xjjgwud", { ltr: ".xjjgwud{color:plum}", rtl: null }, 3000]]],
    ]);

    // write the dependency's build with its rules file
    const writer = new BuildWriter(
        {
            write: async (path, bytes) => {
                await files.mkdir(dirname(join(dependency, path)), { recursive: true });
                await files.writeFile(join(dependency, path), bytes);
            },
        },
        environment.crypto,
    );
    await writer.write("output/library/stylex.json", new TextEncoder().encode(rules));
    await writer.finish({ destack: "2026.11.0", role: "app", package: DEPENDENCY });

    // build teal with the dependency's plum
    const sheets = await stylesheets(environment, "/fixture/solo", {
        "@destack/style-dependency": dependency,
    });

    expect(sheets).toEqual({
        "stylex.css":
            "@layer base;\n\n@layer priority1 {\n  .xjjgwud {\n    color: plum;\n  }\n\n  .xy1w8w {\n    color: teal;\n  }\n}\n",
    });
});

test("keep only a build's own rules after an earlier build in the same process", async () => {
    // build a bundle using plum and teal, then one using teal alone
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    await bundle(environment, "/fixture/entry");
    const sheets = await stylesheets(environment, "/fixture/solo");

    expect(sheets).toEqual({
        "stylex.css": "@layer base;\n\n@layer priority1 {\n  .xy1w8w {\n    color: teal;\n  }\n}\n",
    });
});
