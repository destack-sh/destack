import { mkdir, mkdtemp, realpath, rm, symlink, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { Digest } from "@destack/schema";
import { expect, onTestFinished, test } from "@destack/test";
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
    await expect(Checkout.list(outside)).rejects.toMatchObject({
        code: "BUILD_FAILED",
        message: `git rev-parse failed in ${outside}`,
    });
    await expect(checkout.identify(ignored)).rejects.toMatchObject({
        code: "BUILD_FAILED",
        message: `git lists no package.json in ${ignored}`,
    });
    await expect(checkout.identify(linking)).rejects.toMatchObject({
        code: "BUILD_FAILED",
        message: `git lists the linked directory ${join(linking, "linked")}`,
    });
});
