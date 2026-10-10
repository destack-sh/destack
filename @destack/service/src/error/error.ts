import {
    fallbackORPCErrorStatus,
    ORPCError,
    type ORPCErrorCode,
    type ORPCErrorOptions,
} from "@orpc/client";
import { type MaybeOptionalOptions, resolveMaybeOptionalOptions } from "@orpc/shared";
import { ValidationError, type SchemaIssue } from "@orpc/contract";
import { JsonValue, schema } from "@destack/schema";
import { DomainError, SERVICE_ERROR_STATUSES, type ServiceErrorCode } from "@destack/error";
import { Failure } from "@destack/sync";

/** The failures every procedure declares with their statuses and data, which callers handle by their codes. */
export const DECLARED_ERRORS = {
    NOT_IMPLEMENTED: { status: SERVICE_ERROR_STATUSES.NOT_IMPLEMENTED },
    UNAUTHORIZED: { status: SERVICE_ERROR_STATUSES.UNAUTHORIZED },
    INSUFFICIENT_AUTHENTICATION: { status: SERVICE_ERROR_STATUSES.INSUFFICIENT_AUTHENTICATION },
    QUOTA_EXCEEDED: { status: SERVICE_ERROR_STATUSES.QUOTA_EXCEEDED },
    FORBIDDEN: { status: SERVICE_ERROR_STATUSES.FORBIDDEN },
    NOT_FOUND: { status: SERVICE_ERROR_STATUSES.NOT_FOUND },
    CONFLICT: { status: SERVICE_ERROR_STATUSES.CONFLICT },
    PRECONDITION_FAILED: { status: SERVICE_ERROR_STATUSES.PRECONDITION_FAILED },
    MANAGED: { status: SERVICE_ERROR_STATUSES.MANAGED },
    MISDIRECTED_REQUEST: {
        status: SERVICE_ERROR_STATUSES.MISDIRECTED_REQUEST,
        // where the call's scope is served: another machine, or another partition of the installation
        data: schema.union([
            schema.object({ scope: schema.string().min(1), machine: schema.string().min(1) }),
            schema.object({ scope: schema.string().min(1), partition: schema.string().min(1) }),
        ]),
    },
    UNPROCESSABLE_CONTENT: { status: SERVICE_ERROR_STATUSES.UNPROCESSABLE_CONTENT },
    TOO_MANY_REQUESTS: { status: SERVICE_ERROR_STATUSES.TOO_MANY_REQUESTS },
    SERVICE_UNAVAILABLE: { status: SERVICE_ERROR_STATUSES.SERVICE_UNAVAILABLE },
} as const satisfies Partial<
    Record<ServiceErrorCode, { readonly status: number; readonly data?: schema.Schema }>
>;

/** The client failures worth attempting again: a request timeout, a request too early, and a throttle (RFC 9110, 8470, 6585). */
export const TRANSIENT_STATUSES: ReadonlySet<number> = new Set([408, 425, 429]);

/** A service failure: its code, the status the code declares, a message and details. */
export class ServiceError<Code extends ORPCErrorCode, Data> extends ORPCError<Code, Data> {
    /** Create a failure with the status its code declares, defined when every procedure declares its code. */
    constructor(code: Code, ...rest: MaybeOptionalOptions<ORPCErrorOptions<Data>>) {
        const options = resolveMaybeOptionalOptions(rest);
        super(code, {
            ...options,
            status: options.status ?? ServiceError.status(code),
            defined: options.defined ?? Object.hasOwn(DECLARED_ERRORS, code),
        });
    }

    /** Read the status a code declares, else 500. */
    static status(code: ORPCErrorCode): number {
        const statuses: Readonly<Record<string, number>> = SERVICE_ERROR_STATUSES;

        return statuses[code] ?? fallbackORPCErrorStatus(code, undefined);
    }

    /** Read the data of a failure of a code that declares data, absent for another failure or data the code rejects. */
    static data(
        error: unknown,
        code: "MISDIRECTED_REQUEST",
    ): schema.Infer<typeof DECLARED_ERRORS.MISDIRECTED_REQUEST.data> | undefined {
        return isServiceError(error) && error.code === code
            ? DECLARED_ERRORS[code].data.safeParse(error.data).data
            : undefined;
    }

    /** Read the data of a response's failure of a code that declares data, absent for another status, a body that is no failure of the code, or data the code rejects. */
    static async read(
        response: Response,
        code: "MISDIRECTED_REQUEST",
    ): Promise<schema.Infer<typeof DECLARED_ERRORS.MISDIRECTED_REQUEST.data> | undefined> {
        // skip other statuses without reading their body
        if (response.status !== ServiceError.status(code)) {
            return undefined;
        }

        // read the failure the body answers, skipping one that is no JSON, such as another origin's
        const body: unknown = await response
            .clone()
            .json()
            .catch(() => undefined);
        const failure = schema
            .looseObject({ code: schema.literal(code), data: DECLARED_ERRORS[code].data })
            .safeParse(body);

        return failure.data?.data;
    }

    /** Send a call, and send it once more when it answers misdirected, handing the second attempt where its scope is served. */
    static async follow(
        send: (
            misdirected?: schema.Infer<typeof DECLARED_ERRORS.MISDIRECTED_REQUEST.data>,
        ) => Promise<Response>,
    ): Promise<Response> {
        // answer anything but a misdirection
        const answered = await send();
        const misdirected = await ServiceError.read(answered, "MISDIRECTED_REQUEST");
        if (misdirected === undefined) {
            return answered;
        }

        // drop the misdirected answer and send once more
        await answered.body?.cancel();

        return send(misdirected);
    }

    /** Read the service error a failure is or carries, absent for a failure that carries none. */
    static of(error: unknown): ORPCError<string, unknown> | undefined {
        // keep a service error
        if (isServiceError(error)) {
            return error;
        }
        // build the service error a failure carries with the failure as its cause
        else if (DomainError.is(error)) {
            const { code, message, data } = error.toServiceError();

            return new ServiceError<ServiceErrorCode, JsonValue | undefined>(code, {
                message,
                data,
                cause: error,
            });
        }

        return undefined;
    }
}

/** Report whether a value is a service error, of any code and data. */
export function isServiceError(value: unknown): value is ORPCError<string, unknown> {
    return value instanceof ORPCError;
}

/** Hide a denial from the caller as a missing resource and keep the denial as the cause for audit. */
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

/** Rebuild the service error a failure records, with its code, status, message and details. */
export function errorOf(failure: Failure): ORPCError<string, JsonValue | undefined> {
    return new ORPCError<string, JsonValue | undefined>(failure.code, {
        status: failure.status,
        message: failure.message,
        ...(failure.data === undefined ? {} : { data: failure.data }),
    });
}

/** Read the failure a failed response answers: the service's own, or one of its status and body text. */
export async function failureOf(response: Response): Promise<Failure> {
    // read the service's own failure, else describe the status by the body
    const text = await response.text();
    const answered = Failure.strip().safeParse(JsonValue.parse(text));

    return answered.success
        ? { ...answered.data, status: response.status }
        : { code: `HTTP_${response.status}`, status: response.status, message: text };
}

/** Read the denial of a failure: itself for a 401 or 403, the cause of a concealed one. */
export function denialOf(
    failure: ORPCError<string, unknown>,
): ORPCError<string, unknown> | undefined {
    // take a refusal of access, or the refusal a missing resource conceals
    if (failure.status === 401 || failure.status === 403) {
        return failure;
    } else if (failure.status === 404 && isServiceError(failure.cause)) {
        return denialOf(failure.cause);
    }

    return undefined;
}
