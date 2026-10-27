import { expect, test } from "@destack/test";
import { isRuntimeModule, runtimeCss, runtimeTarget } from "./runtime.ts";

test("supply every host module on Bun, workerd's nodejs_compat modules and stubs on workerd, and none in browsers", () => {
    const specifiers = [
        "node:crypto",
        "crypto",
        "node:stream/web",
        "node:fs",
        "node:tty",
        "node:sqlite",
        "bun:sqlite",
    ];
    expect(
        Object.fromEntries(
            (["bun", "workerd", "browser"] as const).map((runtime) => [
                runtime,
                specifiers.filter((specifier) => isRuntimeModule(specifier, runtime)),
            ]),
        ),
    ).toEqual({
        bun: specifiers,
        workerd: ["node:crypto", "crypto", "node:stream/web", "node:fs", "node:tty", "node:sqlite"],
        browser: [],
    });
});

test("compile browser modules to the baseline browsers and server modules to current syntax, stylesheets too", () => {
    expect({
        browser: runtimeTarget("browser"),
        bun: runtimeTarget("bun"),
        workerd: runtimeTarget("workerd"),
        css: runtimeCss("browser"),
        serverCss: runtimeCss("bun"),
    }).toEqual({
        browser: ["chrome124", "edge124", "firefox125", "safari17.4"],
        bun: ["esnext"],
        workerd: ["esnext"],
        // each version as Lightning CSS packs it, the major in the third byte and the minor in the second
        css: {
            transformer: "lightningcss",
            lightningcss: {
                targets: {
                    chrome: 124 << 16,
                    edge: 124 << 16,
                    firefox: 125 << 16,
                    safari: (17 << 16) | (4 << 8),
                },
                // light-dark() stays native, as Lightning CSS's lowering needs a color-scheme rule
                exclude: 1 << 20,
            },
        },
        serverCss: {},
    });
});
