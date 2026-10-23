import { mkdir, mkdtemp, realpath, rm, symlink, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { Digest } from "@destack/schema";
import { expect, onTestFinished, refusal, test } from "@destack/test";
import { Checkout } from "./checkout.ts";

/** Make a temporary directory, removed after the test. */
async function temporary(prefix: string): Promise<string> {
    const directory = await realpath(await mkdtemp(join(tmpdir(), prefix)));
    onTestFinished(() => rm(directory, { recursive: true }));

    return directory;
}

/** Write a package's manifest into a directory. */
async function writeManifest(directory: string, name: string): Promise<void> {
    await mkdir(directory, { recursive: true });
    await writeFile(join(directory, "package.json"), JSON.stringify({ name }));
}

/** Run git in a directory, failing the test on a failed run. */
function git(directory: string, ...command: string[]): string {
    const ran = Bun.spawnSync(["git", "-C", directory, ...command]);
    expect(ran.exitCode).toBe(0);

    return ran.stdout.toString().trim();
}

test("key a package by the files git lists, reading linked files through their links", async () => {
    // write a package in a git checkout that ignores its build output
    const root = await temporary("destack-checkout-");
    const directory = join(root, "package");
    await writeManifest(directory, "@example/checkout");
    await mkdir(join(directory, "src"));
    await writeFile(join(root, "shared.ts"), "export const shared = 1;\n");
    await writeFile(join(directory, ".gitignore"), "target/\n");
    await writeFile(join(directory, "src", "index.ts"), "export {};\n");
    git(root, "init", "--quiet");
    const key = async () => await (await Checkout.list(directory)).identify(directory);
    const initial = await key();

    // write ignored build output, then an untracked module, then stage it
    await mkdir(join(directory, "target"));
    await writeFile(join(directory, "target", "output.bin"), "built");
    const ignored = await key();
    await writeFile(join(directory, "src", "added.ts"), "export {};\n");
    const untracked = await key();
    git(root, "add", ".");
    const staged = await key();

    // link a module to a file outside the package, then replace the link by a copy
    await symlink(join(root, "shared.ts"), join(directory, "src", "shared.ts"));
    const linked = await key();
    git(root, "add", ".");
    const stagedLink = await key();
    await rm(join(directory, "src", "shared.ts"));
    await writeFile(join(directory, "src", "shared.ts"), "export const shared = 1;\n");
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
    const root = await temporary("destack-checkout-");
    await writeManifest(root, "@example/production");
    await mkdir(join(root, "src", "test"), { recursive: true });
    await writeFile(join(root, "src", "index.ts"), "export const answer = 42;\n");
    await writeFile(join(root, "src", "test", "kit.ts"), "export const kit = 1;\n");
    git(root, "init", "--quiet");
    const key = async () => await (await Checkout.list(root)).identify(root);
    const initial = await key();

    // write a test, a tests folder with a fixture, a readme and a justfile
    await mkdir(join(root, "tests", "fixtures"), { recursive: true });
    await writeFile(join(root, "src", "index.test.ts"), "export {};\n");
    await writeFile(join(root, "tests", "build.ts"), "export {};\n");
    await writeFile(join(root, "tests", "fixtures", "note.json"), "{}\n");
    await writeFile(join(root, "README.md"), "# Production\n");
    await writeFile(join(root, "justfile"), "test:\n");
    const tested = await key();

    // change the test kit, then the module
    await writeFile(join(root, "src", "test", "kit.ts"), "export const kit = 2;\n");
    const kit = await key();
    await writeFile(join(root, "src", "index.ts"), "export const answer = 43;\n");
    const changed = await key();

    // keep the key across files no build reads, and change it for the kit and the module
    expect([tested === initial, kit === tested, changed === kit]).toEqual([true, false, false]);
});

test("list a checkout once and key each package by the git blobs below its directory", async () => {
    // write two packages, one nested in the other, in one checkout, staging the outer module
    const root = await temporary("destack-checkout-packages-");
    const outer = join(root, "outer");
    const nested = join(outer, "nested");
    await writeManifest(outer, "@example/outer");
    await writeManifest(nested, "@example/nested");
    await writeFile(join(outer, "index.ts"), "export {};\n");
    await writeFile(join(root, "README.md"), "a workspace\n");
    git(root, "init", "--quiet");
    git(root, "add", "outer/index.ts");

    // key each package by its files' paths and git blob ids
    const checkout = await Checkout.list(nested);
    const blob = (path: string) => git(root, "hash-object", path);
    expect({
        root: checkout.root,
        outer: await checkout.identify(outer),
        nested: await checkout.identify(nested),
    }).toEqual({
        root,
        outer: await Digest.json([
            ["index.ts", blob("outer/index.ts")],
            ["nested/package.json", blob("outer/nested/package.json")],
            ["package.json", blob("outer/package.json")],
        ]),
        nested: await Digest.json([["package.json", blob("outer/nested/package.json")]]),
    });
});

test("refuse a package outside a git checkout, ignored by one or listing a linked directory", async () => {
    // write a package outside any checkout, one in an ignored directory and one linking a directory
    const outside = await temporary("destack-checkout-outside-");
    await writeManifest(outside, "@example/outside");
    const root = await temporary("destack-checkout-refused-");
    const ignored = join(root, "ignored");
    const linking = join(root, "linking");
    await writeManifest(ignored, "@example/ignored");
    await writeManifest(linking, "@example/linking");
    await writeFile(join(root, ".gitignore"), "ignored/\n");
    await symlink(ignored, join(linking, "linked"));
    git(root, "init", "--quiet");
    const checkout = await Checkout.list(root);

    // refuse each instead of keying no files or failing on a directory read
    expect(await refusal(Checkout.list(outside))).toEqual([
        "INTERNAL_SERVER_ERROR",
        "CHECKOUT_FAILED",
        `git rev-parse failed in ${outside}`,
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
