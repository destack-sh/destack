import { Subject } from "@destack/sync";
import type { BlobStore, DatabaseConnection } from "@destack/db/database";
import type { Environment } from "@destack/environment";
import { ResourceError } from "@destack/resource/error";
import { Digest, Duration, type JsonObject, present, schema } from "@destack/schema";
import { ObjectError } from "../error/index.ts";
import { RequestBody } from "../http/body.ts";
import { Limits } from "../method/limits.ts";
import type { ObjectType } from "../object/object.ts";
import type { RequestContext } from "../server/context.ts";
import type { HmacKey } from "../webhook/hmac.ts";
import { FILE_PATH, ObjectFileInput, ObjectFile, PART_BYTES, ObjectFilePart } from "./file.ts";
import { PICTURE_HEADER_BYTES, pictureSize } from "./picture.ts";

/** How long a part waits to become part of a file: a day, so a person can come back to it. */
const PART_MILLISECONDS = 24 * 60 * 60_000;

/** The name of the limit on the parts a caller sends. */
const PART_LIMIT = "file:part";

/** How many parts a caller sends: 8192 of 8 MiB are 64 GiB an hour, 150 Mbit/s without end. */
const PART_RATE: NonNullable<Limits["rate"]> = { count: 8192, every: { hours: 1 } };

/** The media types a browser shows in a page of its own, which run no scripts. */
const SHOWN =
    /^(?:image\/(?:png|jpeg|gif|webp|avif)|video\/.+|audio\/.+|application\/pdf|text\/plain)$/u;

/** A range of bytes a request asks for, as RFC 9110 writes one. */
const RANGE = /^bytes=(\d*)-(\d*)$/u;

/** Read an object's row through its own `get` as a request's caller. */
export type ReadObject = (
    type: ObjectType,
    method: string,
    input: JsonObject,
    context: RequestContext,
) => Promise<unknown>;

/** What a store's files are served from. */
export interface ObjectFileServerOptions {
    /** The blobs of the files' parts. */
    readonly blobs: BlobStore;
    /** The store's database, which counts the parts callers send. */
    readonly database: DatabaseConnection;
    /** The clock parts expire on. */
    readonly environment: Pick<Environment, "clock">;
    /** Read the key the store signs files with. */
    readonly key: () => Promise<HmacKey>;
    /** The types whose files are served. */
    readonly types: readonly ObjectType[];
    /** Read an object as a request's caller. */
    readonly read: ReadObject;
    /** How many parts a caller sends in a period, 8192 an hour by default. */
    readonly rate?: NonNullable<Limits["rate"]>;
}

/** The HTTP side of a store's files: it takes parts, makes files and answers objects' files. */
export class ObjectFileServer {
    /** What the files are served from. */
    readonly #options: ObjectFileServerOptions;
    /** The served types, by their keys. */
    readonly #types: ReadonlyMap<string, ObjectType>;

    /** Serve a store's files of some types. */
    constructor(options: ObjectFileServerOptions) {
        this.#options = options;
        this.#types = new Map(options.types.map((type) => [type.key, type]));
    }

    /** Store a part a caller sends, refusing bytes of another digest, and sign it. */
    async send(
        caller: string,
        digest: Digest,
        body: AsyncIterable<Uint8Array>,
    ): Promise<ObjectFilePart> {
        // refuse a caller past its rate, saying when the next part is due
        const { blobs, database, environment, key, rate = PART_RATE } = this.#options;
        const now = environment.clock.now();
        const retryAfter = await Limits.take(database, PART_LIMIT, caller, rate, now);
        if (retryAfter !== undefined) {
            throw ObjectError.of(
                "RATE_EXHAUSTED",
                `caller '${caller}' exceeds its limit of ${rate.count} parts per ${Duration.milliseconds(rate.every)} ms`,
                { data: { retryAfter } },
            );
        }

        // store it, counting its bytes
        const counted = { size: 0 };
        try {
            await blobs.write(limited(body, counted), digest);
        } catch (error) {
            // answer other bytes as the caller's own failure
            throw ResourceError.is(error, "DIGEST_INVALID")
                ? ObjectError.of("PART_INVALID", "part is invalid: its bytes have another digest")
                : error;
        }

        // refuse an empty part, which no file has
        const { size } = counted;
        if (size === 0) {
            throw ObjectError.of("PART_INVALID", "part is invalid: it is empty");
        }

        // keep it while the caller may come back
        const expiresAt = now + PART_MILLISECONDS;
        await blobs.lease([digest], expiresAt);

        return ObjectFilePart.sign({ digest, size, expiresAt }, caller, await key());
    }

    /** Make a file of the parts a caller sent, in their order. */
    async complete(caller: string, input: ObjectFileInput): Promise<ObjectFile> {
        // require the caller to have sent each part, and each but the last to be whole
        const { blobs, environment } = this.#options;
        const key = await this.#options.key();
        const now = environment.clock.now();
        for (const [place, part] of input.parts.entries()) {
            if (!(await ObjectFilePart.verify(part, caller, key))) {
                throw ObjectError.of("PART_INVALID", "part is invalid: the caller did not send it");
            } else if (part.size !== PART_BYTES && place !== input.parts.length - 1) {
                throw ObjectError.of(
                    "PART_INVALID",
                    "part is invalid: only the last part of a file is shorter",
                );
            } else if (part.expiresAt <= now) {
                throw ObjectError.of(
                    "PART_INVALID",
                    "part is invalid: it expired, so send it again",
                );
            }
        }

        // keep the parts for a row to reference, refusing one a collection deleted
        const parts = input.parts.map(({ digest }) => digest);
        const stored = await blobs.lease(parts, now + PART_MILLISECONDS);
        if (parts.some((part) => !stored.has(part))) {
            throw ObjectError.of("PART_INVALID", "part is invalid: it is gone, so send it again");
        }

        // read a picture's size from its first bytes
        const { name, type } = input;
        const size = input.parts.reduce((total, part) => total + part.size, 0);
        const [first] = parts;
        const picture =
            first !== undefined && type.startsWith("image/")
                ? pictureSize(await bytesOf(blobs.slice(first, 0, PICTURE_HEADER_BYTES)))
                : undefined;

        return ObjectFile.sign({ parts, size, name, type, picture: picture ?? null }, key);
    }

    /** Read a range of a file's bytes, its parts joined in order. */
    async *read(
        file: ObjectFile,
        { offset, length }: { readonly offset: number; readonly length: number } = {
            offset: 0,
            length: file.size,
        },
    ): AsyncIterable<Uint8Array> {
        // read what the range has of each part
        const end = Math.min(offset + length, file.size);
        for (let start = offset; start < end;) {
            const place = Math.floor(start / PART_BYTES);
            const part = present(file.parts[place], "the part of a file's range");
            const within = start - place * PART_BYTES;
            const count = Math.min(PART_BYTES - within, end - start);
            yield* this.#options.blobs.slice(part, within, count);
            start += count;
        }
    }

    /** Serve a request for a file, or return undefined for another. */
    async fetch(request: Request, context: RequestContext): Promise<Response | undefined> {
        // leave other paths
        const { pathname } = new URL(request.url);
        if (pathname !== FILE_PATH && !pathname.startsWith(`${FILE_PATH}/`)) {
            return undefined;
        }
        const steps = stepsOf(pathname.slice(FILE_PATH.length));

        // take a part, make a file, or answer one
        if (request.method === "PUT" && steps.length === 1) {
            return this.#send(request, context, steps);
        } else if (request.method === "POST" && steps.length === 0) {
            return this.#complete(request, context);
        } else if (["GET", "HEAD"].includes(request.method) && steps.length === 4) {
            return this.#open(request, context, steps);
        }
        throw ObjectError.of(
            "ROUTE_NOT_FOUND",
            `route '${request.method}' '${pathname}' not found`,
        );
    }

    /** Take a part of a file from a signed-in caller, whose write decides where it may go. */
    async #send(
        request: Request,
        context: RequestContext,
        [named = ""]: readonly string[],
    ): Promise<Response> {
        // refuse a path that is no digest
        const digest = Digest.safeParse(named);
        if (!digest.success) {
            throw ObjectError.of("PART_INVALID", `part '${named}' is invalid: it is no digest`);
        }

        // take its bytes
        const body: AsyncIterable<Uint8Array> = request.body ?? new Blob([]).stream();

        return Response.json(await this.send(ObjectFileServer.#caller(context), digest.data, body));
    }

    /** Make a file of the parts a signed-in caller sent. */
    async #complete(request: Request, context: RequestContext): Promise<Response> {
        // refuse a body that is no file
        const input = ObjectFileInput.safeParse(await RequestBody.json(request, "a file"));
        if (!input.success) {
            throw ObjectError.of("FILE_INVALID", "file is invalid: it must match its schema", {
                cause: input.error,
            });
        }

        return Response.json(await this.complete(ObjectFileServer.#caller(context), input.data));
    }

    /** Answer an object's file, or a range of it, to a caller who may read its field. */
    async #open(
        request: Request,
        context: RequestContext,
        [scope = "", key = "", id = "", name = ""]: readonly string[],
    ): Promise<Response> {
        // read the object as the caller, through its type's own method
        const type = this.#types.get(key);
        const field = type?.fields[name];
        const method = Object.entries(type?.methods ?? {}).find(
            ([, each]) => each.source?.builder === "get",
        )?.[0];
        if (type === undefined || field === undefined || method === undefined) {
            throw ObjectFileServer.#missing(key, id, name);
        }
        const route = type.route.field;
        const input = { ...(route === undefined ? {} : { [route]: scope }), id };
        const row = await this.#options.read(type, method, input, context);
        const found = field.file(schema.looseObject({}).parse(row)[name]);
        if (found === undefined) {
            throw ObjectFileServer.#missing(key, id, name);
        }

        return this.#answer(request, found.file);
    }

    /** Answer a file's bytes, those a range asks for, or that a caller has them. */
    #answer(request: Request, file: ObjectFile): Response {
        // describe it, shown where it runs nothing and downloaded elsewhere
        const headers = new Headers({
            "accept-ranges": "bytes",
            "cache-control": "private, no-cache",
            "content-disposition": `${SHOWN.test(file.type) ? "inline" : "attachment"}; filename*=UTF-8''${encodeName(file.name)}`,
            "content-security-policy": "default-src 'none'; sandbox",
            "content-type": file.type,
            etag: `"${file.signature}"`,
            "x-content-type-options": "nosniff",
        });
        const held = request.headers.get("if-none-match")?.split(",") ?? [];
        if (held.some((tag) => tag.trim().replace(/^W\//u, "") === headers.get("etag"))) {
            return new Response(null, { status: 304, headers });
        }

        // answer the range, refusing one past the end
        const range = ObjectFileServer.#range(request.headers.get("range"), file.size);
        if (range === "outside") {
            headers.set("content-range", `bytes */${String(file.size)}`);

            return new Response(null, { status: 416, headers });
        }
        const { offset, length } = range ?? { offset: 0, length: file.size };
        headers.set("content-length", String(length));
        if (range !== undefined) {
            const last = String(offset + length - 1);
            headers.set("content-range", `bytes ${String(offset)}-${last}/${String(file.size)}`);
        }
        const body =
            request.method === "HEAD" ? null : streamOf(this.read(file, { offset, length }));

        return new Response(body, { status: range === undefined ? 200 : 206, headers });
    }

    /** Read the range a header asks for, absent for none or several. */
    static #range(
        header: string | null,
        size: number,
    ): { readonly offset: number; readonly length: number } | "outside" | undefined {
        // read its first and last bytes, or how many at the end
        const match = RANGE.exec(header ?? "");
        const [, first = "", last = ""] = match ?? [];
        if (match === null || (first === "" && last === "")) {
            return undefined;
        }
        const offset = first === "" ? Math.max(0, size - Number(last)) : Number(first);
        const end = first === "" || last === "" ? size : Math.min(size, Number(last) + 1);

        return offset < end ? { offset, length: end - offset } : "outside";
    }

    /** Read the key of a request's caller, refusing an anonymous one. */
    static #caller(context: RequestContext): string {
        // require a credential, whose caller the parts are signed for
        context.credential();
        const { subject } = context.caller();
        if (subject === undefined) {
            throw ObjectError.of(
                "CREDENTIAL_UNAUTHENTICATED",
                "authentication required: caller credential is missing",
            );
        }

        return Subject.key(subject);
    }

    /** Describe a file that is not there, or is not the caller's to read. */
    static #missing(key: string, id: string, name: string): Error {
        return ObjectError.of("OBJECT_NOT_FOUND", `file '${name}' of ${key} '${id}' not found`);
    }
}

/** Join chunks into one array of bytes. */
async function bytesOf(chunks: AsyncIterable<Uint8Array>): Promise<Uint8Array<ArrayBuffer>> {
    const copied = await Array.fromAsync(chunks, (chunk) => new Uint8Array(chunk));

    return new Blob(copied).bytes();
}

/** Pass on a part's bytes, counting them, and refuse more than a part has. */
async function* limited(
    body: AsyncIterable<Uint8Array>,
    counted: { size: number },
): AsyncIterable<Uint8Array> {
    for await (const chunk of body) {
        counted.size += chunk.byteLength;
        if (counted.size > PART_BYTES) {
            throw ObjectError.of(
                "PART_INVALID",
                `part is invalid: it is longer than ${String(PART_BYTES)} bytes`,
            );
        }
        yield chunk;
    }
}

/** Read the steps of a path, refusing one that does not decode. */
function stepsOf(path: string): string[] {
    try {
        return path.split("/").slice(1).map(decodeURIComponent);
    } catch (error) {
        throw ObjectError.of("FILE_INVALID", `file '${path}' is invalid: it does not decode`, {
            cause: error,
        });
    }
}

/** Encode a file name for a header, with the marks a URI leaves and RFC 8187 does not. */
function encodeName(name: string): string {
    return encodeURIComponent(name).replace(
        /['()*]/gu,
        (mark) => `%${mark.charCodeAt(0).toString(16).toUpperCase()}`,
    );
}

/** Read an iterable of bytes as a stream, ending it as the reader leaves. */
function streamOf(bytes: AsyncIterable<Uint8Array>): ReadableStream<Uint8Array> {
    const iterator = bytes[Symbol.asyncIterator]();

    return new ReadableStream({
        async pull(controller) {
            // pass on the next chunk, or end
            const next = await iterator.next();
            if (next.done === true) {
                controller.close();
            } else {
                controller.enqueue(next.value);
            }
        },
        async cancel() {
            await iterator.return?.();
        },
    });
}
