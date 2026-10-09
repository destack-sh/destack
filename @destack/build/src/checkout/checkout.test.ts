import { mkdir, mkdtemp, realpath, rename, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { ServiceError } from "@destack/service/error";
import { expect, onTestFinished, test } from "@destack/test";
import { Checkout } from "./checkout.ts";

/** The root of the fixture's working tree. */
const ROOT = "/work/repository";

/** Resolve a package directory, or catch its refusal. */
function locate(directory: string): unknown {
    try {
        return Checkout.package(ROOT, directory);
    } catch (error) {
        return error;
    }
}

test("normalize package directories inside the working tree and refuse those outside it", () => {
    const outside = new ServiceError("BAD_REQUEST", {
        message: "package directory is outside the checkout",
    });

    // record the path below the root with forward slashes and refuse escapes
    expect([
        locate("./pkg/"),
        locate("."),
        locate("packages/../pkg"),
        locate("../x"),
        locate("/elsewhere/pkg"),
        locate(join(ROOT, "pkg")),
    ]).toEqual([
        { path: join(ROOT, "pkg"), directory: "pkg" },
        { path: ROOT, directory: "." },
        { path: join(ROOT, "pkg"), directory: "pkg" },
        outside,
        outside,
        { path: join(ROOT, "pkg"), directory: "pkg" },
    ]);
});

/** Run Git in a fixture working tree as a fixed author, returning its trimmed output. */
async function git(directory: string, ...command: readonly string[]): Promise<string> {
    const process = Bun.spawn(
        [
            "git",
            "-C",
            directory,
            "-c",
            "user.name=Fixture",
            "-c",
            "user.email=fixture@destack.test",
            ...command,
        ],
        { stdout: "pipe", stderr: "pipe" },
    );
    const [code, output, error] = await Promise.all([
        process.exited,
        new Response(process.stdout).text(),
        new Response(process.stderr).text(),
    ]);
    expect(code, error).toBe(0);

    return output.trim();
}

test("read a working tree's root, and its commit while clean but not with changed or untracked files", async () => {
    // commit one file into a new working tree
    const repository = await realpath(await mkdtemp(join(tmpdir(), "destack-checkout-")));
    onTestFinished(() => rm(repository, { recursive: true }));
    await git(repository, "init", "--quiet", "--initial-branch=main");
    await writeFile(join(repository, "README.md"), "# Site\n");
    await git(repository, "add", "README.md");
    await git(repository, "commit", "--quiet", "--message=Add README");
    const head = await git(repository, "rev-parse", "HEAD");
    const clean = await Checkout.commit(repository);

    // change the committed file and add an untracked one
    await writeFile(join(repository, "README.md"), "# Changed\n");
    const changed = await Checkout.commit(repository);
    await git(repository, "checkout", "--quiet", "README.md");
    await writeFile(join(repository, "new.ts"), "export {};\n");
    const untracked = await Checkout.commit(repository);

    // resolve the root from a directory inside and refuse a relative one
    const root = await Checkout.root(repository);
    const relative = await Checkout.root("repository").catch((error: unknown) => error);
    expect([clean, changed, untracked, root, relative]).toEqual([
        head,
        undefined,
        undefined,
        { directory: repository, root: repository },
        new ServiceError("BAD_REQUEST", { message: "checkout directory must be absolute" }),
    ]);
});

test("initialize a new repository, clone a remote at a tag, and relocate a working tree once it moved, refusing one still in place", async () => {
    // commit one file into a remote and tag it, then commit again
    const scratch = await realpath(await mkdtemp(join(tmpdir(), "destack-checkout-")));
    onTestFinished(() => rm(scratch, { recursive: true }));
    const remote = join(scratch, "remote");
    await mkdir(remote);
    await git(remote, "init", "--quiet", "--initial-branch=main");
    await writeFile(join(remote, "README.md"), "# First\n");
    await git(remote, "add", "README.md");
    await git(remote, "commit", "--quiet", "--message=First");
    await git(remote, "tag", "first");
    const tagged = await git(remote, "rev-parse", "HEAD");
    await writeFile(join(remote, "README.md"), "# Second\n");
    await git(remote, "commit", "--quiet", "--all", "--message=Second");

    // initialize a new directory and clone the remote at its tag
    const initialized = await Checkout.initialize(join(scratch, "new", "notes"));
    const cloned = await Checkout.clone(join(scratch, "clone"), remote, "first");
    const commit = await git(cloned.root, "rev-parse", "HEAD");

    // refuse relocating a working tree still in place, then relocate it once moved
    const still = await Checkout.relocated(cloned.root, cloned.root).catch(
        (error: unknown) => error,
    );
    await rename(cloned.root, join(scratch, "moved"));
    const moved = await Checkout.relocated(cloned.root, join(scratch, "moved"));
    expect([initialized, commit, still, moved]).toEqual([
        { directory: join(scratch, "new", "notes"), root: join(scratch, "new", "notes") },
        tagged,
        new ServiceError("CONFLICT", {
            message: `the working tree is still at ${cloned.root}; move it before relocating the checkout`,
        }),
        { directory: join(scratch, "moved"), root: join(scratch, "moved") },
    ]);
});
