import { Tarball } from "@destack/package/archive";
import { Digest, schema } from "@destack/schema";
import { type PackageFile, PackagePath } from "@destack/package/file";
import { MANIFEST_PATH } from "@destack/package/manifest";
import { CallError } from "@destack/object";
import { BuildObject, BuildStore } from "./store.ts";

/** How a store endpoint authorizes pulls, and pushes where it takes them, as OCI actions. */
export interface PackageAccess {
    /** Authorize pulling a build, refusing a request the host does not grant it. */
    readonly pull: (request: Request, digest: Digest) => Promise<void>;
    /** Authorize a push below an account's handle, absent for a read-only endpoint. */
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

/** Serve immutable manifests, locks and selected files, as the host decides each read. */
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

/** Receive an upload: tell which blobs the store has, or write a tarball of objects. */
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

    // tell which of the asked blobs the store has
    if (target?.kind === "has" && request.method === "POST") {
        const asked = schema.array(Digest).parse(await request.json());

        return Response.json(await store.has(asked));
    }
    // write a tarball of objects, files first
    else if (target?.kind === "write" && request.method === "POST" && request.body !== null) {
        await store.write(Tarball.read(request.body));

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

    // read objects of a lock or build, receive uploads, and accept reads only otherwise
    const [kind, root, verb] = path.split("/");
    const reading = Digest.safeParse(root);
    if (
        request.method === "POST" &&
        (kind === "lock" || kind === "manifest") &&
        verb === "read" &&
        reading.success
    ) {
        await access.pull(request, reading.data);

        return await objects(store, { kind, digest: reading.data }, request);
    } else if (request.method === "POST") {
        return write(store, request, access, path);
    } else if (request.method !== "GET" && request.method !== "HEAD") {
        return new Response(null, { status: 405, headers: { allow: "GET, HEAD, POST" } });
    }
    const separator = path.indexOf("/");
    const parsed = Digest.safeParse(path.slice(0, separator));
    if (separator < 0 || !parsed.success) {
        return new Response(null, { status: 404 });
    }
    const digest = parsed.data;

    // authorize the build
    await access.pull(request, digest);

    // describe a referrer of the build, as OCI's referrers API lists a subject's artifacts
    const selected = path.slice(separator + 1);
    if (selected.startsWith("referrers/")) {
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
    // describe the manifest or the lock itself
    if (selected === MANIFEST_PATH || selected === "lock.json") {
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

    // find the path among the files the build distributes
    const distributed = await (await store.contents(digest)).distributed();

    return distributed.find((entry) => entry.path === name) ?? new Response(null, { status: 404 });
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

/** Stream objects of a lock or build as a tarball, refusing any outside its objects. */
async function objects(
    store: BuildStore,
    root: Pick<BuildObject, "kind" | "digest">,
    request: Request,
): Promise<Response> {
    // read the asked objects, requiring each among the root's
    const asked = schema
        .array(schema.object({ kind: schema.enum(["file", "manifest", "lock"]), digest: Digest }))
        .parse(await request.json());
    const owned = new Set(
        (await BuildObject.of(store, [root], "all")).map((object) => object.digest),
    );
    const outside = asked.find((object) => !owned.has(object.digest));
    if (outside !== undefined) {
        return new Response(null, { status: 404 });
    }

    // stream them as a tarball
    return new Response(Tarball.stream(store.read(asked)), {
        headers: { "content-type": "application/x-tar", "cache-control": "no-store" },
    });
}

/** An upload's target: telling which blobs the store has, or writing objects, below a scope. */
type UploadTarget = {
    /** What the upload does. */
    readonly kind: "has" | "write";
    /** The scope the upload is sent below, such as its account's handle, absent for none. */
    readonly scope: string | undefined;
};

/** The targets of uploads. */
const UploadTarget = {
    /** Read an upload's target, optionally below a scope as an OCI blob is below its repository. */
    parse(path: string): UploadTarget | undefined {
        // read the verb, below at most one scope segment
        const segments = path.split("/");
        const [last = ""] = segments.toReversed();
        const scope = segments.length === 2 ? segments[0] : undefined;

        return segments.length <= 2 && (last === "has" || last === "write")
            ? { kind: last, scope }
            : undefined;
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
