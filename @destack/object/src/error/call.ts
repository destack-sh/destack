import {
    fallbackORPCErrorStatus,
    ORPCError,
    type ORPCErrorCode,
    type ORPCErrorJSON,
    type ORPCErrorOptions,
} from "@orpc/client";
import { type MaybeOptionalOptions, resolveMaybeOptionalOptions } from "@orpc/shared";

import { ValidationError, type SchemaIssue } from "@orpc/contract";
import { JsonValue, schema } from "@destack/schema";
import {
    DomainError,
    CALL_ERROR_STATUSES,
    Failure,
    type CallErrorCode,
    FailureReason,
    HTTP_DOMAIN,
} from "@destack/error";
import type {} from "@destack/package/import-meta";

/** The failures every procedure declares with their statuses and data. */
export const DECLARED_ERRORS = {
    NOT_IMPLEMENTED: { status: CALL_ERROR_STATUSES.NOT_IMPLEMENTED },
    UNAUTHORIZED: { status: CALL_ERROR_STATUSES.UNAUTHORIZED },
    PAYMENT_REQUIRED: { status: CALL_ERROR_STATUSES.PAYMENT_REQUIRED },
    FORBIDDEN: { status: CALL_ERROR_STATUSES.FORBIDDEN },
    NOT_FOUND: { status: CALL_ERROR_STATUSES.NOT_FOUND },
    CONFLICT: { status: CALL_ERROR_STATUSES.CONFLICT },
    PRECONDITION_FAILED: { status: CALL_ERROR_STATUSES.PRECONDITION_FAILED },
    MISDIRECTED_REQUEST: {
        status: CALL_ERROR_STATUSES.MISDIRECTED_REQUEST,
        // where the call's scope is served: another machine, or another partition of the installation
        data: schema.union([
            schema.object({ scope: schema.string().min(1), machine: schema.identifier("machine") }),
            schema.object({ scope: schema.string().min(1), partition: schema.string().min(1) }),
        ]),
    },
    UNPROCESSABLE_CONTENT: { status: CALL_ERROR_STATUSES.UNPROCESSABLE_CONTENT },
    TOO_MANY_REQUESTS: {
        status: CALL_ERROR_STATUSES.TOO_MANY_REQUESTS,
        // how long until the call may be retried, in milliseconds, as Retry-After states it
        data: schema.object({ retryAfter: schema.number().nonnegative() }),
    },
    SERVICE_UNAVAILABLE: { status: CALL_ERROR_STATUSES.SERVICE_UNAVAILABLE },
} as const satisfies Partial<
    Record<CallErrorCode, { readonly status: number; readonly data?: schema.Schema }>
>;

/** What a call failure has beside its code, as gRPC's ErrorInfo beside its status. */
export type CallErrorOptions<Data> = ORPCErrorOptions<Data> & {
    /** The failure's stable cause within its domain, such as STALE_REVISION. */
    readonly reason: string;
    /** The package whose reason it is, such as @destack/object, or http. */
    readonly domain: string;
    /** The readable message. */
    readonly message: string;
};

/** A failure's JSON body: oRPC's, with its reason and domain when a package gave one. */
const CallErrorJson = schema.object({
    /** Whether the procedure declares the failure. */
    defined: schema.boolean(),
    /** The call error code. */
    code: schema.string().min(1),
    /** The HTTP status. */
    status: schema.number().int(),
    /** The readable message. */
    message: schema.string(),
    /** The structured details. */
    data: schema.unknown().exactOptional(),
    /** The failure's stable cause within its domain. */
    reason: FailureReason.exactOptional(),
    /** The package whose reason it is. */
    domain: schema.string().min(1).exactOptional(),
});

/** A call failure: its code and status, its reason within its domain, a message and details. */
export class CallError<Code extends ORPCErrorCode, Data> extends ORPCError<Code, Data> {
    /** The failure's stable cause within its domain. */
    readonly reason: string;
    /** The package whose reason it is. */
    readonly domain: string;

    /** Create a failure with the status its code declares. */
    constructor(code: Code, ...rest: MaybeOptionalOptions<CallErrorOptions<Data>>) {
        // take the status and definition its code declares
        const options = resolveMaybeOptionalOptions(rest);
        super(code, {
            ...options,
            status: options.status ?? CallError.status(code),
            defined: options.defined ?? Object.hasOwn(DECLARED_ERRORS, code),
        });

        // record its reason within its domain
        this.reason = FailureReason.parse(options.reason);
        this.domain = options.domain;
    }

    /** Describe a failure no package gave a reason, as HTTP's: its code is its reason. */
    static http<Code extends CallErrorCode>(
        code: Code,
        options: { readonly message: string; readonly cause?: unknown },
    ): CallError<Code, undefined> {
        return new CallError(code, { ...options, reason: code, domain: HTTP_DOMAIN });
    }

    /** Write the failure's body: oRPC's, with its reason and domain. */
    override toJSON(): ORPCErrorJSON<Code, Data> & {
        readonly reason: string;
        readonly domain: string;
    } {
        return { ...super.toJSON(), reason: this.reason, domain: this.domain };
    }

    /** Read a failure from its body, a body without a reason as HTTP's. */
    static fromJson(body: unknown): CallError<string, unknown> | undefined {
        // leave a body of no failure to its reader
        const parsed = CallErrorJson.safeParse(body);
        if (!parsed.success) {
            return undefined;
        }

        // read it with its reason, one without as HTTP's
        const { reason, domain, ...read } = parsed.data;

        return new CallError(read.code, {
            ...(reason === undefined || domain === undefined
                ? { reason: read.code, domain: HTTP_DOMAIN }
                : { reason, domain }),
            message: read.message,
            data: read.data,
            status: read.status,
            defined: read.defined,
        });
    }

    /** Read the status a code declares, else 500. */
    static status(code: ORPCErrorCode): number {
        const statuses: Readonly<Record<string, number>> = CALL_ERROR_STATUSES;

        return statuses[code] ?? fallbackORPCErrorStatus(code, undefined);
    }

    /** Read the call error a failure is when it refuses for good, absent otherwise. */
    static refusal(error: unknown): CallError<string, unknown> | undefined {
        const known = CallError.of(error);
        const isRefused =
            known !== undefined &&
            known.status >= 400 &&
            known.status < 500 &&
            !Failure.isTransient(Failure.of(known));

        return isRefused ? known : undefined;
    }

    /** Read the data of a failure of a code that declares data. */
    static data(
        error: unknown,
        code: "MISDIRECTED_REQUEST",
    ): schema.Infer<typeof DECLARED_ERRORS.MISDIRECTED_REQUEST.data> | undefined {
        const failure = CallError.of(error);

        return failure?.code === code
            ? DECLARED_ERRORS[code].data.safeParse(failure.data).data
            : undefined;
    }

    /** Read the data of a response's failure of a code that declares data. */
    static async read(
        response: Response,
        code: "MISDIRECTED_REQUEST",
    ): Promise<schema.Infer<typeof DECLARED_ERRORS.MISDIRECTED_REQUEST.data> | undefined> {
        // skip other statuses without reading their body
        if (response.status !== CallError.status(code)) {
            return undefined;
        }

        // read the failure in the body, skipping a body that is not JSON, such as another origin's
        const body: unknown = await response
            .clone()
            .json()
            .catch(() => undefined);
        const failure = schema
            .looseObject({ code: schema.literal(code), data: DECLARED_ERRORS[code].data })
            .safeParse(body);

        return failure.data?.data;
    }

    /** Send a call, and send it once more when it answers misdirected. */
    static async follow(
        send: (
            misdirected?: schema.Infer<typeof DECLARED_ERRORS.MISDIRECTED_REQUEST.data>,
        ) => Promise<Response>,
    ): Promise<Response> {
        // answer anything but a misdirection
        const answered = await send();
        const misdirected = await CallError.read(answered, "MISDIRECTED_REQUEST");
        if (misdirected === undefined) {
            return answered;
        }

        // drop the misdirected answer and send once more
        await answered.body?.cancel();

        return send(misdirected);
    }

    /** Read the call error a failure is or has, absent for a failure without one. */
    static of(error: unknown): CallError<string, unknown> | undefined {
        // take a call error as it is, and an oRPC failure as HTTP's with the failure it reports
        if (error instanceof CallError) {
            return error;
        } else if (isCallError(error)) {
            return new CallError(error.code, {
                reason: error.code,
                domain: HTTP_DOMAIN,
                message: error.message,
                data: error.data,
                status: error.status,
                cause: error.cause,
            });
        }
        // build a domain failure's call error with the failure as its cause
        else if (DomainError.is(error)) {
            const { code, reason, domain, message, data } = error.toCallError();

            return new CallError<CallErrorCode, JsonValue | undefined>(code, {
                reason,
                domain,
                message,
                data,
                cause: error,
            });
        }

        return undefined;
    }
}

/** Report whether a value is a call error, of any code and data. */
function isCallError(value: unknown): value is ORPCError<string, unknown> {
    return value instanceof ORPCError;
}

/** Hide a denial from the caller as a missing resource. */
export function conceal(denial: Error, message: string): CallError<"NOT_FOUND", unknown> {
    return CallError.http("NOT_FOUND", {
        message,
        cause: denial,
    });
}

/** Refuse input its schema rejects, as a procedure refuses invalid input. */
export function refuseInput(
    input: unknown,
    issues: readonly SchemaIssue[],
): CallError<"BAD_REQUEST", { readonly issues: readonly SchemaIssue[] }> {
    const message = describeIssues(issues);

    return new CallError("BAD_REQUEST", {
        reason: "INVALID_INPUT",
        domain: import.meta.destack.package.name,
        message,
        data: { issues },
        cause: new ValidationError({ message, issues, data: input }),
    });
}

/** Describe invalid input by its issues, each as its path and its lowercase message. */
export function describeIssues(issues: readonly SchemaIssue[]): string {
    const described = issues.map((issue) => {
        const path = (issue.path ?? [])
            .map((key) => (typeof key === "object" ? key.key : key))
            .join(".");
        const message = issue.message.charAt(0).toLowerCase() + issue.message.slice(1);

        return path === "" ? message : `${path}: ${message}`;
    });

    return `invalid input: ${described.join("; ")}`;
}

/** Read the denial of a failure: itself for a 401 or 403, the cause of a concealed one. */
export function findDenial(
    failure: ORPCError<string, unknown>,
): ORPCError<string, unknown> | undefined {
    // take a refusal of access, or the refusal a missing resource conceals
    if (failure.status === 401 || failure.status === 403) {
        return failure;
    } else if (failure.status === 404 && isCallError(failure.cause)) {
        return findDenial(failure.cause);
    }

    return undefined;
}
