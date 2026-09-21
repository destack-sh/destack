import { defineSchema, schema } from "@destack/schema";
import { PackageError } from "../error/index.ts";

/** A canonical package-relative path using slash separators. */
export const PackagePath = defineSchema(
    schema.string().regex(
        // deno-lint-ignore no-control-regex -- reject control characters in distributed paths
        /^(?!\/)(?![A-Za-z]:)(?!.*\\)(?!.*(?:^|\/)\.{1,2}(?:\/|$))[^/\x00-\x1f\x7f]+(?:\/[^/\x00-\x1f\x7f]+)*$(?![\s\S])/,
    ),
);

/** A SHA-256 digest encoded as lowercase hexadecimal. */
export const Digest = defineSchema(
    schema
        .string()
        .length(64)
        .regex(/^[a-f0-9]{64}$/),
);

/** A source or generated file in a package. */
export const PackageFile = defineSchema(
    schema.object({
        /** The path relative to the source or build root. */
        path: PackagePath,
        /** The SHA-256 digest of the file bytes. */
        digest: Digest,
        /** The file size in bytes. */
        size: schema.number().int().min(0),
        /** The file's media type. */
        mediaType: schema.string().min(1),
    }),
);
/** A source or generated file in a package. */
export type PackageFile = schema.Infer<typeof PackageFile>;

/** Describe a file's path, content, and media type. */
export async function describeFile(
    path: string,
    mediaType: string,
    bytes: Uint8Array<ArrayBuffer>,
): Promise<PackageFile> {
    const digest = await digestFile(bytes);

    return PackageFile.parse({ path, mediaType, size: bytes.byteLength, digest });
}

/** Verify a file's exact size and digest before reading its contents. */
export async function verifyFile(file: PackageFile, bytes: Uint8Array<ArrayBuffer>): Promise<void> {
    if (bytes.byteLength !== file.size) {
        throw new PackageError("INVALID_FILE", `File size mismatch: ${file.path}`);
    }

    // reject any content change, including changes that preserve the length
    const digest = await digestFile(bytes);
    if (digest !== file.digest) {
        throw new PackageError("INVALID_FILE", `File digest mismatch: ${file.path}`);
    }
}

/** Hash file bytes using SHA-256. */
async function digestFile(bytes: Uint8Array<ArrayBuffer>): Promise<string> {
    const digest = new Uint8Array(await crypto.subtle.digest("SHA-256", bytes));

    return Array.from(digest, (byte) => byte.toString(16).padStart(2, "0")).join("");
}
