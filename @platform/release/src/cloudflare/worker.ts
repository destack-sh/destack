import { Metadata, MetadataKind } from "@tufjs/models";
import stable from "../../../../@destack/cli/src/update/root.json" with { type: "json" };
import nightly from "../../../../@destack/cli/src/update/nightly.json" with { type: "json" };
import { Publication, type PublicationBucket, type StoredDocument } from "../server/publication.ts";
import { PUBLIC_PATH } from "../repository/layout.ts";

/** The bootstrap files the root serves from stable, or from nightly until stable has them. */
const ROOT_FILES = new Set(["/install", "/downloads.json"]);

/** Maximum renewal request size, in bytes. */
const REQUEST_SIZE = 128 * 1024;

/** A stored release object's metadata as the Worker serves it. */
interface ServedObject {
    /** The object's size in bytes. */
    readonly size: number;
    /** The object's entity tag quoted for HTTP. */
    readonly httpEtag: string;
    /** The HTTP metadata stored with the object. */
    readonly httpMetadata?: { readonly contentType?: string };
    /** The object's contents, absent from metadata reads. */
    readonly body?: ReadableStream;
}

/** A stored release object with its streamed contents. */
interface ServedBody extends ServedObject, StoredDocument {
    /** The object's contents. */
    readonly body: ReadableStream;
}

/** The bucket operations the Worker needs, as R2 offers them. */
interface R2ReleaseBucket extends PublicationBucket {
    /** Read an object with its contents, null when absent. */
    get(key: string): Promise<ServedBody | null>;
    /** Read an object's metadata, null when absent. */
    head(key: string): Promise<ServedObject | null>;
}

/** Release buckets bound by the deployment administrator. */
interface PublicationEnvironment {
    /** Stable releases writable only by approved release jobs and this service. */
    STABLE_RELEASES: R2ReleaseBucket;
    /** Nightly releases writable by unattended nightly jobs and this service. */
    NIGHTLY_RELEASES: R2ReleaseBucket;
}

/** Serve public downloads and accept only signed freshness renewals. */
async function fetch(request: Request, environment: PublicationEnvironment): Promise<Response> {
    // select one fixed repository without accepting arbitrary bucket names or object paths
    const url = new URL(request.url);
    const isRoot = ROOT_FILES.has(url.pathname);
    const pathName = isRoot ? `/stable${url.pathname}` : url.pathname;
    const match = /^\/(stable|nightly)\/(.+)$/u.exec(pathName);
    const name = match?.[1];
    const path = match?.[2];
    if (name === undefined || path === undefined) {
        return new Response("not found", { status: 404 });
    }
    const bucket = name === "stable" ? environment.STABLE_RELEASES : environment.NIGHTLY_RELEASES;

    // expose a write endpoint limited to the two authenticated freshness documents
    if (path === "renew" && request.method === "POST") {
        return renew(request, bucket, name);
    }

    // restrict public reads to release metadata, immutable downloads and bootstrap files
    if (request.method !== "GET" && request.method !== "HEAD") {
        return new Response("method not allowed", { status: 405, headers: { allow: "GET, HEAD" } });
    }
    if (!PUBLIC_PATH.test(path)) {
        return new Response("not found", { status: 404 });
    }
    const read = (source: R2ReleaseBucket, key: string) =>
        request.method === "HEAD" ? source.head(key) : source.get(key);
    const object =
        (await read(bucket, `${name}/${path}`)) ??
        (isRoot ? await read(environment.NIGHTLY_RELEASES, `nightly/${path}`) : null);
    if (!object) {
        return new Response("not found", { status: 404 });
    }

    return respond(object, path);
}

/** Renew the selected repository with the signed snapshot and timestamp of the request. */
async function renew(request: Request, bucket: R2ReleaseBucket, name: string): Promise<Response> {
    // authenticate against the root embedded for the channel
    const root = Metadata.fromJSON(MetadataKind.Root, name === "stable" ? stable : nightly);
    const publication = new Publication(bucket, `${name}/`, root);
    try {
        const input = await readRequest(request);
        await publication.renew(Buffer.from(input.snapshot), Buffer.from(input.timestamp));

        return new Response(null, { status: 204 });
    } catch (error) {
        // answer why the renewal was rejected
        const reason = error instanceof Error ? error.message : String(error);

        return new Response(`renewal rejected: ${reason}`, { status: 409 });
    }
}

/** Stream a stored object with its validator, media type and cache policy. */
function respond(object: ServedObject, path: string): Response {
    // stream artifacts without retaining their bytes in Worker memory
    const headers = new Headers({
        etag: object.httpEtag,
        "content-length": String(object.size),
        "access-control-allow-origin": "*",
    });
    headers.set("content-type", object.httpMetadata?.contentType ?? "application/octet-stream");
    headers.set(
        "cache-control",
        path.startsWith("targets/") || /^metadata\/\d+\./u.test(path)
            ? "public, max-age=31536000, immutable"
            : "no-store",
    );

    return new Response(object.body ?? null, {
        headers,
    });
}

/** Read a bounded request containing the exact signed document strings. */
async function readRequest(request: Request): Promise<{ snapshot: string; timestamp: string }> {
    // stop oversized requests before accumulating their complete body
    if (!request.body) {
        throw new Error("missing renewal request");
    }
    const reader = request.body.getReader();
    const buffers: Uint8Array[] = [];
    let size = 0;
    try {
        for (;;) {
            const result = await reader.read();
            if (result.done) {
                break;
            }
            const chunk: unknown = result.value;
            if (!(chunk instanceof Uint8Array)) {
                throw new TypeError("a renewal request chunk is not bytes");
            }
            size += chunk.length;
            if (size > REQUEST_SIZE) {
                await reader.cancel();
                throw new Error("renewal request is too large");
            }
            buffers.push(chunk);
        }
    } finally {
        reader.releaseLock();
    }

    // accept exactly the freshness documents, without arbitrary file names or target contents
    const value: unknown = JSON.parse(Buffer.concat(buffers).toString());
    if (
        typeof value !== "object" ||
        value === null ||
        !("snapshot" in value) ||
        !("timestamp" in value) ||
        typeof value.snapshot !== "string" ||
        typeof value.timestamp !== "string" ||
        Object.keys(value).toSorted().join(",") !== "snapshot,timestamp"
    ) {
        throw new Error("invalid renewal request");
    }

    return { snapshot: value.snapshot, timestamp: value.timestamp };
}

/** The public download and signed-renewal Worker. */
export default { fetch };
