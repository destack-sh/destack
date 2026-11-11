import { join } from "node:path";
import { testSimulation } from "@destack/object/test";
import { expect, onTestFinished, test } from "@destack/test";
import { MachineError } from "../../../error/index.ts";
import { WorkingTree } from "../object/index.ts";
import { Git } from "./git.ts";

/** The root of the fixture's working tree. */
const ROOT = "/work/repository";

/** The host the fixture's working trees live on, a world's node over the host's files and Git. */
const host = testSimulation().environment("git");

/** The host's files. */
const { files } = host;

/** Resolve a package directory, or catch its refusal. */
function locate(directory: string): unknown {
    try {
        return WorkingTree.package(ROOT, directory);
    } catch (error) {
        return error;
    }
}

test("normalize package directories inside the working tree and refuse those outside it", () => {
    const outside = MachineError.of(
        "OUTSIDE_CHECKOUT_INVALID",
        "package directory is invalid: it is outside the checkout",
    );

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
    const child = host.process.spawn(
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
        child.exited,
        new Response(child.stdout).text(),
        new Response(child.stderr).text(),
    ]);
    expect(code, error).toBe(0);

    return output.trim();
}

test("read a working tree's root, and its commit while clean but not with changed or untracked files", async () => {
    // commit one file into a new working tree
    const repository = await files.realPath(
        await files.makeTemporaryDirectory("destack-checkout-"),
    );
    onTestFinished(() => files.remove(repository, { recursive: true }));
    await git(repository, "init", "--quiet", "--initial-branch=main");
    await files.writeTextFile(join(repository, "README.md"), "# Site\n");
    await git(repository, "add", "README.md");
    await git(repository, "commit", "--quiet", "--message=Add README");
    const head = await git(repository, "rev-parse", "HEAD");
    const clean = await new Git(host).commit(repository);

    // change the committed file and add an untracked one
    await files.writeTextFile(join(repository, "README.md"), "# Changed\n");
    const changed = await new Git(host).commit(repository);
    await git(repository, "checkout", "--quiet", "README.md");
    await files.writeTextFile(join(repository, "new.ts"), "export {};\n");
    const untracked = await new Git(host).commit(repository);

    // resolve the root from a directory inside and refuse a relative one
    const root = await new Git(host).root(repository);
    const relative = await new Git(host).root("repository").catch((error: unknown) => error);
    expect([clean, changed, untracked, root, relative]).toEqual([
        head,
        undefined,
        undefined,
        { directory: repository, root: repository },
        MachineError.of(
            "RELATIVE_DIRECTORY_INVALID",
            "checkout directory is invalid: it must be absolute",
        ),
    ]);
});

test("initialize a new repository, clone a remote at a tag, and relocate a working tree once it moved, refusing one still in place", async () => {
    // commit one file into a remote and tag it, then commit again
    const scratch = await files.realPath(await files.makeTemporaryDirectory("destack-checkout-"));
    onTestFinished(() => files.remove(scratch, { recursive: true }));
    const remote = join(scratch, "remote");
    await files.mkdir(remote);
    await git(remote, "init", "--quiet", "--initial-branch=main");
    await files.writeTextFile(join(remote, "README.md"), "# First\n");
    await git(remote, "add", "README.md");
    await git(remote, "commit", "--quiet", "--message=First");
    await git(remote, "tag", "first");
    const tagged = await git(remote, "rev-parse", "HEAD");
    await files.writeTextFile(join(remote, "README.md"), "# Second\n");
    await git(remote, "commit", "--quiet", "--all", "--message=Second");

    // initialize a new directory and clone the remote at its tag
    const initialized = await new Git(host).initialize(join(scratch, "new", "notes"));
    const cloned = await new Git(host).clone(join(scratch, "clone"), remote, "first");
    const commit = await git(cloned.root, "rev-parse", "HEAD");

    // refuse relocating a working tree still in place, then relocate it once moved
    const still = await new Git(host)
        .relocated(cloned.root, cloned.root)
        .catch((error: unknown) => error);
    await files.rename(cloned.root, join(scratch, "moved"));
    const moved = await new Git(host).relocated(cloned.root, join(scratch, "moved"));
    expect([initialized, commit, still, moved]).toEqual([
        { directory: join(scratch, "new", "notes"), root: join(scratch, "new", "notes") },
        tagged,
        MachineError.of(
            "TREE_BLOCKED",
            `cannot relocate the checkout: the working tree is still at '${cloned.root}', so move it first`,
        ),
        { directory: join(scratch, "moved"), root: join(scratch, "moved") },
    ]);
});
