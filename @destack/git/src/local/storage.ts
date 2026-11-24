import type {} from "@destack/package/import-meta";
import { TelemetryScope } from "@destack/telemetry";
import { fileURLToPath, pathToFileURL } from "node:url";
import { basename, dirname, join } from "node:path";
import { type Environment, FileSystemError, Pipe } from "@destack/environment";
import type { Connector, Lease, LeaseMode } from "@destack/resource";
import { found, Uuid, zip } from "@destack/schema";
import { type GitListing, GitReference, type GitStorage } from "../storage/index.ts";
import { Git } from "./git.ts";
import { GitError } from "../error/index.ts";

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

/** The blank line ending the headers a CGI program writes before its body (RFC 3875 6.2). */
const CGI_HEADERS_END = new TextEncoder().encode("\r\n\r\n");

/** The status a CGI response without a Status header answers with (RFC 3875 6.3.3). */
const CGI_DEFAULT_STATUS = 200;

/** The user git http-backend sees on writes, which enables receive-pack (git-http-backend(1)). */
const WRITER = "destack";

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

/** The host capabilities local storage runs git and moves its directories with. */
export type LocalGitEnvironment = Environment<"files" | "process" | "host">;

/** Bare repositories in a host directory, run by the git command line. */
export class LocalGitStorage implements GitStorage {
    /** The provider name repositories stored here record. */
    readonly provider = LOCAL_PROVIDER;
    /** The directory with one bare repository per name. */
    readonly directory: string;
    /** The host's files and processes. */
    readonly environment: LocalGitEnvironment;
    /** The spans and durations of the storage's verbs. */
    readonly #scope: TelemetryScope;

    /** Store repositories in a directory of a host. */
    constructor(directory: string, environment: LocalGitEnvironment) {
        // keep the storage's place and transport, instrumenting on the host's telemetry
        this.directory = directory;
        this.environment = environment;
        this.#scope = TelemetryScope.of(import.meta.destack.package, environment.telemetry);
    }

    /** Open storages at their references below a host directory, as the local provider does. */
    static connector(directory: string, environment: LocalGitEnvironment): Connector<GitStorage> {
        return {
            code: LOCAL_PROVIDER,
            connect: async (binding) =>
                Object.assign(
                    new LocalGitStorage(join(directory, binding.reference), environment),
                    {
                        [Symbol.asyncDispose]: async () => {},
                    },
                ),
        };
    }

    /** Run a storage verb in its span, recording its duration by the provider. */
    #span<Value>(name: string, run: () => Promise<Value>): Promise<Value> {
        const provider = { "destack.git.provider": LOCAL_PROVIDER };

        return this.#scope.span(name, provider, run, { duration: { attributes: provider } });
    }

    /** Create an empty bare repository with HEAD at main, or leave an existing one. */
    create(id: string): Promise<string> {
        return this.#span("destack.git.create", async () => {
            // stage the repository in a directory outside the repository names
            const path = this.#path(id);
            const { files, random } = this.environment;
            const staging = join(this.directory, `${STAGING_PREFIX}${Uuid.v4(random)}`);
            await files.mkdir(staging, { recursive: true });

            // initialize it there and move it into place in one rename, dropping it where another create finished first
            try {
                await Git.run(
                    this.directory,
                    ["init", "--bare", "--quiet", `--initial-branch=${INITIAL_BRANCH}`, staging],
                    this.environment,
                );
                await files.rename(staging, path);
            } catch (error) {
                // drop the staging directory, accepting a repository another create finished
                await files.remove(staging, { recursive: true, force: true });
                if (!FileSystemError.is(error, "FILE_EXISTS")) {
                    throw error;
                }
            }

            return this.#remote(id);
        });
    }

    /** Delete a repository's directory after moving it away in one rename. */
    delete(id: string): Promise<void> {
        return this.#span("destack.git.delete", async () => {
            // move the repository away from its name and skip one already gone
            const { files, random } = this.environment;
            const trash = join(this.directory, `${DELETION_PREFIX}${Uuid.v4(random)}`);
            try {
                await files.rename(this.#path(id), trash);
            } catch (error) {
                if (FileSystemError.is(error, "FILE_NOT_FOUND")) {
                    return;
                }
                throw error;
            }

            // remove it where nothing else opens it
            await files.remove(trash, { recursive: true, force: true });
        });
    }

    /** List the branches, tags and checkouts with HEAD. */
    references(id: string): Promise<GitListing> {
        return this.#span("destack.git.references", async () => {
            // list the branches, tags and checkouts beside HEAD's branch
            const path = this.#path(id);
            const [listed, symbolic] = await Promise.all([
                Git.run(
                    path,
                    [
                        "for-each-ref",
                        "--format=%(refname) %(objectname) %(objecttype) %(*objectname) %(*objecttype)",
                        ...Object.values(GitReference.namespaces),
                    ],
                    this.environment,
                ),
                Git.run(path, ["symbolic-ref", "HEAD"], this.environment),
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
                    : await Git.run(
                          path,
                          ["cat-file", "--batch-check=%(objectname) %(objecttype)"],
                          this.environment,
                          { input: nested.map((name) => `${name}^{commit}\n`).join("") },
                      );

            // map each tag of tags to its final commit or to none
            const answers = peeled.split("\n").filter((line) => line !== "");
            const commits = new Map(
                zip(nested, answers).map(([name, answer]) => {
                    const [object, type] = answer.split(" ");

                    return [
                        name,
                        type === "commit" && object !== undefined ? object : null,
                    ] as const;
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
                defaultReference: references.some((reference) => reference.name === head)
                    ? head
                    : null,
                references,
            };
        });
    }

    /** Lease a repository through its local path, which needs no credential and never lapses. */
    open(id: string, mode: LeaseMode): Promise<Lease> {
        return this.#span("destack.git.open", async () => {
            return { url: this.#remote(id), mode, headers: {} };
        });
    }

    /** Answer a smart HTTP request below a leased repository with git http-backend. */
    serve(lease: Lease, path: string, request: Request): Promise<Response> {
        return this.#span("destack.git.serve", async () => {
            // run git's smart HTTP server on the repository, enabling receive-pack for writes only
            const repository = fileURLToPath(lease.url);
            const { search } = new URL(request.url);
            const header = (name: string) => request.headers.get(name) ?? "";
            const child = this.environment.process.spawn(["git", "http-backend"], {
                variables: {
                    ...Git.variables(this.environment, []),
                    GIT_PROJECT_ROOT: dirname(repository),
                    GIT_HTTP_EXPORT_ALL: "1",
                    PATH_INFO: `/${basename(repository)}${path}`,
                    REQUEST_METHOD: request.method,
                    QUERY_STRING: search.slice(1),
                    CONTENT_TYPE: header("content-type"),
                    HTTP_CONTENT_ENCODING: header("content-encoding"),
                    GIT_PROTOCOL: header("git-protocol"),
                    ...(lease.mode === "write" ? { REMOTE_USER: WRITER } : {}),
                },
                stdin: "pipe",
                stdout: "pipe",
                stderr: "inherit",
            });

            // stream the request body in while the response streams out
            const written =
                request.body === null ? child.stdin.close() : Pipe.to(request.body, child.stdin);
            written.catch(() => child.kill());

            return LocalGitStorage.#respond(child.stdout);
        });
    }

    /** Read a CGI program's output as a response: its headers, then its body as it streams. */
    static async #respond(output: ReadableStream<Uint8Array>): Promise<Response> {
        // read until the blank line ending the headers
        const reader = output.getReader();
        let buffered: Uint8Array<ArrayBuffer> = new Uint8Array(0);
        let end = -1;
        while (end === -1) {
            const { done, value } = await reader.read();
            if (done) {
                throw GitError.of("COMMAND_BROKEN", "git http-backend ended before its headers");
            }
            buffered = LocalGitStorage.#concat(buffered, value);
            end = LocalGitStorage.#find(buffered, CGI_HEADERS_END);
        }

        // read the status and the headers
        const lines = new TextDecoder().decode(buffered.subarray(0, end)).split("\r\n");
        const headers = new Headers();
        let status = CGI_DEFAULT_STATUS;
        for (const line of lines) {
            const separator = line.indexOf(":");
            const name = line.slice(0, separator).trim();
            const value = line.slice(separator + 1).trim();
            if (name.toLowerCase() === "status") {
                status = Number.parseInt(value, 10);
            } else {
                headers.append(name, value);
            }
        }

        // stream the bytes after the headers, then the rest of the output
        const rest = buffered.subarray(end + CGI_HEADERS_END.length);
        const body = new ReadableStream<Uint8Array>({
            start: (controller) => {
                if (rest.length > 0) {
                    controller.enqueue(rest);
                }
            },
            pull: async (controller) => {
                const { done, value } = await reader.read();
                if (done) {
                    controller.close();
                } else {
                    controller.enqueue(value);
                }
            },
            cancel: (reason) => reader.cancel(reason),
        });

        return new Response(body, { status, headers });
    }

    /** Join two byte arrays. */
    static #concat(first: Uint8Array, second: Uint8Array): Uint8Array<ArrayBuffer> {
        // copy both into one new array
        const joined = new Uint8Array(first.length + second.length);
        joined.set(first);
        joined.set(second, first.length);

        return joined;
    }

    /** Find the first index of a byte sequence, or -1. */
    static #find(bytes: Uint8Array, sequence: Uint8Array): number {
        for (let index = 0; index + sequence.length <= bytes.length; index += 1) {
            if (sequence.every((byte, offset) => bytes[index + offset] === byte)) {
                return index;
            }
        }

        return -1;
    }

    /** Build a repository's directory as a file URL. */
    #remote(id: string): string {
        return pathToFileURL(this.#path(id)).href;
    }

    /** Locate a repository's directory and refuse names outside the storage directory. */
    #path(name: string): string {
        if (!NAME.test(name)) {
            throw GitError.of(
                "NAME_INVALID",
                `repository name '${name}' is invalid: it must be letters, digits, '_' and '-', starting with a letter or digit`,
            );
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
            throw new TypeError(
                `git for-each-ref line '${line}' is invalid: it has fewer than five fields`,
            );
        }

        return { name, object, type, tagged, taggedType };
    }
}
