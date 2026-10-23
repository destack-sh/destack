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
export const BucketError = defineErrorFamily(import.meta.destack.package, {
    // bad request
    AUTHORIZATION_HEADER_MALFORMED: { code: "BAD_REQUEST" },
    BAD_DIGEST: { code: "BAD_REQUEST" },
    CONTENT_SHA256_MISMATCH: { code: "BAD_REQUEST" },
    INCOMPLETE_BODY: { code: "BAD_REQUEST" },
    INVALID_ARGUMENT: { code: "BAD_REQUEST" },
    INVALID_AUTHORIZATION_QUERY: { code: "BAD_REQUEST" },
    INVALID_DIGEST: { code: "BAD_REQUEST" },
    INVALID_ENCRYPTION_ALGORITHM: { code: "BAD_REQUEST" },
    INVALID_PART: { code: "BAD_REQUEST" },
    INVALID_PART_ORDER: { code: "BAD_REQUEST" },
    INVALID_RANGE: { code: "BAD_REQUEST" },
    INVALID_REQUEST: { code: "BAD_REQUEST" },
    INVALID_STORAGE_CLASS: { code: "BAD_REQUEST" },
    INVALID_TOKEN: { code: "BAD_REQUEST" },
    INVALID_URI: { code: "BAD_REQUEST" },
    MALFORMED_REQUEST: { code: "BAD_REQUEST" },
    MALFORMED_XML: { code: "BAD_REQUEST" },
    MAX_MESSAGE_LENGTH_EXCEEDED: { code: "BAD_REQUEST" },
    METADATA_TOO_LARGE: { code: "BAD_REQUEST" },
    MISSING_CONTENT_LENGTH: { code: "BAD_REQUEST" },

    // forbidden
    ACCESS_DENIED: { code: "FORBIDDEN" },
    ACCESS_FORBIDDEN: { code: "FORBIDDEN" },
    INVALID_ACCESS_KEY_ID: { code: "FORBIDDEN" },
    REQUEST_TIME_TOO_SKEWED: { code: "FORBIDDEN" },
    SIGNATURE_DOES_NOT_MATCH: { code: "FORBIDDEN" },

    // not found
    NO_SUCH_BUCKET: { code: "NOT_FOUND" },
    NO_SUCH_KEY: { code: "NOT_FOUND" },
    NO_SUCH_UPLOAD: { code: "NOT_FOUND" },

    // method not supported
    METHOD_NOT_ALLOWED: { code: "METHOD_NOT_SUPPORTED" },

    // precondition failed
    PRECONDITION_UNMET: { code: "PRECONDITION_FAILED", data: PreconditionFailure },

    // unprocessable content
    FILE_NOT_CLONABLE: { code: "UNPROCESSABLE_CONTENT" },

    // internal server error
    WRITE_FAILED: { code: "INTERNAL_SERVER_ERROR" },

    // not implemented
    UNSUPPORTED: { code: "NOT_IMPLEMENTED" },

    // bad gateway
    UNEXPECTED_STATUS: { code: "BAD_GATEWAY" },
    UNKNOWN_S3_CODE: { code: "BAD_GATEWAY" },

    // service unavailable
    BUSY: { code: "SERVICE_UNAVAILABLE" },
    CLOSED: { code: "SERVICE_UNAVAILABLE" },
    UNAVAILABLE: { code: "SERVICE_UNAVAILABLE" },
});

/** A stable cause of bucket failures. */
export type BucketReason = ReasonOf<typeof BucketError>;

/** Refuse a write whose preconditions did not hold, with the file the key holds now. */
export function preconditionFailed(current: BucketFile | null): FamilyError<"PRECONDITION_UNMET"> {
    return BucketError.of("PRECONDITION_UNMET", "at least one of the preconditions did not hold", {
        data: { current: current === null ? null : current.describe() },
    });
}
