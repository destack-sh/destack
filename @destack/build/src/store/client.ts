import { Digest } from "@destack/schema";
import type { BuildReader } from "@destack/package/manifest";
import { PackageManifest } from "@destack/package/manifest";
import { ServiceError } from "@destack/service/error";
import { distinctFiles, encodeManifest, inBatches } from "./store.ts";

/** The client of a package server: it pushes builds by digest, as a registry client pushes images. */
export class PackageClient {
    /** The URL the server's builds are mounted at, ending in a slash. */
    readonly url: string;
    /** Send a request to the server. */
    readonly #fetch: (request: Request) => Promise<Response>;

    /** Reach a package server at a URL through a fetch. */
    constructor(url: string, fetch: (request: Request) => Promise<Response>) {
        this.url = url.endsWith("/") ? url : `${url}/`;
        this.#fetch = fetch;
    }

    /** Push a build: each file the server lacks, then the manifest under the build's digest, returning that digest. */
    async push(build: BuildReader, signal?: AbortSignal): Promise<string> {
        // upload each distinct file the server lacks
        const files = distinctFiles(await build.distributed());
        await inBatches(files, signal, async (file) => {
            // ask whether the server holds the file
            const target = `${this.url}files/${file.digest}`;
            const held = await this.#fetch(new Request(target, { method: "HEAD" }));

            // upload a file the server lacks
            if (held.status === 404) {
                const body = await build.load(file.path, signal);
                const headers = { "content-type": file.mediaType };
                await this.#upload(new Request(target, { method: "PUT", body, headers }));
            }
            // refuse a failed probe
            else if (!held.ok) {
                throw new ServiceError("BAD_GATEWAY", {
                    message: `cannot probe ${new URL(target).pathname}: HTTP ${held.status}`,
                });
            }
        });

        // upload the manifest last in the store's encoding, under the build's digest
        signal?.throwIfAborted();
        const manifest = encodeManifest(PackageManifest.parse(build.manifest));
        const digest = await Digest.of(manifest);
        await this.#upload(
            new Request(`${this.url}${digest}/manifest.json`, {
                method: "PUT",
                body: manifest,
                headers: { "content-type": "application/json" },
            }),
        );

        return digest;
    }

    /** Send an upload, refusing a failed one. */
    async #upload(request: Request): Promise<void> {
        const response = await this.#fetch(request);
        if (!response.ok) {
            throw new ServiceError("BAD_GATEWAY", {
                message: `cannot upload ${new URL(request.url).pathname}: HTTP ${response.status}`,
            });
        }
    }
}
