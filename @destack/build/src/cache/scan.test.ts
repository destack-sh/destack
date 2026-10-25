import { testSimulation } from "../test/world.ts";
import { dirname, join } from "node:path";
import { expect, test } from "@destack/test";
import { Identities } from "./cache.ts";
import { ProgramImports } from "./scan.ts";

test("follow the modules and files a compiler reads from a package's root files", async () => {
    // write a package whose entry imports by query, by a JavaScript twin, by a directory, by a wildcard subpath and by a directory URL
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    const { files } = environment;
    const root = await files.realPath(await files.makeTemporaryDirectory("destack-scan-"));
    const contents = {
        "package.json": JSON.stringify({
            name: "@example/scan",
            version: "1.0.0",
            imports: { "#shared/*": "./src/shared/*.ts" },
        }),
        "src/index.ts": [
            'import icon from "./icon.svg?url";',
            'import { twin } from "./twin.js";',
            'import { local } from "./local";',
            'import { shared } from "#shared/value";',
            'import { join } from "node:path";',
            'const assets = new URL("./assets/", import.meta.url);',
        ].join("\n"),
        "src/icon.svg": "<svg />",
        "src/twin.ts": "export const twin = 1;",
        "src/local/index.ts": 'export { twin as local } from "../twin.ts";',
        "src/shared/value.ts": "export const shared = 1;",
    };
    for (const [path, text] of Object.entries(contents)) {
        await files.mkdir(dirname(join(root, path)), { recursive: true });
        await files.writeTextFile(join(root, path), text);
    }

    // scan from the entry as the configuration's one root file
    const entry = join(root, "src/index.ts");
    const scanned = await ProgramImports.scan(
        root,
        {
            options: {
                strict: true,
                configFilePath: "/elsewhere/tsconfig.json",
                pathsBasePath: "/elsewhere",
                paths: { "@example/tree": [join(dirname(root), "tree/index.ts")] },
                rootDir: join(root, "src"),
            },
            files: [entry],
        },
        Identities.of({}, environment),
    );
    const targets = (file: string) =>
        scanned.modules
            .get(join(root, file))
            ?.imports.map(({ specifier, target }) => [
                specifier,
                target.kind === "external"
                    ? "external"
                    : [target.kind, target.file.slice(root.length + 1)],
            ]);

    // key option paths by the generated configuration's directory or the package, and follow every module once
    expect({
        options: scanned.options,
        packages: scanned.packages,
        modules: [...scanned.modules.keys()].map((file) => file.slice(root.length + 1)).toSorted(),
        entry: targets("src/index.ts"),
        directories: scanned.modules
            .get(entry)
            ?.directories.map((path) => path.slice(root.length + 1)),
    }).toEqual({
        options: JSON.stringify({
            strict: true,
            configFilePath: "<configuration>/tsconfig.json",
            pathsBasePath: "<configuration>",
            paths: { "@example/tree": ["../tree/index.ts"] },
            rootDir: "src",
        }),
        packages: [],
        modules: ["src/index.ts", "src/local/index.ts", "src/shared/value.ts", "src/twin.ts"],
        entry: [
            ["./icon.svg?url", ["file", "src/icon.svg"]],
            ["./twin.js", ["module", "src/twin.ts"]],
            ["./local", ["module", "src/local/index.ts"]],
            ["#shared/value", ["module", "src/shared/value.ts"]],
            ["node:path", "external"],
        ],
        directories: ["src/assets/"],
    });
});
