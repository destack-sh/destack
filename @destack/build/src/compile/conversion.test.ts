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
        paths: ["_rolldown/_virtual__rolldown_runtime.js", "index.js", "package.json"],
        elapsed: 0,
        isFired: false,
    });
});

test("convert a package to the same files wherever its copy is converted, emptied modules included", async () => {
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

    // write the same paths and bytes from both roots, emptying the replaced module and dependency
    const [first, second] = [await convert(), await convert()];
    expect(second).toEqual(first);
    expect(first.map(([path]) => path)).toEqual([
        "package.json",
        "lib/index.js",
        "lib/node.js",
        "_rolldown/_virtual__destack_empty.js",
        "_rolldown/_virtual__rolldown_empty_js_lib_node.js",
        "_rolldown/_virtual__rolldown_runtime.js",
    ]);
});
