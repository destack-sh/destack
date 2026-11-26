import { expect, onTestFinished, test } from "@destack/test";
import type { Environment } from "@destack/environment";
import { dirname, join } from "node:path";
import { testSimulation } from "../test/world.ts";
import { NpmConversion } from "./conversion.ts";

/** Encode a file's text. */
function encode(text: string): Uint8Array<ArrayBuffer> {
    return new TextEncoder().encode(text);
}

/** Install a package's files in a new directory, as Bun's store lays out an installed copy. */
async function install(
    files: ReadonlyMap<string, Uint8Array<ArrayBuffer>>,
    environment: Pick<Environment<"files">, "files">,
): Promise<string> {
    // write each file below a new directory, removed after the test
    const directory = await environment.files.makeTemporaryDirectory("destack-installed-");
    onTestFinished(() => environment.files.remove(directory, { recursive: true, force: true }));
    for (const [path, bytes] of files) {
        await environment.files.mkdir(dirname(join(directory, path)), { recursive: true });
        await environment.files.writeFile(join(directory, path), bytes);
    }

    return directory;
}

test("convert a CommonJS package to ESM for workerd while the world's time stands still", async () => {
    // arm a timer the world would jump to were the bundler's work untracked
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    const started = environment.clock.now();
    let isFired = false;
    void environment.clock.sleep(1000).then(() => {
        isFired = true;
    });

    // convert a package exporting one CommonJS module
    const files = new Map([
        [
            "package.json",
            encode(JSON.stringify({ name: "answer", version: "1.0.0", main: "index.js" })),
        ],
        ["index.js", encode("module.exports = { answer: 42 };")],
    ]);
    const converted = await new NpmConversion("answer", [], [], "workerd", environment).convert(
        await install(files, environment),
        files,
    );

    // answer the converted module's export and files at the same moment, the timer still armed
    expect({
        exports: converted.exports,
        paths: [...converted.files.keys()].toSorted(),
        elapsed: environment.clock.now() - started,
        isFired,
    }).toEqual({
        exports: { ".": "index.js", "./index.js": "index.js" },
        paths: ["index.js"],
        elapsed: 0,
        isFired: false,
    });
});

test("convert a package to the same files wherever its copy is converted, bundling emptied modules in", async () => {
    // convert a package whose browser field empties a module, in two worlds with their own temporary roots
    const files = new Map([
        [
            "package.json",
            encode(
                JSON.stringify({
                    name: "styles",
                    version: "1.0.0",
                    main: "lib/index.js",
                    browser: { "./lib/node": false, "source-map-js": false, path: false },
                }),
            ),
        ],
        [
            "lib/index.js",
            encode(
                'const node = require("./node");\nconst map = require("source-map-js");\nconst path = require("path");\nmodule.exports = { node, map, path };',
            ),
        ],
        ["lib/node.js", encode("module.exports = 'node';")],
    ]);
    const convert = async () => {
        await using simulation = testSimulation();
        const environment = simulation.environment("converter");
        const converted = await new NpmConversion(
            "styles",
            ["source-map-js"],
            [],
            "browser",
            environment,
        ).convert(await install(files, environment), files);

        return [...converted.files].map(([path, bytes]) => [path, new TextDecoder().decode(bytes)]);
    };

    // write the same bytes from both roots, the replaced module and dependency emptied inside the export's chunk
    const [first, second] = [await convert(), await convert()];
    expect(second).toEqual(first);
    expect(first.map(([path]) => path)).toEqual(["lib/index.js"]);
});

test("keep the converted modules with the files they load and the license, leaving the rest", async () => {
    // convert a package importing a stylesheet, and a module below it loading a wasm file and a worker by URL
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    const files = new Map([
        [
            "package.json",
            encode(JSON.stringify({ name: "engine", version: "1.0.0", main: "index.js" })),
        ],
        ["LICENSE", encode("MIT")],
        ["README.md", encode("# engine")],
        ["index.d.ts", encode("export declare const engine: URL;")],
        ["src/index.ts", encode("export const engine = 1;")],
        ["index.js", encode('import "./style.css";\nexport * from "./lib/engine.js";')],
        [
            "lib/engine.js",
            encode(
                'export const engine = new URL("./engine.wasm", import.meta.url);\nexport const worker = new URL("./worker.js", import.meta.url);',
            ),
        ],
        ["style.css", encode("body {}")],
        ["lib/engine.wasm", encode("wasm")],
        ["lib/worker.js", encode('fetch(new URL("./data.bin", import.meta.url));')],
        ["lib/data.bin", encode("data")],
        ["unused.js", encode("export const unused = true;")],
    ]);
    const converted = await new NpmConversion("engine", [], [], "browser", environment).convert(
        await install(files, environment),
        files,
    );

    // bundle the module into its own chunk, importable by path, its URLs resolved from there, keeping the files they load beside it
    const chunk = new TextDecoder().decode(converted.files.get("lib/engine.js"));
    expect({
        paths: [...converted.files.keys()].toSorted(),
        urls: [...chunk.matchAll(/new URL\("([^"]+)", import\.meta\.url\)/gu)].map(
            ([, url]) => url,
        ),
    }).toEqual({
        paths: [
            "LICENSE",
            "index.js",
            "lib/data.bin",
            "lib/engine.js",
            "lib/engine.wasm",
            "lib/worker.js",
            "style.css",
            "unused.js",
        ],
        urls: ["engine.wasm", "worker.js"],
    });
});
