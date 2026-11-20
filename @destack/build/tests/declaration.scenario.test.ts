import { testSimulation } from "../src/test/world.ts";
import { expect, refusal, test } from "@destack/test";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { present } from "@destack/schema";
import { Workspace, CompilerPool } from "../src/index.ts";
import { LocalBuildStore } from "../src/store/local.ts";
import { THING_FILES, writeThing } from "./fixture.ts";
import { Toolchain } from "@destack/check/toolchain";

/** The app module declaring one thing. */
const THING_APP = present(THING_FILES["src/app.ts"], "the thing app");

/** The installed vitest, whose test function the inspected tests import. */
const VITEST = dirname(fileURLToPath(import.meta.resolve("vitest/package.json")));

/** Evaluate export entries without declaring what test bodies or fixture extensions construct. */
test("evaluate declarations without what test bodies and fixture extensions construct", async () => {
    // evaluate an app declaring one thing, with vitest installed beside its kind package
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
    const evaluate = async () =>
        await compilers.use(directory, (compiler) =>
            compiler.declarations({ directory, runtime: "bun", dependencies, links: {} }),
        );
    const before = await evaluate();

    // construct a thing in a registered test beside a fixture extension, then evaluate again
    const source = `import { test } from "vitest";
import { defineThing } from "@example/owner";
export const extended = test.extend<{ name: string }>({ name: "test-only" });
test("construct a thing", () => {
    const thing = defineThing({ name: "test-only" });
    void thing;
});
`;
    await files.writeTextFile(join(directory, "src/app.test.ts"), source);
    const after = await evaluate();

    // list the one declaration both times
    expect(after).toEqual(before);
    expect(before.map((declaration) => declaration.name)).toEqual(["greeting"]);
});

/** Evaluate an app module beside the thing kind's package, answering its declarations' names. */
async function evaluateApp(app: string): Promise<readonly string[]> {
    // write the app beside its kind's package in one workspace
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    const root = await environment.files.realPath(
        await environment.files.makeTemporaryDirectory("destack-inspect-"),
    );
    const directory = await writeThing(root, app, environment);

    // evaluate it on its dependencies' builds
    await using store = await LocalBuildStore.temporary(environment);
    await using compilers = await CompilerPool.open(store, async () => Toolchain.workspace());
    const dependencies = await (
        await (await Workspace.open(directory, compilers)).package()
    ).storeDependencies({});
    const declarations = await compilers.use(directory, (compiler) =>
        compiler.declarations({ directory, runtime: "bun", dependencies, links: {} }),
    );

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
    expect(await evaluateApp(app)).toEqual(["greeting", "made"]);
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
    expect([await refusal(evaluateApp(app)), await evaluateApp(factory)]).toEqual([
        [
            "BAD_REQUEST",
            "DECLARATION_INVALID",
            "declared 'hidden' is invalid: '@example/app' does not export it",
        ],
        ["greeting"],
    ]);
});
