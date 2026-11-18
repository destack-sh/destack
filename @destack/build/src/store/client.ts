import { Tarball, type TarballEntry } from "@destack/package/archive";
import { Digest, found, nth, present, schema } from "@destack/schema";
import { CallError } from "@destack/object";
import type { PackageFile } from "@destack/package/file";
import { BuildError } from "../error/index.ts";
import { BuildObject, type BuildStore, MAX_WRITE_BYTES } from "./store.ts";

/** A build store over HTTP: has, read and write below a scope, as registry clients push images. */
export class PackageClient {
    /** The URL the store is reached at below its scope, ending in a slash. */
    readonly url: string;
    /** Send a request to the server. */
    readonly #fetch: (request: Request) => Promise<Response>;

    /** Call a build store at a URL through a fetch. */
    constructor(url: string, fetch: (request: Request) => Promise<Response>) {
        this.url = url.endsWith("/") ? url : `${url}/`;
        this.#fetch = fetch;
    }

    /** Tell which of some blobs the store has, one flag per digest. */
    async has(digests: readonly Digest[]): Promise<boolean[]> {
        // ask nothing of none
        if (digests.length === 0) {
            return [];
        }
        const response = await this.#send(
            new Request(`${this.url}has`, {
                method: "POST",
                headers: { "content-type": "application/json" },
                body: JSON.stringify(digests),
            }),
        );

        return schema.array(schema.boolean()).parse(await response.json());
    }

    /** Read objects of the lock or build the URL is below as tarball entries, in order. */
    async *read(
        objects: readonly Pick<BuildObject, "kind" | "digest">[],
    ): AsyncIterable<TarballEntry> {
        // read nothing of none
        if (objects.length === 0) {
            return;
        }
        const response = await this.#send(
            new Request(`${this.url}read`, {
                method: "POST",
                headers: { "content-type": "application/json" },
                body: JSON.stringify(objects.map(({ kind, digest }) => ({ kind, digest }))),
            }),
        );
        yield* Tarball.read(present(response.body, "the body of a read"));
    }

    /** Write objects, files first, as one tarball. */
    async write(entries: AsyncIterable<TarballEntry>, signal?: AbortSignal): Promise<void> {
        const response = await this.#send(
            new Request(`${this.url}write`, {
                method: "POST",
                headers: { "content-type": "application/x-tar" },
                body: Tarball.stream(entries),
                signal: signal ?? null,
            }),
        );
        await response.body?.cancel();
    }

    /** Push locks' objects and their builds' attached files, writing what the server lacks. */
    async push(
        store: Pick<
            BuildStore,
            "closure" | "contents" | "head" | "lock" | "read" | "attachments" | "environment"
        >,
        locks: readonly Digest[],
        signal?: AbortSignal,
    ): Promise<void> {
        // list every lock's objects, files first, and the files each build attached
        const roots = locks.map((digest) => ({ kind: "lock", digest }) as const);
        const objects = await BuildObject.of(store, roots, "all");
        const manifests = objects.filter((object) => object.kind === "manifest");
        const attachments = new Map(
            await Promise.all(
                manifests.map(
                    async ({ digest }) => [digest, await store.attachments(digest)] as const,
                ),
            ),
        );
        const attached = [...attachments.values()].flat().map((file) => ({
            kind: "file" as const,
            digest: file.digest,
            size: file.size,
        }));

        // write what the server lacks, each build's attachments in the write that records the build
        await this.#upload(store, [...attached, ...objects], attachments, signal);
    }

    /** Write the objects the server lacks from a store, in uploads of at most the write limit. */
    async #upload(
        store: Pick<BuildStore, "read" | "environment">,
        objects: readonly BuildObject[],
        attachments: ReadonlyMap<Digest, readonly PackageFile[]>,
        signal: AbortSignal | undefined,
    ): Promise<void> {
        // ask which objects the server has
        const stored = await this.has(objects.map((object) => object.digest));
        const lacking = objects.filter((_object, index) => stored[index] !== true);

        // write them in order, in uploads of at most the write limit
        const batches: BuildObject[][] = [[]];
        let size = 0;
        for (const object of lacking) {
            if (size > 0 && size + object.size > MAX_WRITE_BYTES) {
                batches.push([]);
                size = 0;
            }
            nth(batches, batches.length - 1).push(object);
            size += object.size;
        }
        for (const batch of batches.filter((each) => each.length > 0)) {
            // send each build's attachments after the objects that record it
            signal?.throwIfAborted();
            const carried = batch.flatMap((object) =>
                object.kind === "manifest"
                    ? found(attachments, object.digest).map((file) => ({
                          subject: object.digest,
                          file,
                      }))
                    : [],
            );
            const entries = concatenated(
                store.read(batch),
                BuildObject.attachments(carried, store.environment.crypto),
            );
            await this.write(entries, signal);
        }
    }

    /** Send a request, refusing a failed one with the server's call error, else its answer. */
    async #send(request: Request): Promise<Response> {
        const response = await this.#fetch(request);
        if (!response.ok) {
            // rethrow the server's call error as itself, as a consent the caller can answer
            const answer = await response.text();
            const called = CallError.fromJson(jsonOf(answer));
            if (called !== undefined) {
                throw called;
            }

            // refuse any other failure with the answer
            throw BuildError.of(
                "UPLOAD_UNREACHABLE",
                `could not call '${new URL(request.url).pathname}': HTTP ${response.status} '${answer}'`,
            );
        }

        return response;
    }
}

/** Read text as JSON, absent for text of another format. */
function jsonOf(text: string): unknown {
    try {
        return JSON.parse(text);
    } catch (error) {
        if (error instanceof SyntaxError) {
            return undefined;
        }
        throw error;
    }
}

/** Yield one stream's entries, then another's. */
async function* concatenated(
    first: AsyncIterable<TarballEntry>,
    second: AsyncIterable<TarballEntry>,
): AsyncIterable<TarballEntry> {
    yield* first;
    yield* second;
}
