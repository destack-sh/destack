import type { Environment } from "@destack/environment";
import { Tarball, TarballEntry } from "@destack/package/archive";
import { Digest, schema } from "@destack/schema";
import type { BuildReader } from "@destack/package/manifest";
import { PackageManifest } from "@destack/package/manifest";
import { BuildError } from "../error/index.ts";
import { distinctFiles } from "./store.ts";

/** The client of a package server, pushing builds by digest as registry clients push images. */
export class PackageClient {
    /** The URL the server's builds are mounted at, ending in a slash. */
    readonly url: string;
    /** Send a request to the server. */
    readonly #fetch: (request: Request) => Promise<Response>;
    /** The hashing of manifests and the compression of uploads. */
    readonly #environment: Pick<Environment, "crypto" | "compression">;

    /** Call a package server at a URL through a fetch, in an environment. */
    constructor(
        url: string,
        fetch: (request: Request) => Promise<Response>,
        environment: Pick<Environment, "crypto" | "compression">,
    ) {
        this.url = url.endsWith("/") ? url : `${url}/`;
        this.#fetch = fetch;
        this.#environment = environment;
    }

    /** Push a build's missing files, then its manifest under its digest, and return the digest. */
    async push(build: BuildReader, signal?: AbortSignal): Promise<Digest> {
        // upload the distinct files the server lacks
        const files = distinctFiles(await build.distributed());
        await this.upload(
            files.map((file) => ({
                digest: file.digest,
                load: () => build.load(file.path, signal),
            })),
            signal,
        );

        // upload the manifest last in the store's encoding, under the build's digest
        signal?.throwIfAborted();
        const manifest = PackageManifest.serialize(build.manifest);
        const digest = await Digest.of(manifest, this.#environment.crypto);
        await this.#upload(
            new Request(`${this.url}${digest}/manifest.json`, {
                method: "PUT",
                body: manifest,
                headers: { "content-type": "application/json" },
            }),
        );

        return digest;
    }

    /** Upload the files a server lacks: ask which are missing, then send those in one tarball. */
    async upload(files: readonly Upload[], signal?: AbortSignal): Promise<void> {
        // ask which files the server lacks, as Bazel's remote cache finds missing blobs
        const probe = `${this.url}files/missing`;
        const asked = await this.#fetch(
            new Request(probe, {
                method: "POST",
                headers: { "content-type": "application/json" },
                body: JSON.stringify(files.map((file) => file.digest)),
                signal: signal ?? null,
            }),
        );
        if (!asked.ok) {
            await asked.body?.cancel();
            throw BuildError.of(
                "PROBE_FAILED",
                `cannot probe ${new URL(probe).pathname}: HTTP ${asked.status}`,
            );
        }
        const missing = new Set(schema.array(Digest).parse(await asked.json()));

        // send the missing files in one tarball, each named by its digest, reading each once
        const sending = files.filter((file) => missing.has(file.digest));
        if (sending.length === 0) {
            return;
        }
        const entries = (async function* () {
            for (const file of sending) {
                yield TarballEntry.of(file.digest, await file.load());
            }
        })();
        await this.#upload(
            new Request(`${this.url}files`, {
                method: "POST",
                headers: { "content-type": "application/gzip" },
                body: Tarball.stream(entries, this.#environment.compression),
                signal: signal ?? null,
            }),
        );
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

/** A file to upload: its digest, and how to read its bytes once the server lacks it. */
export interface Upload {
    /** The file's digest. */
    readonly digest: Digest;
    /** Read the file's bytes. */
    readonly load: () => Promise<Uint8Array<ArrayBuffer>>;
}
