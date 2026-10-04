import { pathToFileURL } from "node:url";
import { mkdir, mkdtemp, rename, rm } from "node:fs/promises";
import { join } from "node:path";
import type { Lease, LeaseMode } from "@destack/resource";
import { found, zip } from "@destack/schema";
import type { GitListing, GitReference, GitStorage } from "../storage/index.ts";

/** The names a stored repository's directory may take: no separators, no leading dot. */
const NAME = /^[A-Za-z0-9][A-Za-z0-9_-]*$/u;

/** The prefix of the directories for new repositories, with a leading dot apart from names. */
const STAGING_PREFIX = ".create-";

/** The prefix of the directories deleted repositories move to before their removal, apart from names too. */
const DELETION_PREFIX = ".delete-";

/** The branch a new repository's HEAD names. */
const INITIAL_BRANCH = "main";

/** A reference as git for-each-ref lists it: its name, object and type, and a tag's tagged object and type. */
interface ListedReference {
    /** The reference's name. */
    readonly name: string;
    /** The object it names. */
    readonly object: string;
    /** The type of that object. */
    readonly type: string;
    /** The object an annotated tag names. */
    readonly tagged: string;
    /** The type of the tagged object. */
    readonly taggedType: string;
}

/** Bare repositories in a host directory, run by the git command line. */
export class LocalGitStorage implements GitStorage {
    /** The provider name repositories stored here record. */
    readonly provider = "local";
    /** The directory with one bare repository per name. */
    readonly directory: string;

    /** Store repositories in a directory. */
    constructor(directory: string) {
        this.directory = directory;
    }

    /** Build a repository's directory as a file URL. */
    remote(id: string): string {
        return pathToFileURL(this.#path(id)).href;
    }

    /** Create an empty bare repository with HEAD at main, or keep an existing one. */
    async create(id: string): Promise<void> {
        // stage the repository in a directory no repository name takes
        const path = this.#path(id);
        await mkdir(this.directory, { recursive: true });
        const staging = await mkdtemp(join(this.directory, STAGING_PREFIX));

        // initialize it there and move it into place in one rename, dropping it where another create finished first
        try {
            await LocalGitStorage.#git([
                "init",
                "--bare",
                "--quiet",
                `--initial-branch=${INITIAL_BRANCH}`,
                staging,
            ]);
            await rename(staging, path);
        } catch (error) {
            // drop the staging directory, accepting a repository another create finished
            await rm(staging, { recursive: true, force: true });
            const code = systemCode(error);
            if (code !== "ENOTEMPTY" && code !== "EEXIST") {
                throw error;
            }
        }
    }

    /** Delete a repository's directory after moving it away in one rename. */
    async delete(id: string): Promise<void> {
        // move the repository away from its name and skip one already gone
        const trash = join(this.directory, `${DELETION_PREFIX}${crypto.randomUUID()}`);
        try {
            await rename(this.#path(id), trash);
        } catch (error) {
            if (systemCode(error) === "ENOENT") {
                return;
            }
            throw error;
        }

        // remove it where nothing else opens it
        await rm(trash, { recursive: true, force: true });
    }

    /** List the branches and tags with HEAD, peeling annotated tags once and tags of tags in one batch. */
    async references(id: string): Promise<GitListing> {
        // list the branches and tags beside HEAD's branch
        const path = this.#path(id);
        const [listed, symbolic] = await Promise.all([
            LocalGitStorage.#git(
                [
                    "for-each-ref",
                    "--format=%(refname) %(objectname) %(objecttype) %(*objectname) %(*objecttype)",
                    "refs/heads",
                    "refs/tags",
                ],
                path,
            ),
            LocalGitStorage.#git(["symbolic-ref", "HEAD"], path),
        ]);

        // read each reference with its object and tagged object
        const rows = listed
            .split("\n")
            .filter((line) => line !== "")
            .map((line) => LocalGitStorage.#row(line));

        // peel tags of tags in one batch
        const nested = rows.filter((row) => row.taggedType === "tag").map((row) => row.name);
        const peeled =
            nested.length === 0
                ? ""
                : await LocalGitStorage.#git(
                      ["cat-file", "--batch-check=%(objectname) %(objecttype)"],
                      path,
                      nested.map((name) => `${name}^{commit}\n`).join(""),
                  );

        // map each tag of tags to its final commit or to none
        const answers = peeled.split("\n").filter((line) => line !== "");
        const commits = new Map(
            zip(nested, answers).map(([name, answer]) => {
                const [object, type] = answer.split(" ");

                return [name, type === "commit" && object !== undefined ? object : null] as const;
            }),
        );

        // resolve each reference to its commit
        const references: GitReference[] = rows.map((row) => ({
            name: row.name,
            object: row.object,
            commit: LocalGitStorage.#commit(row, commits),
        }));
        const head = symbolic.trim();

        return {
            defaultReference: references.some((reference) => reference.name === head) ? head : null,
            references,
        };
    }

    /** Lease a repository through its local path, which needs no credential and never lapses. */
    async open(id: string, mode: LeaseMode): Promise<Lease> {
        return { url: this.remote(id), mode, headers: {} };
    }

    /** Locate a repository's directory and refuse names outside the storage directory. */
    #path(name: string): string {
        if (!NAME.test(name)) {
            throw new TypeError(`invalid local repository name: ${name}`);
        }

        return join(this.directory, `${name}.git`);
    }

    /** Resolve the commit a reference names: itself, an annotated tag's tagged commit, a tag of tags' peeled commit, or none. */
    static #commit(
        row: ListedReference,
        peeled: ReadonlyMap<string, string | null>,
    ): string | null {
        if (row.type === "commit") {
            return row.object;
        } else if (row.taggedType === "commit") {
            return row.tagged;
        } else if (row.taggedType === "tag") {
            return found(peeled, row.name);
        }

        return null;
    }

    /** Read a for-each-ref line: the name, object and type, and a tag's tagged object and type. */
    static #row(line: string): ListedReference {
        const [name, object, type, tagged, taggedType] = line.split(" ");
        if (
            name === undefined ||
            object === undefined ||
            type === undefined ||
            tagged === undefined ||
            taggedType === undefined
        ) {
            throw new TypeError(`git for-each-ref printed ${line}`);
        }

        return { name, object, type, tagged, taggedType };
    }

    /** Run git in a repository, returning its output and failing loudly on a nonzero exit. */
    static async #git(
        arguments_: readonly string[],
        directory?: string,
        input = "",
    ): Promise<string> {
        // run git in the repository without reading global configuration
        const child = Bun.spawn(
            ["git", ...(directory === undefined ? [] : ["-C", directory]), ...arguments_],
            {
                env: { ...process.env, GIT_CONFIG_NOSYSTEM: "1", GIT_TERMINAL_PROMPT: "0" },
                stdin: new TextEncoder().encode(input),
                stdout: "pipe",
                stderr: "pipe",
            },
        );
        const [code, output, errors] = await Promise.all([
            child.exited,
            new Response(child.stdout).text(),
            new Response(child.stderr).text(),
        ]);

        // fail with git's message
        if (code !== 0) {
            throw new Error(`git ${arguments_[0]} failed with ${code}: ${errors.trim()}`);
        }

        return output;
    }
}

/** Read the system error code of a failure, absent for failures without one. */
function systemCode(error: unknown): unknown {
    return error instanceof Error && "code" in error ? error.code : undefined;
}
