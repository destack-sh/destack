import type { R2Bucket } from "@cloudflare/workers-types";
import { Metadata, MetadataKind } from "@tufjs/models";
import stable from "../../../../@destack/cli/src/update/root.json" with { type: "json" };
import nightly from "../../../../@destack/cli/src/update/nightly.json" with { type: "json" };
import { Publication } from "./publication.ts";

/** Maximum renewal request size, in bytes. */
const REQUEST_SIZE = 128 * 1024;

/** Release buckets bound by the deployment administrator. */
interface PublicationEnvironment {
    /** Stable releases writable only by approved release jobs and this service. */
    STABLE_RELEASES: R2Bucket;
    /** Nightly releases writable by unattended nightly jobs and this service. */
    NIGHTLY_RELEASES: R2Bucket;
}

/** Serve public downloads and accept only signed freshness renewals. */
async function fetch(request: Request, environment: PublicationEnvironment): Promise<Response> {
    // select one fixed repository without accepting arbitrary bucket names or object paths
    const url = new URL(request.url);
    const pathName = ["/install", "/install.ps1", "/downloads.json"].includes(url.pathname)
        ? `/stable${url.pathname}`
        : url.pathname;
    const match = /^\/(stable|nightly)\/(.+)$/.exec(pathName);
    if (!match) {
        return new Response("not found", { status: 404 });
    }
    const name = match[1];
    const path = match[2];
    const bucket = name === "stable" ? environment.STABLE_RELEASES : environment.NIGHTLY_RELEASES;

    // expose a write endpoint limited to the two authenticated freshness documents
    if (path === "renew" && request.method === "POST") {
        const root = Metadata.fromJSON(MetadataKind.Root, name === "stable" ? stable : nightly);
        const publication = new Publication(bucket, `${name}/`, root);
        try {
            const input = await readRequest(request);
            await publication.renew(Buffer.from(input.snapshot), Buffer.from(input.timestamp));

            return new Response(null, { status: 204 });
        } catch (error) {
            console.error(error);

            return new Response("renewal rejected", { status: 409 });
        }
    }

    // restrict public reads to release metadata, immutable downloads and bootstrap files
    if (request.method !== "GET" && request.method !== "HEAD") {
        return new Response("method not allowed", { status: 405, headers: { allow: "GET, HEAD" } });
    }
    if (
        !/^(metadata\/(?:[1-9]\d*\.(?:root|targets|snapshot)|timestamp)\.json|targets\/[0-9a-f]{64}\.[a-z0-9_-]+\.(?:tar\.gz|dmg|exe)|downloads\.json|install(?:\.ps1)?)$/.test(
            path,
        )
    ) {
        return new Response("not found", { status: 404 });
    }
    const key = `${name}/${path}`;
    const object = request.method === "HEAD" ? await bucket.head(key) : await bucket.get(key);
    if (!object) {
        return new Response("not found", { status: 404 });
    }

    // stream artifacts without retaining their bytes in Worker memory
    const headers = new Headers({
        etag: object.httpEtag,
        "content-length": String(object.size),
        "access-control-allow-origin": "*",
    });
    headers.set("content-type", object.httpMetadata?.contentType ?? "application/octet-stream");
    headers.set(
        "cache-control",
        path.startsWith("targets/") || /^metadata\/\d+\./.test(path)
            ? "public, max-age=31536000, immutable"
            : "no-store",
    );

    return new Response("body" in object ? (object.body as unknown as ReadableStream) : null, {
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
            size += result.value.length;
            if (size > REQUEST_SIZE) {
                await reader.cancel();
                throw new Error("renewal request is too large");
            }
            buffers.push(result.value);
        }
    } finally {
        reader.releaseLock();
    }

    // accept exactly the freshness documents, without arbitrary file names or target contents
    const value = JSON.parse(Buffer.concat(buffers).toString());
    if (
        !value ||
        typeof value.snapshot !== "string" ||
        typeof value.timestamp !== "string" ||
        Object.keys(value).sort().join(",") !== "snapshot,timestamp"
    ) {
        throw new Error("invalid renewal request");
    }

    return value;
}

/** The public download and signed-renewal Worker. */
export default { fetch };
