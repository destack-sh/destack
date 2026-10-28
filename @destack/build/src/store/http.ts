import { Digest } from "@destack/schema";
import { type PackageFile, PackagePath } from "@destack/package/file";
import { CallError } from "@destack/object";
import { PackageStore } from "./store.ts";

/** The most bytes one upload has: 64 MiB, which bounds its memory. */
const MAX_UPLOAD_BYTES = 64 * 1024 * 1024;

/** The host's decision on reading and uploading stored builds. */
export interface PackageAccess {
    /** Authorize reading a build, refusing a request the host does not grant it. */
    readonly authorize: (request: Request, digest: Digest) => Promise<void>;
    /** Authorize an upload below a scope such as its package, absent where nobody uploads. */
    readonly upload?: (request: Request, scope: string | undefined) => Promise<void>;
}

/** Decide whether a request's If-None-Match lists an entity tag, weakly compared. */
export function isUnchanged(request: Request, etag: string): boolean {
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
export async function serveStore(
    store: PackageStore,
    mount: URL,
    request: Request,
    access: PackageAccess,
): Promise<Response> {
    try {
        return await read(store, mount, request, access);
    } catch (error) {
        // answer the failures that know their call error with its body, rethrowing internal ones
        const failure = CallError.of(error);
        if (failure === undefined || failure.code === "INTERNAL_SERVER_ERROR") {
            throw error;
        }

        return Response.json(failure.toJSON(), {
            status: failure.status,
            headers: { "cache-control": "no-store" },
        });
    }
}

/** Receive an upload: check a file, store a file by digest, or store a complete manifest. */
async function write(
    store: PackageStore,
    request: Request,
    access: PackageAccess,
    path: string,
): Promise<Response> {
    // refuse uploads and find no uploaded file where nobody uploads
    if (access.upload === undefined) {
        return request.method === "PUT"
            ? new Response(null, { status: 405, headers: { allow: "GET, HEAD" } })
            : new Response(null, { status: 404 });
    }
    const target = UploadTarget.parse(path);
    await access.upload(request, target?.scope);

    // tell whether the store has a file
    if (request.method === "HEAD") {
        const isStored = target?.kind === "file" && (await store.has(target.digest));

        return new Response(null, { status: isStored ? 200 : 404 });
    }
    if (target === undefined) {
        return new Response(null, { status: 404 });
    }

    // read the upload's bytes within the limit
    const bytes = await readUpload(request);
    if (bytes === undefined) {
        return new Response(null, { status: 413 });
    }

    // store a file under its digest
    if (target.kind === "file") {
        await store.receive(target.digest, bytes);
    }
    // store a manifest
    else {
        await store.adopt(target.digest, bytes);
    }

    return new Response(null, { status: 201 });
}

/** Authorize the containing build before resolving paths or cache conditions. */
async function read(
    store: PackageStore,
    mount: URL,
    request: Request,
    access: PackageAccess,
): Promise<Response> {
    // read the path below the mount
    const prefix = mount.pathname.replace(/\/$/u, "") + "/";
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
        return write(store, request, access, path);
    } else if (request.method !== "GET" && request.method !== "HEAD") {
        return new Response(null, { status: 405, headers: { allow: "GET, HEAD, PUT" } });
    }
    const separator = path.indexOf("/");
    const parsed = Digest.safeParse(path.slice(0, separator));
    if (separator < 0 || !parsed.success) {
        return new Response(null, { status: 404 });
    }
    const digest = parsed.data;

    // authorize the build
    await access.authorize(request, digest);

    // stream the whole build as an archive, validated by the digest of the manifest it contains
    const selected = path.slice(separator + 1);
    if (selected === "archive") {
        return archive(store, request, digest);
    }
    // describe a referrer of the build, as OCI's referrers API lists a subject's artifacts
    else if (selected.startsWith("referrers/")) {
        return referrer(store, digest, selected.slice("referrers/".length));
    }

    // resolve the selected path among the build's manifest and the files it lists
    const file = await select(store, digest, selected);
    if (file instanceof Response) {
        return file;
    }

    return serve(store, request, digest, selected, file);
}

/** Find the manifest or a listed file by its path, answering malformed or unknown paths. */
async function select(
    store: PackageStore,
    digest: Digest,
    selected: string,
): Promise<PackageFile | Response> {
    // describe the manifest itself
    if (selected === "manifest.json") {
        return await store.head(digest);
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
    const contents = await store.contents(digest);
    const direct = contents.references().find((entry) => entry.path === name);
    const listed = direct ?? (await contents.distributed()).find((entry) => entry.path === name);

    return listed ?? new Response(null, { status: 404 });
}

/** Describe a referrer of a build by its encoded path, answering an unknown one as missing. */
async function referrer(store: PackageStore, digest: Digest, encoded: string): Promise<Response> {
    const path = decodePath(encoded);
    const described = path === undefined ? undefined : await store.referrer(digest, path);

    return described === undefined
        ? new Response(null, { status: 404 })
        : Response.json(described, { headers: { "cache-control": "private, no-cache" } });
}

/** Serve a manifest or a file under HTTP cache conditions and byte ranges. */
async function serve(
    store: PackageStore,
    request: Request,
    digest: Digest,
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
    if (isUnchanged(request, etag)) {
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

    // open the selected bytes, which a disconnecting client cancels
    const selection = range
        ? { offset: range.start, length: range.end - range.start + 1 }
        : undefined;
    const body = store.open(file, selection);

    // describe the range the body covers
    if (range) {
        headers.set("content-range", `bytes ${range.start}-${range.end}/${file.size}`);
        headers.set("content-length", String(range.end - range.start + 1));
    }

    return new Response(body, { status: range ? 206 : 200, headers });
}

/** Serve a stored build as one whole archive, answering cache conditions without file reads. */
async function archive(store: PackageStore, request: Request, digest: Digest): Promise<Response> {
    // require the stored build before promising its archive
    await store.head(digest);
    const etag = `"${digest}.tar.gz"`;
    const headers = new Headers({
        "content-type": "application/gzip",
        "cache-control": "private, no-cache",
        vary: "Authorization, Cookie",
        etag,
        "x-content-type-options": "nosniff",
    });

    // answer unchanged archives and heads, else stream the archive
    if (isUnchanged(request, etag)) {
        return new Response(null, { status: 304, headers });
    } else if (request.method === "HEAD") {
        return new Response(null, { headers });
    }

    return new Response(store.export(digest), { headers });
}

/** An upload's target: a file by its digest, or a manifest by the digest of its build. */
interface UploadTarget {
    /** What the upload stores. */
    readonly kind: "file" | "manifest";
    /** The digest keying it. */
    readonly digest: Digest;
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

        // key a file by its digest
        const file = Digest.safeParse(second);
        const manifest = Digest.safeParse(first);
        if (third === undefined && first === "files" && file.success) {
            return { kind: "file", digest: file.data, scope };
        }
        // key a manifest by the digest of its build
        else if (third === undefined && second === "manifest.json" && manifest.success) {
            return { kind: "manifest", digest: manifest.data, scope };
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
