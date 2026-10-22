import { DeclarationDescription } from "@destack/package";
import { test } from "@destack/test";
import { mkdir, mkdtemp, realpath, rm, symlink, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { schema } from "@destack/schema";
import { TestDeclaration } from "../src/typescript/test.ts";
import { Workspace, CompilerPool } from "../src/index.ts";
import { LocalPackageStore } from "../src/store/local.ts";
import { THING_FILES } from "./fixture.ts";

/** The installed vitest, whose test function the inspected tests import. */
const VITEST = dirname(fileURLToPath(import.meta.resolve("vitest/package.json")));

/** Retain test discovery without collecting test bodies' declarations or fixture extensions. */
test("inspect test bodies and fixture extensions without declaring what they construct", async ({
    expect,
}) => {
    // inspect an app declaring one thing, with vitest installed beside its kind package
    const directory = await realpath(await mkdtemp(join(tmpdir(), "destack-inspect-")));
    try {
        for (const [path, text] of Object.entries(THING_FILES)) {
            await mkdir(dirname(join(directory, path)), { recursive: true });
            await writeFile(join(directory, path), text);
        }
        await symlink(VITEST, join(directory, "node_modules", "vitest"));
        expect(Bun.spawnSync(["git", "init", "--quiet", directory]).exitCode).toBe(0);
        await using store = await LocalPackageStore.temporary();
        await using compilers = await CompilerPool.open(store);
        const workspace = await Workspace.open(directory, compilers);
        const dependencies = await workspace.storeDependencies({});
        const inspect = async () =>
            await compilers.use(directory, (compiler) =>
                compiler.inspect({ directory, runtime: "bun", dependencies }),
            );
        const before = await inspect();
        const description = schema.object({
            tests: schema.array(TestDeclaration),
            declarations: schema.array(DeclarationDescription),
        });
        const original = description.parse(before.descriptions);

        // construct a thing in a registered test beside a fixture extension, then inspect again
        const source = `import { test } from "vitest";
import { defineThing } from "@example/owner";
export const extended = test.extend<{ name: string }>({ name: "test-only" });
test("construct a thing", () => {
    const thing = defineThing({ name: "test-only" });
    void thing;
});
`;
        await writeFile(join(directory, "src/app.test.ts"), source);
        const after = await inspect();

        // keep the one declaration and add only the statically collected test
        expect(description.parse(after.descriptions)).toEqual({
            declarations: original.declarations,
            tests: [
                ...original.tests,
                {
                    kind: "test",
                    name: "construct a thing",
                    suites: [],
                    file: "src/app.test.ts",
                    start: source.indexOf('test("'),
                    end: source.lastIndexOf(";"),
                    modifiers: [],
                },
            ],
        });
        expect(original.declarations.map((declaration) => declaration.name)).toEqual(["greeting"]);
    } finally {
        await rm(directory, { recursive: true });
    }
});
