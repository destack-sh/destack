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
            // ask whether the server has the file
            const target = `${this.url}files/${file.digest}`;
            const held = await this.#fetch(new Request(target, { method: "HEAD" }));
            await held.body?.cancel();

            // upload a file the server lacks
            if (held.status === 404) {
                const body = await build.load(file.path, signal);
                const headers = { "content-type": file.mediaType };
                await this.#upload(new Request(target, { method: "PUT", body, headers }));
            }
            // refuse a failed probe
            else if (!held.ok) {
                throw BuildError.of(
                    "PROBE_FAILED",
                    `cannot probe ${new URL(target).pathname}: HTTP ${held.status}`,
                );
            }
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
