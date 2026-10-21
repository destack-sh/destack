import { JsonValue, schema } from "@destack/schema";
import { CALL_ERROR_STATUSES, CallErrorCode, DomainError } from "./error.ts";

/** The call error codes, in the order their statuses declare them. */
const CALL_ERROR_CODES = Object.keys(CALL_ERROR_STATUSES).filter((code) => CallErrorCode.is(code));

/** The schema of a call's failure. */
const failureSchema = schema.object({
    /** The call error code, such as CONFLICT. */
    code: schema.enum(CALL_ERROR_CODES),
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
        // read a call error by its code and details
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

            return { code: error.code, message: error.message, ...data };
        }
        // read an error with only an HTTP status by the status's code
        else if (error instanceof Error && "status" in error && typeof error.status === "number") {
            return { code: CallErrorCode.of(error.status), message: error.message };
        }
        // read any other thrown value as an internal failure with its message
        else {
            const message = error instanceof Error ? error.message : String(error);

            return { code: "INTERNAL_SERVER_ERROR", message };
        }
    },

    /** Raise a failure again as an error that converts back to it. */
    error(failure: Failure, options?: ErrorOptions): DomainError {
        return Object.assign(new Error(failure.message, options), {
            name: "Failure",
            code: failure.code,
            toCallError: (): Failure => failure,
        });
    },

    /** Read the HTTP status of a failure's code. */
    status(failure: Failure): number {
        return CALL_ERROR_STATUSES[failure.code];
    },

    /** Read the failure a failed response answers: its body's, else its status's with the text. */
    async read(response: Response): Promise<Failure> {
        const text = await response.text();
        const answered = failureSchema.strip().safeParse(JsonValue.parse(text));

        return answered.success
            ? answered.data
            : { code: CallErrorCode.of(response.status), message: text };
    },
});
