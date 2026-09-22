import { test } from "@destack/test";
import { readFile, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { schema } from "@destack/schema";
import { DeclarationDescription } from "@destack/package/inspect";
import { TestDeclaration } from "@destack/test/inspect";
import { PackageBuilder } from "../src/build/builder.ts";
import { Fixture } from "./fixture.ts";

/** Retain test discovery without collecting the resources constructed by test bodies. */
test("inspect test bodies without declaring their temporary resources", async ({ expect }) => {
    await using input = await Fixture.open("web");
    const manifest = join(input.source, "package.json");
    const metadata = JSON.parse(await readFile(manifest, "utf8"));
    await writeFile(manifest, JSON.stringify({ ...metadata, exports: { ".": "./src/App.tsx" } }));
    await using compiler = await PackageBuilder.start(input.source);
    const before = await compiler.inspect({ target: "server", runtime: "bun" });
    const description = schema.object({
        tests: schema.array(TestDeclaration),
        declarations: schema.array(DeclarationDescription),
    });
    const original = description.parse(before.descriptions);

    // exercise a real declaration constructor inside an ordinary registered test
    const vault = fileURLToPath(new URL("../../vault/src/index.ts", import.meta.url));
    const source = `import { test } from "@destack/test";
import { defineSecret } from ${JSON.stringify(vault)};
test("bind a secret", () => {
    const secret = defineSecret({ name: "test-only" });
    void secret;
});
`;
    await writeFile(join(input.source, "src/resource.test.ts"), source);
    const after = await compiler.inspect({ target: "server", runtime: "bun" });

    // compare the complete declaration collection and the statically collected test
    expect(description.parse(after.descriptions)).toEqual({
        declarations: original.declarations,
        tests: [
            ...original.tests,
            {
                kind: "test",
                name: "bind a secret",
                file: "src/resource.test.ts",
                start: source.indexOf('test("'),
                end: source.lastIndexOf(";"),
                modifiers: [],
            },
        ],
    });
});

/** Invalid static test declarations. */
const invalid = [
    {
        file: "src/app.test.ts",
        source: 'import { test } from "@destack/test";\ntest(String("renders"), () => {});\n',
        code: "INSPECTION_FAILED",
        message: "Test declaration requires a literal title: src/app.test.ts:38",
    },
    {
        file: "src/app.test.ts",
        source: 'import { test } from "@destack/test";\nif (true) { test("renders", () => {}); }\n',
        code: "INSPECTION_FAILED",
        message: "Test declaration requires module or suite scope: src/app.test.ts:50",
    },
];

test.concurrent.for(invalid)("reject $message", async (fixture, { expect }) => {
    await using input = await Fixture.open("web");
    await writeFile(join(input.source, fixture.file), fixture.source);
    const path = join(input.source, "package.json");
    const metadata = JSON.parse(await readFile(path, "utf8"));
    await writeFile(path, JSON.stringify({ ...metadata, exports: { ".": "./src/App.tsx" } }));
    await using compiler = await PackageBuilder.start(input.source);
    await expect(compiler.inspect({ target: "browser" })).rejects.toMatchObject({
        code: fixture.code,
        message: fixture.message,
    });
});
