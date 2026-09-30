import { Digest, PackagePath, type PackageFile } from "@destack/package/file";
import { PackageError } from "@destack/package/error";
import { ServiceError } from "@destack/service/error";
import type { PackageStore } from "./store.ts";

/** The host's decision on reading a stored build. */
export interface PackageAccess {
    /** Authorize reading a build and return its retention expiry, if temporary. */
    authorize(request: Request, digest: string): Promise<{ expiresAt?: number }>;
}

/** HTTP reads of stored builds: their manifests, files and whole archives. */
export class PackageServer {
    /** The URL the reads are mounted at. */
    readonly url: URL;
    /** The stored builds. */
    readonly store: PackageStore;
    /** The host's decision on each read. */
    readonly access: PackageAccess;

    /** Serve a store at a URL. */
    constructor(url: URL, store: PackageStore, access: PackageAccess) {
        this.url = url;
        this.store = store;
        this.access = access;
    }

    /** Decide whether a request's If-None-Match lists an entity tag, weakly compared. */
    static isUnchanged(request: Request, etag: string): boolean {
        return (
            request.headers
                .get("if-none-match")
                ?.split(",")
                .some((value) => {
                    const candidate = value.trim().replace(/^W\//, "");

                    return candidate === "*" || candidate === etag;
                }) ?? false
        );
    }

    /** Serve immutable manifests, selected files and complete archives. */
    async fetch(request: Request): Promise<Response> {
        try {
            return await this.#read(request);
        } catch (error) {
            // answer service and package failures with their code, and rethrow every other failure
            if (!(error instanceof ServiceError || error instanceof PackageError)) {
                throw error;
            }

            return Response.json(
                { error: error.code, message: error.message },
                {
                    status: error instanceof ServiceError ? error.status : 400,
                    headers: { "cache-control": "no-store" },
                },
            );
        }
    }

    /** Authorize the containing build before resolving paths or cache conditions. */
    async #read(request: Request): Promise<Response> {
        // accept reads only
        if (request.method !== "GET" && request.method !== "HEAD") {
            return new Response(null, { status: 405, headers: { allow: "GET, HEAD" } });
        }

        // read the build's digest below the mount
        const prefix = this.url.pathname.replace(/\/$/, "") + "/";
        const pathname = new URL(request.url).pathname;
        if (!pathname.startsWith(prefix)) {
            return new Response(null, { status: 404 });
        }
        const path = pathname.slice(prefix.length);
        const separator = path.indexOf("/");
        const digest = path.slice(0, separator);
        if (separator < 0 || !Digest.safeParse(digest).success) {
            return new Response(null, { status: 404 });
        }

        // authorize the build, refusing an expired grant
        const grant = await this.access.authorize(request, digest);
        if (grant.expiresAt !== undefined && Date.now() >= grant.expiresAt) {
            return new Response(null, { status: 410, headers: { "cache-control": "no-store" } });
        }

        // stream the whole build as an archive, validated by the digest of the manifest it contains
        const selected = path.slice(separator + 1);
        if (selected === "archive") {
            return this.#archive(request, digest);
        }

        // resolve the selected path among the build's manifest and the files it lists
        let file: PackageFile;

        // describe the manifest itself
        if (selected === "manifest.json") {
            file = await this.store.head(digest);
        }
        // find a file the manifest or its inventory lists, refusing a malformed path
        else if (selected.startsWith("files/")) {
            let name: string;
            try {
                name = decodeURIComponent(selected.slice("files/".length));
            } catch (error) {
                if (!(error instanceof URIError)) {
                    throw error;
                }

                return new Response(null, { status: 400 });
            }
            if (!PackagePath.safeParse(name).success) {
                return new Response(null, { status: 400 });
            }

            // look the path up among the manifest's own files first, else in the inventory
            const contents = await this.store.contents(digest);
            const direct = contents.reader.references().find((entry) => entry.path === name);
            const found =
                direct ?? (await contents.reader.inventory()).find((entry) => entry.path === name);
            if (!found) {
                return new Response(null, { status: 404 });
            }
            file = found;
        }
        // find nothing else
        else {
            return new Response(null, { status: 404 });
        }

        // evaluate HTTP conditions only after access has been granted
        const headers = new Headers({
            "content-type": file.mediaType,
            "content-length": String(file.size),
            "cache-control": "private, no-cache",
            vary: "Authorization, Cookie",
            etag: `"${file.digest}"`,
            "accept-ranges": "bytes",
            "x-content-type-options": "nosniff",
        });
        if (PackageServer.isUnchanged(request, headers.get("etag")!)) {
            headers.delete("content-length");

            return new Response(null, { status: 304, headers });
        }

        // read a satisfiable byte range of a GET, unless its validator refers to another version
        let range: { start: number; end: number } | undefined;
        const requested = request.headers.get("range");
        if (
            request.method === "GET" &&
            requested &&
            (!request.headers.has("if-range") ||
                request.headers.get("if-range") === headers.get("etag"))
        ) {
            const match = /^bytes=(\d*)-(\d*)$/.exec(requested);
            if (match && (match[1] || match[2])) {
                const start = match[1]
                    ? Number(match[1])
                    : Math.max(0, file.size - Number(match[2]));
                const end =
                    match[1] && match[2]
                        ? Math.min(file.size - 1, Number(match[2]))
                        : file.size - 1;
                if (
                    Number.isSafeInteger(start) &&
                    Number.isSafeInteger(end) &&
                    start <= end &&
                    start < file.size
                ) {
                    range = { start, end };
                } else {
                    headers.set("content-range", `bytes */${file.size}`);
                    headers.set("content-length", "0");

                    return new Response(null, { status: 416, headers });
                }
            }
        }

        // answer a head without a body
        if (request.method === "HEAD") {
            return new Response(null, { headers });
        }

        // stream through cancellation
        const selection = range
            ? { offset: range.start, length: range.end - range.start + 1 }
            : undefined;
        const body =
            selected === "manifest.json"
                ? (await this.store.manifest(digest, selection)).body
                : (await this.store.open(file, selection)).body;

        // describe the range the body covers
        if (range) {
            headers.set("content-range", `bytes ${range.start}-${range.end}/${file.size}`);
            headers.set("content-length", String(range.end - range.start + 1));
        }

        return new Response(body.pipeThrough(new TransformStream(), { signal: request.signal }), {
            status: range ? 206 : 200,
            headers,
        });
    }

    /** Serve a stored build as one archive, whole, which answers cache conditions without reading its files. */
    async #archive(request: Request, digest: string): Promise<Response> {
        // require the stored build before promising its archive
        await this.store.head(digest);
        const headers = new Headers({
            "content-type": "application/gzip",
            "cache-control": "private, no-cache",
            vary: "Authorization, Cookie",
            etag: `"${digest}.tar.gz"`,
            "x-content-type-options": "nosniff",
        });

        // answer unchanged archives and heads, else stream the archive
        if (PackageServer.isUnchanged(request, headers.get("etag")!)) {
            return new Response(null, { status: 304, headers });
        } else if (request.method === "HEAD") {
            return new Response(null, { headers });
        }

        return new Response(this.store.export(digest), { headers });
    }
}
