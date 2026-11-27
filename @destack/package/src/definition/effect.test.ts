import { expect, test } from "@destack/test";
import { SideEffects } from "./effect.ts";

test("tell the modules with side effects, as webpack and Rollup read package.json's globs", () => {
    // read each declaration against modules at the root and below directories
    const paths = ["index.js", "src/style.css", "./lib/polyfill.js", "lib/a/polyfill.js", "x.ts"];
    const declared: readonly SideEffects[] = [
        true,
        false,
        ["*.css"],
        ["./lib/*.js"],
        ["lib/**/polyfill.js"],
        ["?.ts"],
    ];

    expect(declared.map((value) => paths.map((path) => SideEffects.has(value, path)))).toEqual([
        [true, true, true, true, true],
        [false, false, false, false, false],
        [false, true, false, false, false],
        [false, false, true, false, false],
        [false, false, true, true, false],
        [false, false, false, false, true],
    ]);
});
