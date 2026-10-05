import {
    fallbackORPCErrorStatus,
    ORPCError,
    type ORPCErrorCode,
    type ORPCErrorOptions,
} from "@orpc/client";
import { type MaybeOptionalOptions, resolveMaybeOptionalOptions } from "@orpc/shared";
import { ValidationError, type SchemaIssue } from "@orpc/contract";
import {
    type JsonValue,
    ReportableError,
    SERVICE_ERROR_STATUSES,
    type ServiceErrorCode,
} from "@destack/schema";
import type { Failure } from "@destack/sync";

/** The client failures worth attempting again: a request timeout, a request too early, and a throttle (RFC 9110, 8470, 6585). */
export const TRANSIENT_STATUSES: ReadonlySet<number> = new Set([408, 425, 429]);

/** A service failure: its code, the status the code declares, a message and details. */
export class ServiceError<Code extends ORPCErrorCode, Data> extends ORPCError<Code, Data> {
    /** Create a failure with the status its code declares. */
    constructor(code: Code, ...rest: MaybeOptionalOptions<ORPCErrorOptions<Data>>) {
        const options = resolveMaybeOptionalOptions(rest);
        super(code, { ...options, status: options.status ?? ServiceError.status(code) });
    }

    /** Read the status a code declares, else 500. */
    static status(code: ORPCErrorCode): number {
        const statuses: Readonly<Record<string, number>> = SERVICE_ERROR_STATUSES;

        return statuses[code] ?? fallbackORPCErrorStatus(code, undefined);
    }

    /** Read the service error a failure is or names, absent for a failure that names none. */
    static of(error: unknown): ORPCError<string, unknown> | undefined {
        // keep a service error
        if (isServiceError(error)) {
            return error;
        }
        // build the service error a failure names with the failure as its cause
        else if (ReportableError.is(error)) {
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
    const message = "Input validation failed";

    return new ORPCError("BAD_REQUEST", {
        message,
        data: { issues },
        cause: new ValidationError({ message, issues, data: input }),
    });
}

/** Rebuild the service error a failure records, with its code, status, message and details. */
export function errorOf(failure: Failure): ORPCError<string, JsonValue | undefined> {
    return new ORPCError<string, JsonValue | undefined>(failure.code, {
        status: failure.status,
        message: failure.message,
        ...(failure.data === undefined ? {} : { data: failure.data }),
    });
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
