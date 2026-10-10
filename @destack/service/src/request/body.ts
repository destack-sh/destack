import { ServiceError } from "../error/index.ts";

/** The most bytes a request body carries when a reader sets no limit: 1 MiB, the OTLP/HTTP collectors' default. */
export const BODY_BYTES = 1024 * 1024;

/** What a body is read from: a request's headers and body stream, whichever runtime typed it. */
type BodySource = Pick<Request, "body"> & { readonly headers: Pick<Headers, "get"> };

/** A request's body read within a size limit, refusing a larger one before and while reading it. */
export const RequestBody = {
    /** Read a body's bytes within a limit, stating what it carries in a refusal. */
    async bytes(
        request: BodySource,
        name: string,
        limit: number = BODY_BYTES,
    ): Promise<Uint8Array<ArrayBuffer>> {
        // refuse a body its declared length shows too large before reading it
        const tooLarge = new ServiceError("PAYLOAD_TOO_LARGE", {
            message: `${name} carries at most ${limit} bytes`,
        });
        if (Number(request.headers.get("content-length") ?? 0) > limit) {
            throw tooLarge;
        }

        // read the body up to the limit
        const chunks: Uint8Array[] = [];
        let size = 0;
        for await (const chunk of request.body ?? new ReadableStream<Uint8Array>()) {
            size += chunk.byteLength;
            if (size > limit) {
                throw tooLarge;
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
    },

    /** Read a body's text within a limit. */
    async text(request: BodySource, name: string, limit: number = BODY_BYTES): Promise<string> {
        return new TextDecoder().decode(await RequestBody.bytes(request, name, limit));
    },

    /** Read a body as JSON within a limit, refusing a body that is no JSON. */
    async json(request: BodySource, name: string, limit: number = BODY_BYTES): Promise<unknown> {
        const text = await RequestBody.text(request, name, limit);
        try {
            return JSON.parse(text);
        } catch (error) {
            throw new ServiceError("BAD_REQUEST", { message: `${name} is no JSON`, cause: error });
        }
    },
};
