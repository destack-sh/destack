import { Digest } from "@destack/schema";
import { PackagePath, type PackageFile } from "@destack/package/file";
import { CallError } from "@destack/runtime/error";
import type { PackageStore } from "./store.ts";

/** The most bytes one upload holds: 64 MiB, which bounds its memory. */
const MAX_UPLOAD_BYTES = 64 * 1024 * 1024;

/** The host's decision on reading and uploading stored builds. */
export interface PackageAccess {
    /** Authorize reading a build and return its retention expiry, if temporary. */
    authorize(request: Request, digest: string): Promise<{ expiresAt?: number }>;
    /** Authorize an upload below a scope such as its package, absent where nobody uploads. */
    upload?(request: Request, scope: string | undefined): Promise<void>;
}

/** HTTP reads of stored builds, manifests, files and archives, and uploads by digest. */
export class PackageServer {
    /** The URL the reads are mounted at. */
    readonly url: URL;
    /** The stored builds. */
    readonly store: PackageStore;

    /** Serve a store at a URL. */
    constructor(url: URL, store: PackageStore) {
        this.url = url;
        this.store = store;
    }

    /** Decide whether a request's If-None-Match lists an entity tag, weakly compared. */
    static isUnchanged(request: Request, etag: string): boolean {
        return (
            request.headers
                .get("if-none-match")
                ?.split(",")
                .some((value) => {
                    const candidate = value.trim().replace(/^W\//u, "");

                    return candidate === "*" || candidate === etag;
                }) ?? false
        );
    }

    /** Serve immutable manifests, selected files and archives, as the host decides each read. */
    async fetch(request: Request, access: PackageAccess): Promise<Response> {
        try {
            return await this.#read(request, access);
        } catch (error) {
            // answer the failures that know their call error, rethrowing internal ones
            const failure = CallError.of(error);
            if (failure === undefined || failure.code === "INTERNAL_SERVER_ERROR") {
                throw error;
            }

            return Response.json(
                { error: failure.code, message: failure.message },
                { status: failure.status, headers: { "cache-control": "no-store" } },
            );
        }
    }

    /** Receive an upload: check a file, store a file by digest, or store a complete manifest. */
    async #write(request: Request, access: PackageAccess, path: string): Promise<Response> {
        // refuse uploads and find no uploaded file where nobody uploads
        if (access.upload === undefined) {
            return request.method === "PUT"
                ? new Response(null, { status: 405, headers: { allow: "GET, HEAD" } })
                : new Response(null, { status: 404 });
        }
        const target = UploadTarget.parse(path);
        await access.upload(request, target?.scope);

        // tell whether a file is held
        if (request.method === "HEAD") {
            const held =
                target?.kind === "file"
                    ? await this.store.bucket.head(`files/${target.digest}`)
                    : null;

            return new Response(null, { status: held === null ? 404 : 200 });
        }
        if (target === undefined) {
            return new Response(null, { status: 404 });
        }

        // read the upload's bytes within the limit
        const bytes = await readUpload(request);
        if (bytes === undefined) {
            return new Response(null, { status: 413 });
        }

        // store a file under its upload's media type, refusing an upload without one
        if (target.kind === "file") {
            const type = request.headers.get("content-type");
            if (type === null) {
                throw new CallError("BAD_REQUEST", {
                    message: `the upload of ${target.digest} has no media type`,
                });
            }
            await this.store.receive(target.digest, bytes, type);
        }
        // store a manifest
        else {
            await this.store.adopt(target.digest, bytes);
        }

        return new Response(null, { status: 201 });
    }

    /** Authorize the containing build before resolving paths or cache conditions. */
    async #read(request: Request, access: PackageAccess): Promise<Response> {
        // read the path below the mount
        const prefix = this.url.pathname.replace(/\/$/u, "") + "/";
        const pathname = new URL(request.url).pathname;
        if (!pathname.startsWith(prefix)) {
            return new Response(null, { status: 404 });
        }
        const path = pathname.slice(prefix.length);

        // receive uploads and probes of uploaded files, and accept reads only otherwise
        if (
            request.method === "PUT" ||
            (request.method === "HEAD" && UploadTarget.parse(path)?.kind === "file")
        ) {
            return this.#write(request, access, path);
        } else if (request.method !== "GET" && request.method !== "HEAD") {
            return new Response(null, { status: 405, headers: { allow: "GET, HEAD, PUT" } });
        }
        const separator = path.indexOf("/");
        const digest = path.slice(0, separator);
        if (separator < 0 || !Digest.safeParse(digest).success) {
            return new Response(null, { status: 404 });
        }

        // authorize the build, refusing an expired grant
        const grant = await access.authorize(request, digest);
        if (grant.expiresAt !== undefined && Date.now() >= grant.expiresAt) {
            return new Response(null, { status: 410, headers: { "cache-control": "no-store" } });
        }

        // stream the whole build as an archive, validated by the digest of the manifest it contains
        const selected = path.slice(separator + 1);
        if (selected === "archive") {
            return this.#archive(request, digest);
        }

        // resolve the selected path among the build's manifest and the files it lists
        const file = await this.#select(digest, selected);
        if (file instanceof Response) {
            return file;
        }

        return this.#serve(request, digest, selected, file);
    }

    /** Find the manifest or a listed file by its path, answering malformed or unknown paths. */
    async #select(digest: string, selected: string): Promise<PackageFile | Response> {
        // describe the manifest itself
        if (selected === "manifest.json") {
            return await this.store.head(digest);
        }
        // find nothing outside the files
        else if (!selected.startsWith("files/")) {
            return new Response(null, { status: 404 });
        }

        // refuse a malformed path
        const name = decodePath(selected.slice("files/".length));
        if (name === undefined || !PackagePath.safeParse(name).success) {
            return new Response(null, { status: 400 });
        }

        // look the path up among the manifest's direct references first, else in the list
        const contents = await this.store.contents(digest);
        const direct = contents.references().find((entry) => entry.path === name);
        const listed =
            direct ?? (await contents.distributed()).find((entry) => entry.path === name);

        return listed ?? new Response(null, { status: 404 });
    }

    /** Serve a manifest or a file under HTTP cache conditions and byte ranges. */
    async #serve(
        request: Request,
        digest: string,
        selected: string,
        file: PackageFile,
    ): Promise<Response> {
        // evaluate HTTP conditions only after access has been granted
        const etag = `"${file.digest}"`;
        const headers = new Headers({
            "content-type": file.mediaType,
            "content-length": String(file.size),
            "cache-control": "private, no-cache",
            vary: "Authorization, Cookie",
            etag,
            "accept-ranges": "bytes",
            "x-content-type-options": "nosniff",
        });
        if (PackageServer.isUnchanged(request, etag)) {
            headers.delete("content-length");

            return new Response(null, { status: 304, headers });
        }

        // read a satisfiable byte range of a GET, unless its validator refers to another version
        const range = readRange(request, etag, file.size);
        if (range === "unsatisfiable") {
            headers.set("content-range", `bytes */${file.size}`);
            headers.set("content-length", "0");

            return new Response(null, { status: 416, headers });
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

    /** Serve a stored build as one whole archive, answering cache conditions without file reads. */
    async #archive(request: Request, digest: string): Promise<Response> {
        // require the stored build before promising its archive
        await this.store.head(digest);
        const etag = `"${digest}.tar.gz"`;
        const headers = new Headers({
            "content-type": "application/gzip",
            "cache-control": "private, no-cache",
            vary: "Authorization, Cookie",
            etag,
            "x-content-type-options": "nosniff",
        });

        // answer unchanged archives and heads, else stream the archive
        if (PackageServer.isUnchanged(request, etag)) {
            return new Response(null, { status: 304, headers });
        } else if (request.method === "HEAD") {
            return new Response(null, { headers });
        }

        return new Response(this.store.export(digest), { headers });
    }
}

/** An upload's target: a file by its digest, or a manifest by the digest of its build. */
interface UploadTarget {
    /** What the upload stores. */
    readonly kind: "file" | "manifest";
    /** The digest naming it. */
    readonly digest: string;
    /** The scope the upload is sent below, such as its package, absent for none. */
    readonly scope: string | undefined;
}

/** The targets of uploads. */
const UploadTarget = {
    /** Read an upload's target, optionally below a scope as an OCI blob is below its repository. */
    parse(path: string): UploadTarget | undefined {
        // read the scope before a file or manifest path of two segments
        const segments = path.split("/");
        const scope = segments.length === 3 ? segments[0] : undefined;
        const [first = "", second = "", third] = scope === undefined ? segments : segments.slice(1);

        // name a file by its digest
        if (third === undefined && first === "files" && Digest.safeParse(second).success) {
            return { kind: "file", digest: second, scope };
        }
        // name a manifest by the digest of its build
        else if (
            third === undefined &&
            second === "manifest.json" &&
            Digest.safeParse(first).success
        ) {
            return { kind: "manifest", digest: first, scope };
        }
        // name nothing else
        else {
            return undefined;
        }
    },
};

/** Decode a percent-encoded path, absent when malformed. */
function decodePath(encoded: string): string | undefined {
    try {
        return decodeURIComponent(encoded);
    } catch (error) {
        if (!(error instanceof URIError)) {
            throw error;
        }

        return undefined;
    }
}

/** Read a GET's byte range of a file, absent for the whole file or another version. */
function readRange(
    request: Request,
    etag: string,
    size: number,
): { start: number; end: number } | "unsatisfiable" | undefined {
    // read a range of a GET whose validator names this version
    const requested = request.headers.get("range");
    const isCurrent = !request.headers.has("if-range") || request.headers.get("if-range") === etag;
    if (request.method !== "GET" || requested === null || !isCurrent) {
        return undefined;
    }

    // read the first and last bytes, a suffix length without a first byte
    const [, from, to] = /^bytes=(\d*)-(\d*)$/u.exec(requested) ?? [];
    if (from === undefined || to === undefined || (from === "" && to === "")) {
        return undefined;
    }
    const start = from === "" ? Math.max(0, size - Number(to)) : Number(from);
    const end = from !== "" && to !== "" ? Math.min(size - 1, Number(to)) : size - 1;

    // refuse a range outside the file
    const isSatisfiable =
        Number.isSafeInteger(start) && Number.isSafeInteger(end) && start <= end && start < size;

    return isSatisfiable ? { start, end } : "unsatisfiable";
}

/** Read an upload's bytes, or nothing once they exceed the limit. */
async function readUpload(request: Request): Promise<Uint8Array<ArrayBuffer> | undefined> {
    // refuse a declared length past the limit before reading
    if (Number(request.headers.get("content-length") ?? "0") > MAX_UPLOAD_BYTES) {
        return undefined;
    }

    // read the chunks, stopping once they exceed the limit
    const chunks: Uint8Array[] = [];
    let size = 0;
    for await (const chunk of request.body ?? []) {
        size += chunk.byteLength;
        if (size > MAX_UPLOAD_BYTES) {
            return undefined;
        }
        chunks.push(chunk);
    }

    // join the chunks
    const bytes = new Uint8Array(size);
    let offset = 0;
    for (const chunk of chunks) {
        bytes.set(chunk, offset);
        offset += chunk.byteLength;
    }

    return bytes;
}
