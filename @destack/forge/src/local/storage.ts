import { fileURLToPath, pathToFileURL } from "node:url";
import { mkdir, mkdtemp, rename, rm } from "node:fs/promises";
import { join } from "node:path";
import type { Connector, Lease, LeaseMode } from "@destack/resource";
import { found, zip } from "@destack/schema";
import type {
    GitListing,
    GitReference,
    GitStorage,
    GitStorageDeclaration,
} from "../storage/index.ts";
import { Git } from "./git.ts";

/** The provider and connector code of local Git storage. */
export const LOCAL_PROVIDER = "local";

/** The names a stored repository's directory may take: no separators, no leading dot. */
const NAME = /^[A-Za-z0-9][A-Za-z0-9_-]*$/u;

/** The prefix of the directories for new repositories, with a leading dot no repository takes. */
const STAGING_PREFIX = ".create-";

/** The prefix of the directories deleted repositories move to before their removal. */
const DELETION_PREFIX = ".delete-";

/** The branch of a new repository's HEAD. */
const INITIAL_BRANCH = "main";

/** A reference as git for-each-ref lists it. */
interface ListedReference {
    /** The reference's name. */
    readonly name: string;
    /** The object it points at. */
    readonly object: string;
    /** The type of that object. */
    readonly type: string;
    /** The object an annotated tag points at. */
    readonly tagged: string;
    /** The type of the tagged object. */
    readonly taggedType: string;
}

/** Bare repositories in a host directory, run by the git command line. */
export class LocalGitStorage implements GitStorage {
    /** The connector opening a storage at its directory's file URL. */
    static readonly connector: Connector<GitStorage, GitStorageDeclaration> = {
        code: LOCAL_PROVIDER,
        connect: async (binding) =>
            Object.assign(new LocalGitStorage(fileURLToPath(binding.reference)), {
                [Symbol.asyncDispose]: async () => {},
            }),
    };

    /** The provider name repositories stored here record. */
    readonly provider = LOCAL_PROVIDER;
    /** The directory with one bare repository per name. */
    readonly directory: string;

    /** Store repositories in a directory. */
    constructor(directory: string) {
        this.directory = directory;
    }

    /** Create an empty bare repository with HEAD at main, or leave an existing one. */
    async create(id: string): Promise<string> {
        // stage the repository in a directory outside the repository names
        const path = this.#path(id);
        await mkdir(this.directory, { recursive: true });
        const staging = await mkdtemp(join(this.directory, STAGING_PREFIX));

        // initialize it there and move it into place in one rename, dropping it where another create finished first
        try {
            await Git.run(this.directory, [
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
            const code = LocalGitStorage.#systemCode(error);
            if (code !== "ENOTEMPTY" && code !== "EEXIST") {
                throw error;
            }
        }

        return this.#remote(id);
    }

    /** Delete a repository's directory after moving it away in one rename. */
    async delete(id: string): Promise<void> {
        // move the repository away from its name and skip one already gone
        const trash = join(this.directory, `${DELETION_PREFIX}${crypto.randomUUID()}`);
        try {
            await rename(this.#path(id), trash);
        } catch (error) {
            if (LocalGitStorage.#systemCode(error) === "ENOENT") {
                return;
            }
            throw error;
        }

        // remove it where nothing else opens it
        await rm(trash, { recursive: true, force: true });
    }

    /** List the branches and tags with HEAD. */
    async references(id: string): Promise<GitListing> {
        // list the branches and tags beside HEAD's branch
        const path = this.#path(id);
        const [listed, symbolic] = await Promise.all([
            Git.run(path, [
                "for-each-ref",
                "--format=%(refname) %(objectname) %(objecttype) %(*objectname) %(*objecttype)",
                "refs/heads",
                "refs/tags",
            ]),
            Git.run(path, ["symbolic-ref", "HEAD"]),
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
                : await Git.run(path, ["cat-file", "--batch-check=%(objectname) %(objecttype)"], {
                      input: nested.map((name) => `${name}^{commit}\n`).join(""),
                  });

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
        return { url: this.#remote(id), mode, headers: {} };
    }

    /** Build a repository's directory as a file URL. */
    #remote(id: string): string {
        return pathToFileURL(this.#path(id)).href;
    }

    /** Locate a repository's directory and refuse names outside the storage directory. */
    #path(name: string): string {
        if (!NAME.test(name)) {
            throw new TypeError(`invalid local repository name: ${name}`);
        }

        return join(this.directory, `${name}.git`);
    }

    /** Resolve the commit of a reference. */
    static #commit(
        row: ListedReference,
        peeled: ReadonlyMap<string, string | null>,
    ): string | null {
        // read a branch or lightweight tag's commit
        if (row.type === "commit") {
            return row.object;
        }
        // read an annotated tag's commit
        else if (row.taggedType === "commit") {
            return row.tagged;
        }
        // read a tag of a tag's peeled commit
        else if (row.taggedType === "tag") {
            return found(peeled, row.name);
        }
        // resolve tags of trees and blobs to no commit
        else {
            return null;
        }
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

    /** Read the system error code of a failure, absent for failures without one. */
    static #systemCode(error: unknown): unknown {
        return error instanceof Error && "code" in error ? error.code : undefined;
    }
}
