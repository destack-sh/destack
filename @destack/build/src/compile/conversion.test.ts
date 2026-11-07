import { expect, test } from "@destack/test";
import { testSimulation } from "../test/world.ts";
import { NpmConversion } from "./conversion.ts";

/** Encode a file's text. */
function encode(text: string): Uint8Array<ArrayBuffer> {
    return new TextEncoder().encode(text);
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
        files,
    );

    // answer the converted module's export and files at the same moment, the timer still armed
    expect({
        exports: converted.exports,
        paths: [...converted.files.keys()].toSorted(),
        elapsed: environment.clock.now() - started,
        isFired,
    }).toEqual({
        exports: { ".": "index.js" },
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
        ).convert(files);

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
        files,
    );

    // bundle the module into the export's chunk, its URLs resolved from there, keeping the files they load beside it
    const chunk = new TextDecoder().decode(converted.files.get("index.js"));
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
            "lib/engine.wasm",
            "lib/worker.js",
            "style.css",
        ],
        urls: ["lib/engine.wasm", "lib/worker.js"],
    });
});
