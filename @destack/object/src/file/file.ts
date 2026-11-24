import { canonicalize, Digest, type JsonValue, schema } from "@destack/schema";
import { TABLE } from "@destack/db";
import type { Transaction } from "@destack/db/database";
import { ObjectError } from "../error/index.ts";
import type { ObjectStore } from "../server/store.ts";
import type { HmacKey } from "../webhook/hmac.ts";
import { PictureSize } from "./picture.ts";

/** The bytes of each part of a file but the last: 8 MiB, 6 seconds at 10 Mbit/s. */
export const PART_BYTES = 8 * 1024 * 1024;

/** Where a store serves files: parts by digest, and objects' files by their field. */
export const FILE_PATH = "/store/file";

/** The longest file name, as file systems store it. */
const MAX_NAME_LENGTH = 255;

/** The longest media type, as RFC 6838 limits its two names. */
const MAX_TYPE_LENGTH = 255;

/** A name that is no path, without control characters and lone surrogates. */
const NAME = /^(?!\.{1,2}$)[^/\\\p{Cc}\p{Cs}]+$/u;

/** A media type with its parameters, in printable characters (RFC 9110 8.3.1). */
const MEDIA_TYPE = /^[\w!#$&^.+-]+\/[\w!#$&^.+-]+(?:\s*;[\x20-\x7e]*)?$/u;

/** A media type, or a range of every subtype. */
const MEDIA_RANGE = /^[a-z0-9.+-]+\/(?:\*|[a-z0-9.+-]+)$/u;

/** What a file's signature is over, beside its values. */
const FILE_LABEL = "@destack/object/file";

/** What a part's signature is over, beside its values and its caller. */
const PART_LABEL = "@destack/object/part";

/** The store whose rows have files, with the key it signs them with. */
export interface ObjectFileStore extends Pick<ObjectStore, "objects" | "blobs" | "principal"> {
    /** Read the key the store signs files with. */
    readonly fileKey: () => Promise<HmacKey>;
}

/** A file a field has: its parts in order, and what its reader learns without reading them. */
const objectFile = schema.object({
    /** The blobs of its parts, each `PART_BYTES` long but the last. */
    parts: schema.array(Digest),
    /** Its size in bytes. */
    size: schema.number().int().nonnegative(),
    /** Its name, such as `lisbon.jpg`. */
    name: schema.string().max(MAX_NAME_LENGTH).regex(NAME),
    /** Its media type, such as `image/jpeg`. */
    type: schema.string().max(MAX_TYPE_LENGTH).regex(MEDIA_TYPE),
    /** Its size in pixels, null for another file than a picture. */
    picture: PictureSize.nullable(),
    /** The signature of the store whose bytes it has, over everything else. */
    signature: schema.base64(),
});
/** A file a field has: its parts in order, and what its reader learns without reading them. */
export type ObjectFile = schema.Infer<typeof objectFile>;
/** A file a field has: its parts in order, and what its reader learns without reading them. */
export const ObjectFile = Object.assign(objectFile, {
    /** Sign a file with a store's key. */
    async sign(file: Omit<ObjectFile, "signature">, key: HmacKey): Promise<ObjectFile> {
        const signature = await key.sign(contentOf(FILE_LABEL, file));

        return { ...file, signature: signature.toBase64() };
    },

    /** Report whether a store's key signed a file as it is. */
    verify({ signature, ...file }: ObjectFile, key: HmacKey): Promise<boolean> {
        return key.verify(Uint8Array.fromBase64(signature), contentOf(FILE_LABEL, file));
    },

    /** Enlist a store's written files in their transaction's commit, which requires them. */
    enlist(transaction: Transaction, store: ObjectFileStore): void {
        transaction.enlist(`destack.object.file:${store.principal.id}`, {
            prepare: () => ObjectFile.written(transaction, store),
            confirm: async () => {},
            cancel: async () => {},
        });
    },

    /** Require each file a transaction writes, with its bytes, before it commits. */
    async written(transaction: Transaction, store: ObjectFileStore): Promise<void> {
        // find the file of each field whose row had another, none for most writes
        const typed = new Map(store.objects.map((type) => [type.table[TABLE].sqlName, type]));
        const written = transaction.changes.changes.flatMap(({ table, before, after }) =>
            Object.entries(typed.get(table[TABLE].sqlName)?.fields ?? {}).flatMap(
                ([name, field]) => {
                    // leave the file the row had
                    const had = field.file(before?.[name]);
                    const found = field.file(after?.[name]);
                    const isSame =
                        had !== undefined &&
                        canonicalize(had.file) === canonicalize(found?.file ?? null);

                    return found === undefined || isSame ? [] : [{ ...found, name }];
                },
            ),
        );
        if (written.length === 0) {
            return;
        }

        // require the store made each, within its field's limits
        const key = await store.fileKey();
        for (const { file, limits, name } of written) {
            await ObjectFile.require(file, limits, name, key);
        }

        // keep their bytes for the rows to reference, refusing a file whose own a collection deleted
        const stored = await store.blobs.lease(written.flatMap(({ file }) => file.parts));
        const gone = written.find(({ file }) => file.parts.some((part) => !stored.has(part)));
        if (gone !== undefined) {
            throw ObjectError.of(
                "FILE_INVALID",
                `file '${gone.file.name}' is invalid: its bytes are gone, so send it again`,
            );
        }
    },

    /** Refuse a file a store did not sign, or outside its field's limits. */
    async require(
        file: ObjectFile,
        limits: ObjectFileLimits,
        field: string,
        key: HmacKey,
    ): Promise<void> {
        // refuse one the store did not sign as it is
        if (!(await ObjectFile.verify(file, key))) {
            throw ObjectError.of(
                "FILE_INVALID",
                `file '${file.name}' is invalid: the store did not make it`,
            );
        }
        // refuse another media type
        else if (!isAccepted(file.type, limits)) {
            throw ObjectError.of(
                "FILE_INVALID",
                `file '${file.name}' is invalid: '${field}' takes no file of type ${file.type}`,
            );
        }
        // refuse a larger one
        else if (limits.size !== undefined && file.size > limits.size) {
            throw ObjectError.of(
                "FILE_INVALID",
                `file '${file.name}' is invalid: '${field}' takes at most ${limits.size} bytes`,
            );
        }
    },
});

/** A part a caller sent, signed by its store, which makes a file with the others. */
const objectFilePart = schema.object({
    /** The digest of its blob. */
    digest: Digest,
    /** Its size in bytes. */
    size: schema.number().int().positive().max(PART_BYTES),
    /** When the store forgets it unless a file has it, in UTC milliseconds. */
    expiresAt: schema.number().int(),
    /** The store's signature over the rest and who sent the part. */
    signature: schema.base64(),
});
/** A part a caller sent, signed by its store, which makes a file with the others. */
export type ObjectFilePart = schema.Infer<typeof objectFilePart>;
/** A part a caller sent, signed by its store, which makes a file with the others. */
export const ObjectFilePart = Object.assign(objectFilePart, {
    /** Sign a part for the caller who sent it, with a store's key. */
    async sign(
        part: Omit<ObjectFilePart, "signature">,
        caller: string,
        key: HmacKey,
    ): Promise<ObjectFilePart> {
        const signature = await key.sign(contentOf(PART_LABEL, { ...part, caller }));

        return { ...part, signature: signature.toBase64() };
    },

    /** Report whether a store's key signed a part for a caller. */
    verify({ signature, ...part }: ObjectFilePart, caller: string, key: HmacKey): Promise<boolean> {
        const content = contentOf(PART_LABEL, { ...part, caller });

        return key.verify(Uint8Array.fromBase64(signature), content);
    },
});

/** What a caller says of a file whose parts it sent. */
export const ObjectFileInput = schema.object({
    /** Its parts, in order. */
    parts: schema.array(ObjectFilePart),
    /** Its name. */
    name: ObjectFile.shape.name,
    /** Its media type. */
    type: ObjectFile.shape.type,
});
/** What a caller says of a file whose parts it sent. */
export type ObjectFileInput = schema.Infer<typeof ObjectFileInput>;

/** The media types and sizes a file field takes. */
export const ObjectFileLimits = schema.object({
    /** The media types, such as `image/*` or `application/pdf`, every type when absent. */
    accept: schema.array(schema.string().regex(MEDIA_RANGE)).min(1).exactOptional(),
    /** The most bytes of a file, the space's own limit alone when absent. */
    size: schema.number().int().positive().exactOptional(),
});
/** The media types and sizes a file field takes. */
export type ObjectFileLimits = schema.Infer<typeof ObjectFileLimits>;

/** Write what a signature is over: what it is for, and a value's canonical form. */
function contentOf(label: string, value: JsonValue): Uint8Array<ArrayBuffer> {
    return new TextEncoder().encode(`${label}\n${canonicalize(value)}`);
}

/** Report whether a media type is one of those a file field accepts. */
export function isAccepted(type: string, { accept }: ObjectFileLimits): boolean {
    // match a type alone, or every subtype of a range
    const [essence = ""] = type.toLowerCase().split(";");

    return (
        accept === undefined ||
        accept.some((range) =>
            range.endsWith("/*")
                ? essence.startsWith(range.slice(0, -1))
                : essence.trim() === range,
        )
    );
}
