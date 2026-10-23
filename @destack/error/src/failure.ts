import { JsonValue, schema } from "@destack/schema";
import { CALL_ERROR_STATUSES, CallErrorCode, DomainError } from "./error.ts";

/** The call error codes, in the order their statuses declare them. */
const CALL_ERROR_CODES = Object.keys(CALL_ERROR_STATUSES).filter((code) => CallErrorCode.is(code));

/** The codes of failures a later or rerouted attempt may not meet (RFC 9110, 6585). */
const TRANSIENT_CODES: ReadonlySet<CallErrorCode> = new Set([
    "TIMEOUT",
    "MISDIRECTED_REQUEST",
    "TOO_MANY_REQUESTS",
    "SERVICE_UNAVAILABLE",
]);

/** The domain of failures without a package's reason: HTTP, whose status code is their reason. */
export const HTTP_DOMAIN = "http";

/** A failure's stable cause within its domain, as gRPC's ErrorInfo reason is (AIP-193). */
export const FailureReason = schema
    .string()
    .max(63)
    .regex(/^[A-Z][A-Z0-9_]+[A-Z0-9]$/u);

/** The schema of a call's failure. */
const failureSchema = schema.object({
    /** The call error code, such as CONFLICT. */
    code: schema.enum(CALL_ERROR_CODES),
    /** The failure's stable cause within its domain, such as STALE_REVISION. */
    reason: FailureReason,
    /** The package whose reason it is, such as @destack/object, or http. */
    domain: schema.string().min(1),
    /** The readable message. */
    message: schema.string(),
    /** The structured details. */
    data: schema.json().exactOptional(),
});
/** How a call failed: the call error a caller receives, as plain values. */
export type Failure = schema.Infer<typeof failureSchema>;

/** How a call failed, how a thrown value reads as one, and how one raises again. */
export const Failure = Object.assign(failureSchema, {
    /** Describe a thrown value as the failure its caller receives. */
    of(error: unknown): Failure {
        // read a domain error as the call error it converts to
        if (DomainError.is(error)) {
            return error.toCallError();
        }
        // read a call error by its code, reason and details, one without a reason as HTTP's
        else if (
            error instanceof Error &&
            "code" in error &&
            typeof error.code === "string" &&
            CallErrorCode.is(error.code)
        ) {
            const data =
                "data" in error && error.data !== undefined
                    ? { data: JsonValue.of(error.data) }
                    : {};
            const reason =
                "reason" in error && typeof error.reason === "string" ? error.reason : undefined;
            const domain =
                "domain" in error && typeof error.domain === "string" ? error.domain : undefined;

            return {
                ...Failure.http(error.code, error.message),
                ...(reason === undefined || domain === undefined
                    ? {}
                    : { reason: FailureReason.parse(reason), domain }),
                ...data,
            };
        }
        // read a refused connection as the peer being unavailable
        else if (
            error instanceof Error &&
            "code" in error &&
            (error.code === "ECONNREFUSED" || error.code === "ConnectionRefused")
        ) {
            return Failure.http("SERVICE_UNAVAILABLE", error.message);
        }
        // read an error with only an HTTP status by the status's code
        else if (error instanceof Error && "status" in error && typeof error.status === "number") {
            return Failure.http(CallErrorCode.of(error.status), error.message);
        }
        // read any other thrown value as an internal failure with its message
        else {
            const message = error instanceof Error ? error.message : String(error);

            return Failure.http("INTERNAL_SERVER_ERROR", message);
        }
    },

    /** Report whether a later or rerouted attempt may succeed: a timeout, move, limit or outage. */
    isTransient(failure: Pick<Failure, "code">): boolean {
        return TRANSIENT_CODES.has(failure.code);
    },

    /** Describe a failure no package gave a reason, as HTTP's: its code is its reason. */
    http(code: CallErrorCode, message: string): Failure {
        return { code, reason: code, domain: HTTP_DOMAIN, message };
    },

    /** Raise a failure again as an error that converts back to it. */
    error(failure: Failure, options?: ErrorOptions): DomainError {
        return Object.assign(new Error(failure.message, options), {
            name: "Failure",
            code: failure.code,
            reason: failure.reason,
            domain: failure.domain,
            toCallError: (): Failure => failure,
        });
    },

    /** Read the HTTP status of a failure's code. */
    status(failure: Failure): number {
        return CALL_ERROR_STATUSES[failure.code];
    },

    /** Read the failure a failed response answers: its body's, else its status's with the text. */
    async read(response: Response): Promise<Failure> {
        // read the body's failure, one without a reason as HTTP's
        const text = await response.text();
        const answered = failureSchema
            .partial({ reason: true, domain: true })
            .strip()
            .safeParse(JsonValue.parse(text));
        if (!answered.success) {
            return Failure.http(CallErrorCode.of(response.status), text);
        }
        const { reason, domain, ...failure } = answered.data;

        return reason === undefined || domain === undefined
            ? { ...Failure.http(failure.code, failure.message), ...failure }
            : { ...failure, reason, domain };
    },
});
