import type { DomainError, ServiceErrorCode, ServiceErrorReport } from "@destack/error";
import { BucketError, type BucketErrorCode } from "../error/index.ts";

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

/** The service error code of each S3 error status. */
const SERVICE_CODES = {
    400: "BAD_REQUEST",
    403: "FORBIDDEN",
    404: "NOT_FOUND",
    405: "METHOD_NOT_SUPPORTED",
    411: "BAD_REQUEST",
    412: "PRECONDITION_FAILED",
    416: "BAD_REQUEST",
    501: "NOT_IMPLEMENTED",
    503: "SERVICE_UNAVAILABLE",
} as const satisfies Readonly<
    Record<(typeof S3_ERROR_STATUS)[keyof typeof S3_ERROR_STATUS], ServiceErrorCode>
>;

/** The S3 error code of each storage failure a request causes; the host reports the rest. */
const STORAGE_ERROR_CODE: Partial<Record<BucketErrorCode, S3ErrorCode>> = {
    INVALID_KEY: "InvalidArgument",
    INVALID_RANGE: "InvalidRange",
    INVALID_CURSOR: "InvalidArgument",
    INVALID_LIMIT: "InvalidArgument",
    INVALID_PART: "InvalidPart",
    INVALID_CHECKSUM: "BadDigest",
    INVALID_STORAGE_CLASS: "InvalidStorageClass",
    INVALID_CUSTOMER_KEY: "InvalidRequest",
    INCOMPLETE_BODY: "IncompleteBody",
    PRECONDITION_FAILED: "PreconditionFailed",
    NO_SUCH_KEY: "NoSuchKey",
    NO_SUCH_UPLOAD: "NoSuchUpload",
    LOCKED: "AccessDenied",
    UNSUPPORTED: "NotImplemented",
    FENCED: "ServiceUnavailable",
};

/** The storage failure each S3 error code answers. */
const STORAGE_FAILURES: ReadonlyMap<S3ErrorCode, BucketErrorCode> = new Map(
    Object.entries(STORAGE_ERROR_CODE).flatMap(([failure, code]) => {
        const answering = Object.values(STORAGE_ERROR_CODE).filter((each) => each === code);

        return answering.length === 1 && BucketError.isCode(failure) ? [[code, failure]] : [];
    }),
);

/** The S3 error code of each status a document-less answer, such as HEAD's, carries. */
const STATUS_ERROR_CODES: Readonly<Record<number, S3ErrorCode>> = {
    400: "BadRequest",
    403: "AccessDenied",
    404: "NoSuchKey",
    405: "MethodNotAllowed",
    412: "PreconditionFailed",
    416: "InvalidRange",
    501: "NotImplemented",
    503: "ServiceUnavailable",
};

/** An S3 error code. */
export type S3ErrorCode = keyof typeof S3_ERROR_STATUS;

/** A failed S3 request, answered with its code and HTTP status. */
export class S3Error extends Error implements DomainError {
    /** The S3 error code. */
    readonly code: S3ErrorCode;

    /** Create a failure with an S3 error code. */
    constructor(code: S3ErrorCode, message: string, options?: ErrorOptions) {
        super(message, options);
        this.name = "S3Error";
        this.code = code;
    }

    /** The HTTP status of the code. */
    get status(): number {
        return S3_ERROR_STATUS[this.code];
    }

    /** Convert the failure to the service error a caller receives, by the status of its code. */
    toServiceError(): ServiceErrorReport {
        return { code: SERVICE_CODES[S3_ERROR_STATUS[this.code]], message: this.message };
    }

    /** Read the S3 error an error document names, refusing a code S3 does not define. */
    static named(code: string, message: string): S3Error {
        if (!isCode(code)) {
            throw new TypeError(`the bucket answered the unknown S3 error ${code}`);
        }

        return new S3Error(code, message);
    }

    /** Read the S3 error a status answers without a document. */
    static answered(status: number, message: string): S3Error {
        const code = STATUS_ERROR_CODES[status];
        if (code === undefined) {
            throw new TypeError(`the bucket answered status ${status} without a document`);
        }

        return new S3Error(code, message);
    }

    /** Read the storage failure the error answers. */
    toBucketError(): BucketError | undefined {
        const failure = STORAGE_FAILURES.get(this.code);

        return failure === undefined ? undefined : new BucketError(failure, this.message);
    }

    /** Read a failure the request caused as an S3 error, and return undefined for host failures. */
    static read(error: unknown): S3Error | undefined {
        // keep S3 errors and translate storage failures the request caused
        if (error instanceof S3Error) {
            return error;
        } else if (!(error instanceof BucketError)) {
            return undefined;
        }

        // translate a storage failure with an S3 code
        const code = STORAGE_ERROR_CODE[error.code];

        return code === undefined ? undefined : new S3Error(code, error.message, { cause: error });
    }
}

/** Report whether a value names an S3 error code. */
function isCode(value: string): value is S3ErrorCode {
    return Object.hasOwn(S3_ERROR_STATUS, value);
}
