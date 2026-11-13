import { testSimulation } from "../src/test/world.ts";
import { DeclarationDescription } from "@destack/package";
import { expect, refusal, test } from "@destack/test";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { present, schema } from "@destack/schema";
import { TestDeclaration } from "../src/typescript/test.ts";
import { Workspace, CompilerPool } from "../src/index.ts";
import { LocalBuildStore } from "../src/store/local.ts";
import { THING_FILES, writeThing } from "./fixture.ts";
import { Toolchain } from "@destack/check/toolchain";

/** The app module declaring one thing. */
const THING_APP = present(THING_FILES["src/app.ts"], "the thing app");

/** The installed vitest, whose test function the inspected tests import. */
const VITEST = dirname(fileURLToPath(import.meta.resolve("vitest/package.json")));

/** Retain test discovery without collecting test bodies' declarations or fixture extensions. */
test("inspect test bodies and fixture extensions without declaring what they construct", async () => {
    // inspect an app declaring one thing, with vitest installed beside its kind package
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    const { files } = environment;
    const root = await files.realPath(await files.makeTemporaryDirectory("destack-inspect-"));
    const directory = await writeThing(root, THING_APP, environment);
    await files.symlink(VITEST, join(directory, "node_modules", "vitest"));
    await using store = await LocalBuildStore.temporary(environment);
    await using compilers = await CompilerPool.open(store, async () => Toolchain.workspace());
    const workspacePackage = await (await Workspace.open(directory, compilers)).package();
    const dependencies = await workspacePackage.storeDependencies({});
    const inspect = async () =>
        await compilers.use(directory, (compiler) =>
            compiler.inspect({ directory, runtime: "bun", dependencies, links: {} }),
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
    await files.writeTextFile(join(directory, "src/app.test.ts"), source);
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
});

/** Inspect an app module beside the thing kind's package, answering its declarations. */
async function inspectApp(app: string): Promise<readonly string[]> {
    // write the app beside its kind's package in one workspace
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    const root = await environment.files.realPath(
        await environment.files.makeTemporaryDirectory("destack-inspect-"),
    );
    const directory = await writeThing(root, app, environment);

    // inspect it on its dependencies' builds
    await using store = await LocalBuildStore.temporary(environment);
    await using compilers = await CompilerPool.open(store, async () => Toolchain.workspace());
    const dependencies = await (
        await (await Workspace.open(directory, compilers)).package()
    ).storeDependencies({});
    const inspected = await compilers.use(directory, (compiler) =>
        compiler.inspect({ directory, runtime: "bun", dependencies, links: {} }),
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

test("refuse a declaration its module does not export, leaving one a factory makes", async () => {
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
    expect([await refusal(inspectApp(app)), await inspectApp(factory)]).toEqual([
        [
            "BAD_REQUEST",
            "DECLARATION_INVALID",
            "declared 'hidden' is invalid: '@example/app' does not export it",
        ],
        ["greeting"],
    ]);
});
