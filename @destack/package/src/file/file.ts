import { defineSchema, Digest, schema } from "@destack/schema";
import { PackageError } from "../error/index.ts";

/** A canonical package-relative path using slash separators. */
export const PackagePath = defineSchema(
    schema
        .string()
        .regex(
            /^(?!\/)(?![A-Za-z]:)(?!.*\\)(?!.*(?:^|\/)\.{1,2}(?:\/|$))[^/\x00-\x1f\x7f]+(?:\/[^/\x00-\x1f\x7f]+)*$(?![\s\S])/,
        ),
);

/** The schema of a source or generated file in a package. */
const packageFileSchema = defineSchema(
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
export type PackageFile = schema.Infer<typeof packageFileSchema>;

/** A source or generated file in a package. */
export const PackageFile = Object.assign(packageFileSchema, { describe, verify });

/** Describe a file's path, content, and media type. */
async function describe(
    path: string,
    mediaType: string,
    bytes: Uint8Array<ArrayBuffer>,
): Promise<PackageFile> {
    const digest = await Digest.of(bytes);

    return packageFileSchema.parse({ path, mediaType, size: bytes.byteLength, digest });
}

/** Verify a file's exact size and digest before reading its contents. */
async function verify(file: PackageFile, bytes: Uint8Array<ArrayBuffer>): Promise<void> {
    if (bytes.byteLength !== file.size) {
        throw new PackageError("INVALID_FILE", `file size mismatch: ${file.path}`);
    }

    // reject any content change, including changes that preserve the length
    const digest = await Digest.of(bytes);
    if (digest !== file.digest) {
        throw new PackageError("INVALID_FILE", `file digest mismatch: ${file.path}`);
    }
}
