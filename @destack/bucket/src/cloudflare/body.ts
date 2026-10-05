import type * as Cloudflare from "@cloudflare/workers-types";
import type { BucketBody } from "../bucket/index.ts";

/** The bytes an R2 binding takes, with the stream and blob classes the Workers types declare. */
type R2Value = Cloudflare.ReadableStream | ArrayBuffer | ArrayBufferView | string | Cloudflare.Blob;

/** File bytes passed between Destack and an R2 binding, whose runtime shares the web stream and blob classes. */
export const R2Body = { read, write };

/** Read the stream of a file R2 returns as the web stream it is at runtime. */
function read(entry: Cloudflare.R2ObjectBody): ReadableStream<Uint8Array> {
    const body = entry.body;
    if (!(body instanceof ReadableStream)) {
        throw new TypeError("r2 returned a file body outside the web stream class");
    }

    return body;
}

/** Convert a body to the value an R2 binding takes. */
function write(body: BucketBody): R2Value {
    // pass strings and buffers, which both type declarations share
    if (typeof body === "string" || body instanceof ArrayBuffer || ArrayBuffer.isView(body)) {
        return body;
    }
    // pass streams and blobs as the classes the Workers runtime declares
    else if (isWorkersValue(body)) {
        return body;
    }
    // refuse classes from outside the runtime
    else {
        throw new TypeError("the body is a stream or blob from outside the web classes");
    }
}

/** Check that a value is a web stream or blob, which the Workers runtime declares as its own classes. */
function isWorkersValue(value: unknown): value is Cloudflare.ReadableStream | Cloudflare.Blob {
    return value instanceof ReadableStream || value instanceof Blob;
}
