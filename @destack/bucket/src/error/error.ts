import { defineErrorFamily, type FamilyError, type ReasonOf } from "@destack/error";
import { schema } from "@destack/schema";
import type { BucketFile } from "../bucket/file.ts";
import { FileDescription } from "../bucket/description.ts";
import type {} from "@destack/package/import-meta";

/** The file a failed precondition found at its key, null when the key holds none. */
export const PreconditionFailure = schema.object({
    /** The current file, null for a missing key. */
    current: FileDescription.nullable(),
});
/** The file a failed precondition found at its key. */
export type PreconditionFailure = schema.Infer<typeof PreconditionFailure>;

/** The bucket package's failures, each reason with the call error code it answers. */
export const BucketError = defineErrorFamily({
    reasons: {
        // not found
        BUCKET_NOT_FOUND: {},
        KEY_NOT_FOUND: {},
        UPLOAD_NOT_FOUND: {},

        // invalid
        ARGUMENT_INVALID: {},
        AUTHORIZATION_HEADER_INVALID: {},
        AUTHORIZATION_QUERY_INVALID: {},
        BODY_INVALID: {},
        CONTENT_INVALID: {},
        CONTENT_SHA256_INVALID: {},
        DIGEST_INVALID: {},
        ENCRYPTION_ALGORITHM_INVALID: {},
        MAX_MESSAGE_LENGTH_INVALID: {},
        METADATA_INVALID: {},
        PART_INVALID: {},
        PART_ORDER_INVALID: {},
        RANGE_INVALID: {},
        REQUEST_BODY_INVALID: {},
        REQUEST_INVALID: {},
        STORAGE_CLASS_INVALID: {},
        TOKEN_INVALID: {},
        URI_INVALID: {},
        XML_INVALID: {},

        // required
        CONTENT_LENGTH_REQUIRED: {},

        // denied
        ACCESS_DENIED: {},
        ACCESS_KEY_DENIED: {},
        CORS_DENIED: {},
        REQUEST_TIME_DENIED: {},
        SIGNATURE_DENIED: {},

        // blocked
        PRECONDITION_BLOCKED: { data: PreconditionFailure },

        // unsupported
        CLONE_UNSUPPORTED: {},
        METHOD_UNSUPPORTED: {},
        OPERATION_UNSUPPORTED: {},

        // unavailable
        BUCKET_UNAVAILABLE: {},

        // unreachable
        S3_UNREACHABLE: {},

        // broken
        WRITE_BROKEN: {},
    },
});

/** A stable cause of bucket failures. */
export type BucketReason = ReasonOf<typeof BucketError>;

/** Refuse a write whose preconditions did not hold, with the file the key holds now. */
export function preconditionFailed(
    current: BucketFile | null,
): FamilyError<"PRECONDITION_BLOCKED"> {
    return BucketError.of(
        "PRECONDITION_BLOCKED",
        "cannot write the file: at least one of the preconditions did not hold",
        {
            data: { current: current === null ? null : current.describe() },
        },
    );
}
