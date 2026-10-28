import { Digest, type DigestCrypto } from "@destack/schema";
import type { BuildReader } from "@destack/package/manifest";
import { PackageManifest } from "@destack/package/manifest";
import { BuildError } from "../error/index.ts";
import { distinctFiles, encodeManifest, inBatches } from "./store.ts";

/** The client of a package server, pushing builds by digest as registry clients push images. */
export class PackageClient {
    /** The URL the server's builds are mounted at, ending in a slash. */
    readonly url: string;
    /** Send a request to the server. */
    readonly #fetch: (request: Request) => Promise<Response>;
    /** The hashing the manifest's digest is taken with. */
    readonly #crypto: DigestCrypto;

    /** Call a package server at a URL through a fetch, digesting with some crypto. */
    constructor(url: string, fetch: (request: Request) => Promise<Response>, crypto: DigestCrypto) {
        this.url = url.endsWith("/") ? url : `${url}/`;
        this.#fetch = fetch;
        this.#crypto = crypto;
    }

    /** Push a build's missing files, then its manifest under its digest, and return the digest. */
    async push(build: BuildReader, signal?: AbortSignal): Promise<Digest> {
        // upload each distinct file the server lacks
        const files = distinctFiles(await build.distributed());
        await inBatches(files, signal, async (file) => {
            await this.upload(file.digest, () => build.load(file.path, signal));
        });

        // upload the manifest last in the store's encoding, under the build's digest
        signal?.throwIfAborted();
        const manifest = encodeManifest(PackageManifest.parse(build.manifest));
        const digest = await Digest.of(manifest, this.#crypto);
        await this.#upload(
            new Request(`${this.url}${digest}/manifest.json`, {
                method: "PUT",
                body: manifest,
                headers: { "content-type": "application/json" },
            }),
        );

        return digest;
    }

    /** Upload a file the server lacks under its digest, reading its bytes only if needed. */
    async upload(digest: Digest, load: () => Promise<Uint8Array<ArrayBuffer>>): Promise<void> {
        // ask whether the server has the file
        const target = `${this.url}files/${digest}`;
        const stored = await this.#fetch(new Request(target, { method: "HEAD" }));
        await stored.body?.cancel();

        // upload a file the server lacks
        if (stored.status === 404) {
            await this.#upload(new Request(target, { method: "PUT", body: await load() }));
        }
        // refuse a failed probe
        else if (!stored.ok) {
            throw BuildError.of(
                "PROBE_FAILED",
                `cannot probe ${new URL(target).pathname}: HTTP ${stored.status}`,
            );
        }
    }

    /** Send an upload, releasing its answer's body, and refuse a failed one. */
    async #upload(request: Request): Promise<void> {
        const response = await this.#fetch(request);
        await response.body?.cancel();
        if (!response.ok) {
            throw BuildError.of(
                "UPLOAD_FAILED",
                `cannot upload ${new URL(request.url).pathname}: HTTP ${response.status}`,
            );
        }
    }
}
