import { mkdir, realpath, stat } from "node:fs/promises";
import { devNull } from "node:os";
import { dirname, isAbsolute, relative, resolve, sep } from "node:path";
import { and, type DatabaseConnection, eq } from "@destack/db";
import { type Identifier, schema } from "@destack/schema";
import { ServiceError } from "@destack/service/error";
import { checkout, type WorkingTreeSource, type WorkingTree } from "../object/checkout.ts";

/** How long a Git command reading a working tree may take, in milliseconds: local reads take tens. */
const GIT_TIMEOUT_MILLISECONDS = 3000;

/** The packages of checkouts, found by their directory below a checkout's root. */
export const Checkout = {
    /** Resolve an absolute directory to its canonical path and the root of its Git working tree. */
    async root(directory: string): Promise<WorkingTree> {
        // require an absolute path before asking Git
        requireAbsolute(directory);
        const canonical = await realpath(directory).catch((error: NodeJS.ErrnoException) => {
            if (error.code !== "ENOENT") {
                throw error;
            }
            throw new ServiceError("NOT_FOUND", { message: `no directory at ${directory}` });
        });

        // ask Git for the working tree's root
        const root = await git(canonical, "rev-parse", "--show-toplevel");

        return { directory: canonical, root: await realpath(root) };
    },

    /** Open a checkout's working tree from its source: the directory as it is, a new repository, or a clone. */
    open(directory: string, source: WorkingTreeSource): Promise<WorkingTree> {
        // register the working tree already there
        if (source.kind === "existing") {
            return Checkout.root(directory);
        }
        // initialize a new repository
        else if (source.kind === "empty") {
            return Checkout.initialize(directory);
        }
        // clone the remote
        else {
            return Checkout.clone(directory, source.remote, source.ref);
        }
    },

    /** Create a directory holding a new Git repository, answering its working tree. */
    async initialize(directory: string): Promise<WorkingTree> {
        // create the directory and the repository in it
        requireAbsolute(directory);
        await mkdir(directory, { recursive: true });
        await git(directory, "init", "--quiet");

        return Checkout.root(directory);
    },

    /** Clone a remote repository into a new directory with the person's own Git configuration and credentials, at a branch, tag or commit when given, answering its working tree. */
    async clone(directory: string, remote: string, reference?: string): Promise<WorkingTree> {
        // clone into the new directory, then check out the reference
        requireAbsolute(directory);
        await asPerson(dirname(directory), "clone", "--quiet", "--", remote, directory);
        if (reference !== undefined) {
            await asPerson(directory, "checkout", "--quiet", reference, "--");
        }

        return Checkout.root(directory);
    },

    /** Resolve the directory a checkout's working tree moved to, requiring its earlier directory gone. */
    async relocated(previous: string, directory: string): Promise<WorkingTree> {
        // require the working tree moved away from its earlier directory
        const isGone = await stat(previous).then(
            () => false,
            (error: NodeJS.ErrnoException) => {
                if (error.code !== "ENOENT") {
                    throw error;
                }

                return true;
            },
        );
        if (!isGone) {
            throw new ServiceError("CONFLICT", {
                message: `the working tree is still at ${previous}; move it before relocating the checkout`,
            });
        }

        return Checkout.root(directory);
    },

    /** Read the commit a working tree is at when clean, untracked files included, and nothing for a dirty one. */
    async commit(root: string): Promise<string | undefined> {
        const status = await git(root, "status", "--porcelain", "--untracked-files=normal");

        return status === "" ? git(root, "rev-parse", "HEAD") : undefined;
    },

    /** Resolve a package directory in a working tree, normalized as installations record it. */
    package(
        root: string,
        directory: string,
    ): { readonly path: string; readonly directory: string } {
        // refuse a directory outside the working tree or on another drive
        const path = resolve(root, directory);
        const below = Checkout.below(root, path);
        if (below === undefined) {
            throw new ServiceError("BAD_REQUEST", {
                message: "package directory is outside the checkout",
            });
        }

        return { path, directory: below };
    },

    /** Normalize an absolute path below a working tree's root as installations record it, absent outside the tree. */
    below(root: string, path: string): string | undefined {
        const below = relative(root, path);
        const isOutside = isAbsolute(below) || below === ".." || below.startsWith(`..${sep}`);

        return isOutside ? undefined : below === "" ? "." : below.split(sep).join("/");
    },

    /** Resolve a package directory in a checkout a machine registers with its working tree's root, refusing another machine's or a missing checkout. */
    async locate(
        database: DatabaseConnection,
        machineId: Identifier<"machine">,
        source: { readonly checkout: string; readonly directory: string },
    ): Promise<{ readonly root: string; readonly path: string; readonly directory: string }> {
        const [registered] = await database
            .select({ root: checkout.table.root })
            .from(checkout.table)
            .where(
                and(
                    eq(checkout.table.scope, machineId),
                    eq(checkout.table.id, schema.identifier("checkout").parse(source.checkout)),
                ),
            );
        if (registered === undefined) {
            throw new ServiceError("NOT_FOUND", {
                message: `this machine registers no checkout ${source.checkout}`,
            });
        }

        return { root: registered.root, ...Checkout.package(registered.root, source.directory) };
    },
};

/** Refuse a checkout directory that is no absolute path. */
function requireAbsolute(directory: string): void {
    if (!isAbsolute(directory)) {
        throw new ServiceError("BAD_REQUEST", { message: "checkout directory must be absolute" });
    }
}

/** Run a Git command that reaches a remote as the person running the machine, with their configuration, credential helpers and keys, refusing with Git's diagnostic. */
async function asPerson(directory: string, ...command: readonly string[]): Promise<void> {
    // run Git in the person's environment, without prompting on the terminal
    const child = Bun.spawn(["git", "-C", directory, ...command], {
        env: { ...process.env, GIT_TERMINAL_PROMPT: "0" },
        stdout: "ignore",
        stderr: "pipe",
    });
    const [code, error] = await Promise.all([child.exited, new Response(child.stderr).text()]);

    // refuse with Git's diagnostic
    if (code !== 0) {
        throw new ServiceError("BAD_REQUEST", {
            message: `git ${command[0]} failed: ${error.trim()}`,
        });
    }
}

/** Run a Git command in a directory without user or system configuration, returning its trimmed output. */
async function git(directory: string, ...command: readonly string[]): Promise<string> {
    // run Git and read its output and diagnostic
    const process = Bun.spawn(["git", "-C", directory, ...command], {
        env: {
            PATH: Bun.env["PATH"],
            SystemRoot: Bun.env["SystemRoot"],
            GIT_CONFIG_NOSYSTEM: "1",
            GIT_CONFIG_GLOBAL: devNull,
        },
        stdout: "pipe",
        stderr: "pipe",
        timeout: GIT_TIMEOUT_MILLISECONDS,
    });
    const [code, output, error] = await Promise.all([
        process.exited,
        new Response(process.stdout).text(),
        new Response(process.stderr).text(),
    ]);

    // refuse with Git's diagnostic
    if (code !== 0) {
        throw new ServiceError("BAD_REQUEST", {
            message: `git ${command[0]} failed: ${error.trim()}`,
        });
    }

    return output.trim();
}
