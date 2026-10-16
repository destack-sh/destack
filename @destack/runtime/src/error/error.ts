import {
    fallbackORPCErrorStatus,
    ORPCError,
    type ORPCErrorCode,
    type ORPCErrorOptions,
} from "@orpc/client";
import { type MaybeOptionalOptions, resolveMaybeOptionalOptions } from "@orpc/shared";
import { ValidationError, type SchemaIssue } from "@orpc/contract";
import { JsonValue, schema } from "@destack/schema";
import { DomainError, CALL_ERROR_STATUSES, type CallErrorCode } from "@destack/error";
import { Failure } from "@destack/sync";

/** The failures every procedure declares with their statuses and data. */
export const DECLARED_ERRORS = {
    NOT_IMPLEMENTED: { status: CALL_ERROR_STATUSES.NOT_IMPLEMENTED },
    UNAUTHORIZED: { status: CALL_ERROR_STATUSES.UNAUTHORIZED },
    INSUFFICIENT_AUTHENTICATION: { status: CALL_ERROR_STATUSES.INSUFFICIENT_AUTHENTICATION },
    QUOTA_EXCEEDED: { status: CALL_ERROR_STATUSES.QUOTA_EXCEEDED },
    FORBIDDEN: { status: CALL_ERROR_STATUSES.FORBIDDEN },
    NOT_FOUND: { status: CALL_ERROR_STATUSES.NOT_FOUND },
    CONFLICT: { status: CALL_ERROR_STATUSES.CONFLICT },
    PRECONDITION_FAILED: { status: CALL_ERROR_STATUSES.PRECONDITION_FAILED },
    MANAGED: { status: CALL_ERROR_STATUSES.MANAGED },
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

/** The client failures worth attempting again (RFC 9110, 8470, 6585). */
export const TRANSIENT_STATUSES: ReadonlySet<number> = new Set([408, 425, 429]);

/** A call failure: its code, the status the code declares, a message and details. */
export class CallError<Code extends ORPCErrorCode, Data> extends ORPCError<Code, Data> {
    /** Create a failure with the status its code declares. */
    constructor(code: Code, ...rest: MaybeOptionalOptions<ORPCErrorOptions<Data>>) {
        const options = resolveMaybeOptionalOptions(rest);
        super(code, {
            ...options,
            status: options.status ?? CallError.status(code),
            defined: options.defined ?? Object.hasOwn(DECLARED_ERRORS, code),
        });
    }

    /** Read the status a code declares, else 500. */
    static status(code: ORPCErrorCode): number {
        const statuses: Readonly<Record<string, number>> = CALL_ERROR_STATUSES;

        return statuses[code] ?? fallbackORPCErrorStatus(code, undefined);
    }

    /** Report whether a status refuses for good: a client failure other than the transient ones. */
    static isRefusal(status: number): boolean {
        return status >= 400 && status < 500 && !TRANSIENT_STATUSES.has(status);
    }

    /** Read the data of a failure of a code that declares data. */
    static data(
        error: unknown,
        code: "MISDIRECTED_REQUEST",
    ): schema.Infer<typeof DECLARED_ERRORS.MISDIRECTED_REQUEST.data> | undefined {
        return isCallError(error) && error.code === code
            ? DECLARED_ERRORS[code].data.safeParse(error.data).data
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
    static of(error: unknown): ORPCError<string, unknown> | undefined {
        // take a call error as it is
        if (isCallError(error)) {
            return error;
        }
        // build a domain failure's call error with the failure as its cause
        else if (DomainError.is(error)) {
            const { code, message, data } = error.toCallError();

            return new CallError<CallErrorCode, JsonValue | undefined>(code, {
                message,
                data,
                cause: error,
            });
        }

        return undefined;
    }
}

/** Report whether a value is a call error, of any code and data. */
export function isCallError(value: unknown): value is ORPCError<string, unknown> {
    return value instanceof ORPCError;
}

/** Hide a denial from the caller as a missing resource. */
export function conceal(
    denial: ORPCError<string, unknown>,
    message: string,
): ORPCError<"NOT_FOUND", unknown> {
    return new ORPCError("NOT_FOUND", { message, cause: denial });
}

/** Refuse input its schema rejects, as a procedure refuses invalid input. */
export function refuseInput(
    input: unknown,
    issues: readonly SchemaIssue[],
): ORPCError<"BAD_REQUEST", { readonly issues: readonly SchemaIssue[] }> {
    const message = describeIssues(issues);

    return new ORPCError("BAD_REQUEST", {
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

/** Rebuild the call error a failure records, with its code, status, message, details and cause. */
export function errorOf(failure: Failure, cause?: Error): ORPCError<string, JsonValue | undefined> {
    return new ORPCError<string, JsonValue | undefined>(failure.code, {
        status: failure.status,
        message: failure.message,
        ...(failure.data === undefined ? {} : { data: failure.data }),
        ...(cause === undefined ? {} : { cause }),
    });
}

/** Read the failure a failed response answers. */
export async function failureOf(response: Response): Promise<Failure> {
    // read the server's own failure, else describe the status by the body
    const text = await response.text();
    const answered = Failure.strip().safeParse(JsonValue.parse(text));

    return answered.success
        ? { ...answered.data, status: response.status }
        : { code: `HTTP_${response.status}`, status: response.status, message: text };
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
