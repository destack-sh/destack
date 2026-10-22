import { execFile } from "node:child_process";
import { createHash } from "node:crypto";
import { readFile } from "node:fs/promises";
import { join } from "node:path";
import { promisify } from "node:util";
import { Digest, schema } from "@destack/schema";
import { BuildError, isMissing } from "../error/index.ts";
import { relativePath } from "../source/dependency.ts";

/** Run a command and read its standard output. */
const run = promisify(execFile);

/** The most bytes a checkout's listing may print, 64 MiB: a million paths of 64 bytes. */
const LISTING_BYTES = 64 * 1024 * 1024;

/** The mode git stores a symbolic link with, whose blob holds the link's target path. */
const LINK_MODE = "120000";

/** The hash functions of git's object formats. */
const ObjectFormat = schema.enum(["sha1", "sha256"]);

/** The files of a git checkout: its tracked files and the untracked files git does not ignore. */
export class Checkout {
    /** The checkout's top-level directory. */
    readonly root: string;
    /** The git blob id of each listed file by path, absent when git stores no current one. */
    readonly files: ReadonlyMap<string, string | undefined>;
    /** The hash function of the checkout's object format. */
    readonly #format: schema.Infer<typeof ObjectFormat>;

    /** Keep a checkout's root, its listed files and its object format. */
    private constructor(
        root: string,
        files: ReadonlyMap<string, string | undefined>,
        format: schema.Infer<typeof ObjectFormat>,
    ) {
        this.root = root;
        this.files = files;
        this.#format = format;
    }

    /** List the files of the git checkout enclosing a directory, refusing one outside. */
    static async list(directory: string): Promise<Checkout> {
        // find the checkout's top-level directory and object format
        const revision = ["rev-parse", "--show-toplevel", "--show-object-format"];
        const [root, format] = (await Checkout.#git(revision, directory)).split("\n");
        if (root === undefined || root === "") {
            throw new BuildError("BUILD_FAILED", `git reports no checkout of ${directory}`);
        }

        // list the stored blobs, then the untracked and modified files git stores no blob for
        const [stored, changed] = await Promise.all([
            Checkout.#git(["ls-files", "--stage", "-z"], root),
            Checkout.#git(["ls-files", "--others", "--modified", "--exclude-standard", "-z"], root),
        ]);
        const files = new Map<string, string | undefined>();
        for (const entry of Checkout.#entries(stored)) {
            // keep a regular file's blob, reading a link through to its target
            const [mode, blob] = entry.slice(0, entry.indexOf("\t")).split(" ");
            files.set(entry.slice(entry.indexOf("\t") + 1), mode === LINK_MODE ? undefined : blob);
        }
        for (const path of Checkout.#entries(changed)) {
            files.set(path, undefined);
        }

        return new Checkout(root, files, ObjectFormat.parse(format));
    }

    /** Digest a package's files by path and git blob id, hashing files without a blob. */
    async identify(directory: string): Promise<Digest> {
        // take the files below the directory
        const below = relativePath(this.root, directory);
        const prefix = below === "" ? "" : `${below}/`;
        const files = [...this.files].filter(([path]) => path.startsWith(prefix));

        // refuse a package outside the checkout or ignored by it, whose listing lacks its manifest
        if (!files.some(([path]) => path === `${prefix}package.json`)) {
            throw new BuildError("BUILD_FAILED", `git lists no package.json in ${directory}`);
        }

        // key each file by its stored blob, else by the blob of its bytes, in path order
        const blobs = await Promise.all(
            files
                .toSorted(([left], [right]) => (left < right ? -1 : 1))
                .map(async ([path, blob]) => [
                    path.slice(prefix.length),
                    blob ?? (await this.#hash(join(this.root, path))),
                ]),
        );

        return await Digest.json(blobs);
    }

    /** Hash a file's bytes through its links as git hashes a blob, null for a deleted file. */
    async #hash(path: string): Promise<string | null> {
        try {
            const bytes = await readFile(path);

            return createHash(this.#format)
                .update(`blob ${bytes.byteLength}\0`)
                .update(bytes)
                .digest("hex");
        } catch (error) {
            // key a deleted file as null
            if (isMissing(error)) {
                return null;
            }
            // refuse a linked directory, which git lists as one entry
            else if (error instanceof Error && "code" in error && error.code === "EISDIR") {
                throw new BuildError("BUILD_FAILED", `git lists the linked directory ${path}`);
            }
            throw error;
        }
    }

    /** Split git's NUL-separated output into its entries. */
    static #entries(output: string): string[] {
        return output.split("\0").filter((entry) => entry !== "");
    }

    /** Run git in a directory and read its standard output, refusing a failed run. */
    static async #git(command: readonly string[], directory: string): Promise<string> {
        const ran = await run("git", command, { cwd: directory, maxBuffer: LISTING_BYTES }).catch(
            (cause: unknown) => {
                throw new BuildError("BUILD_FAILED", `git ${command[0]} failed in ${directory}`, {
                    cause,
                });
            },
        );

        return ran.stdout;
    }
}
