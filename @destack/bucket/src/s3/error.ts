import type { FamilyError } from "@destack/error";
import { BucketError, type BucketReason } from "../error/index.ts";

/** The HTTP status of each S3 error code, as the S3 API reference lists them. */
const S3_ERROR_STATUS = {
    AccessDenied: 403,
    AccessForbidden: 403,
    AuthorizationHeaderMalformed: 400,
    AuthorizationQueryParametersError: 400,
    BadDigest: 400,
    BadRequest: 400,
    IncompleteBody: 400,
    InvalidAccessKeyId: 403,
    InvalidArgument: 400,
    InvalidDigest: 400,
    InvalidEncryptionAlgorithmError: 400,
    InvalidPart: 400,
    InvalidPartOrder: 400,
    InvalidRange: 416,
    InvalidRequest: 400,
    InvalidStorageClass: 400,
    InvalidToken: 400,
    InvalidURI: 400,
    MalformedXML: 400,
    MaxMessageLengthExceeded: 400,
    MetadataTooLarge: 400,
    MethodNotAllowed: 405,
    MissingContentLength: 411,
    NoSuchBucket: 404,
    NoSuchKey: 404,
    NoSuchUpload: 404,
    NotImplemented: 501,
    PreconditionFailed: 412,
    RequestTimeTooSkewed: 403,
    ServiceUnavailable: 503,
    SignatureDoesNotMatch: 403,
    XAmzContentSHA256Mismatch: 400,
} as const;

/** The S3 error code of each bucket failure S3 has a code for, one failure per code. */
const S3_ERROR_CODE: Partial<Record<BucketReason, S3ErrorCode>> = {
    ACCESS_DENIED: "AccessDenied",
    CORS_DENIED: "AccessForbidden",
    AUTHORIZATION_HEADER_INVALID: "AuthorizationHeaderMalformed",
    CONTENT_INVALID: "BadDigest",
    CONTENT_SHA256_INVALID: "XAmzContentSHA256Mismatch",
    BODY_INVALID: "IncompleteBody",
    ACCESS_KEY_DENIED: "InvalidAccessKeyId",
    ARGUMENT_INVALID: "InvalidArgument",
    AUTHORIZATION_QUERY_INVALID: "AuthorizationQueryParametersError",
    DIGEST_INVALID: "InvalidDigest",
    ENCRYPTION_ALGORITHM_INVALID: "InvalidEncryptionAlgorithmError",
    PART_INVALID: "InvalidPart",
    PART_ORDER_INVALID: "InvalidPartOrder",
    RANGE_INVALID: "InvalidRange",
    REQUEST_INVALID: "InvalidRequest",
    STORAGE_CLASS_INVALID: "InvalidStorageClass",
    TOKEN_INVALID: "InvalidToken",
    URI_INVALID: "InvalidURI",
    REQUEST_BODY_INVALID: "BadRequest",
    XML_INVALID: "MalformedXML",
    MAX_MESSAGE_LENGTH_INVALID: "MaxMessageLengthExceeded",
    METADATA_INVALID: "MetadataTooLarge",
    METHOD_UNSUPPORTED: "MethodNotAllowed",
    CONTENT_LENGTH_REQUIRED: "MissingContentLength",
    BUCKET_NOT_FOUND: "NoSuchBucket",
    KEY_NOT_FOUND: "NoSuchKey",
    UPLOAD_NOT_FOUND: "NoSuchUpload",
    PRECONDITION_BLOCKED: "PreconditionFailed",
    REQUEST_TIME_DENIED: "RequestTimeTooSkewed",
    SIGNATURE_DENIED: "SignatureDoesNotMatch",
    BUCKET_UNAVAILABLE: "ServiceUnavailable",
    OPERATION_UNSUPPORTED: "NotImplemented",
};

/** The failure each S3 error code reads back as, all but a failed precondition. */
const S3_FAILURES: ReadonlyMap<string, ReadReason> = new Map(
    Object.entries(S3_ERROR_CODE).flatMap(([reason, code]) =>
        isReadReason(reason) ? [[code, reason] as const] : [],
    ),
);

/** The S3 error code of each status of a document-less answer, such as HEAD's. */
const STATUS_ERROR_CODES: Readonly<Record<number, S3ErrorCode>> = {
    400: "BadRequest",
    403: "AccessDenied",
    404: "NoSuchKey",
    405: "MethodNotAllowed",
    416: "InvalidRange",
    501: "NotImplemented",
    503: "ServiceUnavailable",
};

/** The reasons S3 error documents read back as, all but a failed precondition. */
type ReadReason = Exclude<BucketReason, "PRECONDITION_BLOCKED">;

/** An S3 error code. */
export type S3ErrorCode = keyof typeof S3_ERROR_STATUS;

/** An S3 error answer: its code, HTTP status and message. */
export interface S3Answer {
    /** The S3 error code. */
    readonly code: S3ErrorCode;
    /** The HTTP status of the code. */
    readonly status: number;
    /** The failure's message. */
    readonly message: string;
}

/** S3 error answers: written for bucket failures, and read back as them. */
export const S3Answer = {
    /** Write the S3 answer to a failure the request caused, absent for host failures. */
    write(error: unknown): S3Answer | undefined {
        // answer the bucket failures S3 has a code for
        if (!BucketError.is(error)) {
            return undefined;
        }
        const code = S3_ERROR_CODE[error.reason];

        return code === undefined
            ? undefined
            : { code, status: S3_ERROR_STATUS[code], message: error.message };
    },

    /** Read the failure an S3 error document answers, a gateway failure for an unknown code. */
    read(code: string, message: string): FamilyError<ReadReason> {
        const reason = S3_FAILURES.get(code);

        return reason === undefined
            ? BucketError.of(
                  "S3_UNREACHABLE",
                  `could not map S3 error '${code}' from the bucket: ${message}`,
              )
            : BucketError.of(reason, message);
    },

    /** Read the failure a status answers without a document, a gateway failure for others. */
    readStatus(status: number, message: string): FamilyError<ReadReason> {
        const code = STATUS_ERROR_CODES[status];

        return code === undefined
            ? BucketError.of(
                  "S3_UNREACHABLE",
                  `could not read a document from the bucket's status ${status} answer`,
              )
            : S3Answer.read(code, message);
    },
};

/** Report whether a value is the reason of a failure an S3 error document reads back as. */
function isReadReason(value: string): value is ReadReason {
    return Object.hasOwn(BucketError.reasons, value) && value !== "PRECONDITION_BLOCKED";
}
