import { Tarball, TarballEntry } from "@destack/package/archive";
import { Digest, schema } from "@destack/schema";
import { type PackageFile, PackagePath } from "@destack/package/file";
import { CallError } from "@destack/object";
import { BuildStore } from "./store.ts";

/** The most bytes one upload has: 64 MiB, which bounds its memory. */
const MAX_UPLOAD_BYTES = 64 * 1024 * 1024;

/** How a store endpoint authorizes pulls, and pushes where it takes them, as OCI actions. */
export interface PackageAccess {
    /** Authorize pulling a build, refusing a request the host does not grant it. */
    readonly pull: (request: Request, digest: Digest) => Promise<void>;
    /** Authorize a push below a scope such as its package, absent for a read-only endpoint. */
    readonly push?: (request: Request, scope: string | undefined) => Promise<void>;
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
    store: BuildStore,
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

/** Receive an upload: list missing files, store a tarball of files, or store a manifest. */
async function write(
    store: BuildStore,
    request: Request,
    access: PackageAccess,
    path: string,
): Promise<Response> {
    // refuse pushes at a read-only endpoint, as a read-only OCI registry
    if (access.push === undefined) {
        return new Response(null, { status: 405, headers: { allow: "GET, HEAD" } });
    }
    const target = UploadTarget.parse(path);
    await access.push(request, target?.scope);

    // list the files among the asked digests the store lacks
    if (target?.kind === "missing" && request.method === "POST") {
        const asked = schema.array(Digest).parse(await request.json());

        return Response.json(await store.missing(asked));
    }
    // store a tarball's files under the digests they are named by, a pack per upload's bytes
    else if (target?.kind === "files" && request.method === "POST" && request.body !== null) {
        let gathering: { readonly digest: string; readonly bytes: Uint8Array<ArrayBuffer> }[] = [];
        let gathered = 0;
        for await (const entry of Tarball.read(request.body, store.environment.compression)) {
            // refuse a file above the limit
            if (entry.size > MAX_UPLOAD_BYTES) {
                return new Response(null, { status: 413 });
            }

            // store the gathered files once they reach the limit
            gathering.push({ digest: entry.path, bytes: await TarballEntry.bytes(entry) });
            gathered += entry.size;
            if (gathered >= MAX_UPLOAD_BYTES) {
                await store.receive(gathering);
                gathering = [];
                gathered = 0;
            }
        }
        await store.receive(gathering);

        return new Response(null, { status: 201 });
    }
    // store a manifest within the limit
    else if (target?.kind === "manifest" && request.method === "PUT") {
        const bytes = await readUpload(request);
        if (bytes === undefined) {
            return new Response(null, { status: 413 });
        }
        await store.adopt(target.digest, bytes);

        return new Response(null, { status: 201 });
    }

    return new Response(null, { status: 404 });
}

/** Authorize the containing build before resolving paths or cache conditions. */
async function read(
    store: BuildStore,
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

    // receive uploads, and accept reads only otherwise
    if (request.method === "PUT" || request.method === "POST") {
        return write(store, request, access, path);
    } else if (request.method !== "GET" && request.method !== "HEAD") {
        return new Response(null, { status: 405, headers: { allow: "GET, HEAD, POST, PUT" } });
    }
    const separator = path.indexOf("/");
    const parsed = Digest.safeParse(path.slice(0, separator));
    if (separator < 0 || !parsed.success) {
        return new Response(null, { status: 404 });
    }
    const digest = parsed.data;

    // authorize the build
    await access.pull(request, digest);

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
    store: BuildStore,
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
async function referrer(store: BuildStore, digest: Digest, encoded: string): Promise<Response> {
    const path = decodePath(encoded);
    const described = path === undefined ? undefined : await store.referrer(digest, path);

    return described === undefined
        ? new Response(null, { status: 404 })
        : Response.json(described, { headers: { "cache-control": "private, no-cache" } });
}

/** Serve a manifest or a file under HTTP cache conditions and byte ranges. */
async function serve(
    store: BuildStore,
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
async function archive(store: BuildStore, request: Request, digest: Digest): Promise<Response> {
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

    // stream the whole build, or the output the request asks for
    const output = new URL(request.url).searchParams.get("output") ?? undefined;

    return new Response(store.export(digest, output), { headers });
}

/** An upload's target: the files a push asks about or sends, or a build's manifest. */
type UploadTarget =
    | {
          /** The digests a push asks the store's missing files among. */
          readonly kind: "missing";
          /** The scope the upload is sent below, such as its package, absent for none. */
          readonly scope: string | undefined;
      }
    | {
          /** A tarball of files, each named by its digest. */
          readonly kind: "files";
          /** The scope the upload is sent below, such as its package, absent for none. */
          readonly scope: string | undefined;
      }
    | {
          /** A build's manifest. */
          readonly kind: "manifest";
          /** The digest of the build. */
          readonly digest: Digest;
          /** The scope the upload is sent below, such as its package, absent for none. */
          readonly scope: string | undefined;
      };

/** The targets of uploads. */
const UploadTarget = {
    /** Read an upload's target, optionally below a scope as an OCI blob is below its repository. */
    parse(path: string): UploadTarget | undefined {
        // read the files' paths, then a manifest's, each optionally below one scope segment
        const segments = path.split("/");
        const [last = "", previous = ""] = segments.toReversed();
        const manifest = Digest.safeParse(previous);
        if (segments.length <= 3 && previous === "files" && last === "missing") {
            return { kind: "missing", scope: segments.length === 3 ? segments[0] : undefined };
        } else if (segments.length <= 2 && last === "files") {
            return { kind: "files", scope: segments.length === 2 ? segments[0] : undefined };
        } else if (segments.length <= 3 && last === "manifest.json" && manifest.success) {
            const scope = segments.length === 3 ? segments[0] : undefined;

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
