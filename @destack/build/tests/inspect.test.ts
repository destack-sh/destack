import { DeclarationDescription } from "@destack/package";
import { expect, onTestFinished, refusal, test } from "@destack/test";
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
test("inspect test bodies and fixture extensions without declaring what they construct", async () => {
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

        // list the one declaration and add only the statically collected test
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

/** Inspect an app module beside the thing kind's package, answering its declarations. */
async function inspectApp(app: string): Promise<readonly string[]> {
    // write the app with its kind's package installed beside it
    const directory = await realpath(await mkdtemp(join(tmpdir(), "destack-inspect-")));
    onTestFinished(() => rm(directory, { recursive: true }));
    for (const [path, text] of Object.entries({ ...THING_FILES, "src/app.ts": app })) {
        await mkdir(dirname(join(directory, path)), { recursive: true });
        await writeFile(join(directory, path), text);
    }
    expect(Bun.spawnSync(["git", "init", "--quiet", directory]).exitCode).toBe(0);

    // inspect it on its dependencies' builds
    await using store = await LocalPackageStore.temporary();
    await using compilers = await CompilerPool.open(store);
    const dependencies = await (await Workspace.open(directory, compilers)).storeDependencies({});
    const inspected = await compilers.use(directory, (compiler) =>
        compiler.inspect({ directory, runtime: "bun", dependencies }),
    );
    const { declarations } = schema
        .looseObject({ declarations: schema.array(DeclarationDescription) })
        .parse(inspected.descriptions);

    return declarations.map((declaration) => declaration.name);
}

test("describe a factory's declaration by the exported constant its call initializes, leaving a factory of lists", async () => {
    // make a thing in a factory, and two in a factory of a list
    const app = `import { defineThing } from "@example/owner";

function thing(name: string) {
    return defineThing({ name });
}

function family(name: string) {
    return [defineThing({ name: \`\${name}-one\` }), defineThing({ name: \`\${name}-two\` })];
}

export const greeting = defineThing({ name: "greeting" });
export const made = thing("made");
export const members = family("member");
`;

    // describe the constructor's and the factory's exported constants
    expect(await inspectApp(app)).toEqual(["greeting", "made"]);
});

test("refuse a declaration its module does not export, made by its constructor or a factory", async () => {
    const app = `import { defineThing } from "@example/owner";

const hidden = defineThing({ name: "hidden" });
export const greeting = defineThing({ name: "greeting" });
void hidden;
`;
    const factory = `import { defineThing } from "@example/owner";

function thing(name: string) {
    return defineThing({ name });
}

const made = thing("made");
export const greeting = defineThing({ name: "greeting" });
void made;
`;
    expect([await refusal(inspectApp(app)), await refusal(inspectApp(factory))]).toEqual([
        [
            "UNPROCESSABLE_CONTENT",
            "DECLARATION_UNEXPORTED",
            "declared hidden is not exported by @example/app",
        ],
        [
            "UNPROCESSABLE_CONTENT",
            "DECLARATION_UNEXPORTED",
            "declared made is not exported by @example/app",
        ],
    ]);
});
