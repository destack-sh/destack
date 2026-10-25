import { testSimulation } from "../test/world.ts";
import type { Environment, FileSystem } from "@destack/environment";
import { join } from "node:path";
import { Digest } from "@destack/schema";
import { expect, refusal, test } from "@destack/test";
import { Checkout } from "./checkout.ts";

/** Make a temporary directory in a world, at its real path. */
async function temporary(prefix: string, files: FileSystem): Promise<string> {
    return await files.realPath(await files.makeTemporaryDirectory(prefix));
}

/** Write a package's manifest into a directory. */
async function writeManifest(directory: string, name: string, files: FileSystem): Promise<void> {
    await files.mkdir(directory, { recursive: true });
    await files.writeTextFile(join(directory, "package.json"), JSON.stringify({ name }));
}

/** Run git in a directory, failing the test on a failed run. */
async function git(
    environment: Environment<"process">,
    directory: string,
    ...command: string[]
): Promise<string> {
    const ran = environment.process.spawn(["git", "-C", directory, ...command]);
    const output = await new Response(ran.stdout).text();
    expect(await ran.exited).toBe(0);

    return output.trim();
}

test("key a package by the files git lists, reading linked files through their links", async () => {
    // write a package in a git checkout that ignores its build output
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    const { files } = environment;
    const root = await temporary("destack-checkout-", files);
    const directory = join(root, "package");
    await writeManifest(directory, "@example/checkout", files);
    await files.mkdir(join(directory, "src"));
    await files.writeTextFile(join(root, "shared.ts"), "export const shared = 1;\n");
    await files.writeTextFile(join(directory, ".gitignore"), "target/\n");
    await files.writeTextFile(join(directory, "src", "index.ts"), "export {};\n");
    await git(environment, root, "init", "--quiet");
    const key = async () => await (await Checkout.list(directory, environment)).identify(directory);
    const initial = await key();

    // write ignored build output, then an untracked module, then stage it
    await files.mkdir(join(directory, "target"));
    await files.writeTextFile(join(directory, "target", "output.bin"), "built");
    const ignored = await key();
    await files.writeTextFile(join(directory, "src", "added.ts"), "export {};\n");
    const untracked = await key();
    await git(environment, root, "add", ".");
    const staged = await key();

    // link a module to a file outside the package, then replace the link by a copy
    await files.symlink(join(root, "shared.ts"), join(directory, "src", "shared.ts"));
    const linked = await key();
    await git(environment, root, "add", ".");
    const stagedLink = await key();
    await files.remove(join(directory, "src", "shared.ts"));
    await files.writeTextFile(join(directory, "src", "shared.ts"), "export const shared = 1;\n");
    const copied = await key();

    // ignore output and staging, change the key for an untracked file, key a link by its target
    expect({
        isIgnoredUnchanged: ignored === initial,
        isUntrackedUnchanged: untracked === initial,
        isStagedUnchanged: staged === untracked,
        isLinkUnchanged: linked === untracked,
        isStagedLinkUnchanged: stagedLink === linked,
        isLinkCopied: copied === linked,
    }).toEqual({
        isIgnoredUnchanged: true,
        isUntrackedUnchanged: false,
        isStagedUnchanged: true,
        isLinkUnchanged: false,
        isStagedLinkUnchanged: true,
        isLinkCopied: true,
    });
});

test("key a package by the files a build reads, keeping the key across test and doc edits", async () => {
    // write a package with a module and its test kit in a git checkout
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    const { files } = environment;
    const root = await temporary("destack-checkout-", files);
    await writeManifest(root, "@example/production", files);
    await files.mkdir(join(root, "src", "test"), { recursive: true });
    await files.writeTextFile(join(root, "src", "index.ts"), "export const answer = 42;\n");
    await files.writeTextFile(join(root, "src", "test", "kit.ts"), "export const kit = 1;\n");
    await git(environment, root, "init", "--quiet");
    const key = async () => await (await Checkout.list(root, environment)).identify(root);
    const initial = await key();

    // write a test, a tests folder with a fixture, a readme and a justfile
    await files.mkdir(join(root, "tests", "fixtures"), { recursive: true });
    await files.writeTextFile(join(root, "src", "index.test.ts"), "export {};\n");
    await files.writeTextFile(join(root, "tests", "build.ts"), "export {};\n");
    await files.writeTextFile(join(root, "tests", "fixtures", "note.json"), "{}\n");
    await files.writeTextFile(join(root, "README.md"), "# Production\n");
    await files.writeTextFile(join(root, "justfile"), "test:\n");
    const tested = await key();

    // change the test kit, then the module
    await files.writeTextFile(join(root, "src", "test", "kit.ts"), "export const kit = 2;\n");
    const kit = await key();
    await files.writeTextFile(join(root, "src", "index.ts"), "export const answer = 43;\n");
    const changed = await key();

    // keep the key across files no build reads, and change it for the kit and the module
    expect([tested === initial, kit === tested, changed === kit]).toEqual([true, false, false]);
});

test("list a checkout once and key each package by the git blobs below its directory", async () => {
    // write two packages, one nested in the other, in one checkout, staging the outer module
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    const { files } = environment;
    const root = await temporary("destack-checkout-packages-", files);
    const outer = join(root, "outer");
    const nested = join(outer, "nested");
    await writeManifest(outer, "@example/outer", files);
    await writeManifest(nested, "@example/nested", files);
    await files.writeTextFile(join(outer, "index.ts"), "export {};\n");
    await files.writeTextFile(join(root, "README.md"), "a workspace\n");
    await git(environment, root, "init", "--quiet");
    await git(environment, root, "add", "outer/index.ts");

    // key each package by its files' paths and git blob ids
    const checkout = await Checkout.list(nested, environment);
    const blob = (path: string) => git(environment, root, "hash-object", path);
    expect({
        root: checkout.root,
        outer: await checkout.identify(outer),
        nested: await checkout.identify(nested),
    }).toEqual({
        root,
        outer: await Digest.json(
            [
                ["index.ts", await blob("outer/index.ts")],
                ["nested/package.json", await blob("outer/nested/package.json")],
                ["package.json", await blob("outer/package.json")],
            ],
            environment.crypto,
        ),
        nested: await Digest.json(
            [["package.json", await blob("outer/nested/package.json")]],
            environment.crypto,
        ),
    });
});

test("refuse a package outside a git checkout, ignored by one or listing a linked directory", async () => {
    // write a package outside any checkout, one in an ignored directory and one linking a directory
    await using simulation = testSimulation();
    const environment = simulation.environment("test");
    const { files } = environment;
    const outside = await temporary("destack-checkout-outside-", files);
    await writeManifest(outside, "@example/outside", files);
    const root = await temporary("destack-checkout-refused-", files);
    const ignored = join(root, "ignored");
    const linking = join(root, "linking");
    await writeManifest(ignored, "@example/ignored", files);
    await writeManifest(linking, "@example/linking", files);
    await files.writeTextFile(join(root, ".gitignore"), "ignored/\n");
    await files.symlink(ignored, join(linking, "linked"));
    await git(environment, root, "init", "--quiet");
    const checkout = await Checkout.list(root, environment);

    // refuse each instead of keying no files or failing on a directory read
    expect(await refusal(Checkout.list(outside, environment))).toEqual([
        "INTERNAL_SERVER_ERROR",
        "CHECKOUT_FAILED",
        `git rev-parse failed in ${outside}: fatal: not a git repository (or any of the parent directories): .git`,
    ]);
    expect(await refusal(checkout.identify(ignored))).toEqual([
        "INTERNAL_SERVER_ERROR",
        "CHECKOUT_FAILED",
        `git lists no package.json in ${ignored}`,
    ]);
    expect(await refusal(checkout.identify(linking))).toEqual([
        "INTERNAL_SERVER_ERROR",
        "CHECKOUT_FAILED",
        `git lists the linked directory ${join(linking, "linked")}`,
    ]);
});
