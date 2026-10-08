import { expect, refusal, test } from "@destack/test";
import type { ModuleDescription } from "@destack/package/code";
import { Digest } from "@destack/schema";
import { joinModules } from "./module.ts";

/** Describe the module a runtime inspects: its source digest, the one import and the one global it resolves. */
function described(digest: string, specifier: string, global: string): ModuleDescription {
    return {
        path: "src/platform.ts",
        source: {
            path: "src/platform.ts",
            mediaType: "text/typescript",
            size: 64,
            digest: Digest.parse(digest),
        },
        length: 64,
        imports: [
            { specifier: "./shared.ts", target: { module: "src/shared.ts", name: "shared" } },
            { specifier, target: { module: specifier.slice(2), name: "platform" } },
        ],
        globals: [
            {
                name: global,
                members: [],
                dynamic: false,
                source: { file: "src/platform.ts", start: 0, end: 3 },
            },
        ],
        errors: [],
        symbols: [],
        exports: [],
    };
}

test("join a module each runtime inspects into one description, refusing other source bytes", async () => {
    // describe the module as bun and the browser resolve it
    const digest = "a".repeat(64);
    const bun = described(digest, "./bun.ts", "Bun");
    const browser = described(digest, "./browser.ts", "window");

    // keep the shared import once and each runtime's import and global
    expect(joinModules(bun, browser)).toEqual({
        ...bun,
        imports: [...bun.imports, browser.imports[1]],
        globals: [...bun.globals, ...browser.globals],
    });

    // refuse a description of other source bytes
    const other = described("b".repeat(64), "./browser.ts", "window");
    expect(await refusal(Promise.resolve().then(() => joinModules(bun, other)))).toEqual([
        "BUILD_FAILED",
        "module src/platform.ts was inspected from two sources",
    ]);
});
