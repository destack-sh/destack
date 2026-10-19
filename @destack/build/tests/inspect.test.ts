import { DeclarationDescription } from "@destack/package";
import { test } from "@destack/test";
import { readFile, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { schema } from "@destack/schema";
import { TestDeclaration } from "../src/typescript/test.ts";
import { Compiler } from "../src/build/compiler.ts";
import { Fixture } from "./fixture.ts";

/** Retain test discovery without collecting test bodies' resources or fixture extensions. */
test("inspect test bodies and fixture extensions without declaring their resources", async ({
    expect,
}) => {
    await using input = await Fixture.open("web");
    await exportApplication(input.source);
    await using compiler = await Compiler.start(input.source);
    const before = await compiler.inspect({ runtime: "bun" });
    const description = schema.object({
        tests: schema.array(TestDeclaration),
        declarations: schema.array(DeclarationDescription),
    });
    const original = description.parse(before.descriptions);

    // exercise a real declaration constructor in a registered test beside a fixture extension
    const vault = fileURLToPath(new URL("../../vault/src/declare/index.ts", import.meta.url));
    const source = `import { test } from "@destack/test";
import { defineSecret } from ${JSON.stringify(vault)};
export const extended = test.extend<{ name: string }>({ name: "test-only" });
test("bind a secret", () => {
    const secret = defineSecret({ name: "test-only" });
    void secret;
});
`;
    await writeFile(join(input.source, "src/resource.test.ts"), source);
    const after = await compiler.inspect({ runtime: "bun" });

    // compare the complete declaration collection and the statically collected test
    expect(description.parse(after.descriptions)).toEqual({
        declarations: original.declarations,
        tests: [
            ...original.tests,
            {
                kind: "test",
                name: "bind a secret",
                suites: [],
                file: "src/resource.test.ts",
                start: source.indexOf('test("'),
                end: source.lastIndexOf(";"),
                modifiers: [],
            },
        ],
    });
});

/** Export the web fixture's application module as its package root. */
async function exportApplication(source: string): Promise<void> {
    const path = join(source, "package.json");
    const manifest = schema.looseObject({}).parse(JSON.parse(await readFile(path, "utf8")));
    await writeFile(path, JSON.stringify({ ...manifest, exports: { ".": "./src/app.tsx" } }));
}
