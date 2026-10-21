import { expect, test } from "@destack/test";
import { isRuntimeModule } from "./runtime.ts";

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
