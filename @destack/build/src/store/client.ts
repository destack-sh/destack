import { Tarball, type TarballEntry } from "@destack/package/archive";
import { Digest, nth, present, schema } from "@destack/schema";
import { CallError } from "@destack/object";
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

    /** Push locks' objects from a store, writing what the server lacks in bounded uploads. */
    async push(
        store: Pick<BuildStore, "closure" | "contents" | "head" | "lock" | "read">,
        locks: readonly Digest[],
        signal?: AbortSignal,
    ): Promise<void> {
        // list every lock's objects once, files first, and ask which the server has
        const roots = locks.map((digest) => ({ kind: "lock", digest }) as const);
        const objects = await BuildObject.of(store, roots, "all");
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
            signal?.throwIfAborted();
            await this.write(store.read(batch), signal);
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
