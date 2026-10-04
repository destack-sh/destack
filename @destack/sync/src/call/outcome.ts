import { defineSchema, JsonValue, schema } from "@destack/schema";

/** The HTTP status of a failure thrown without one. */
const INTERNAL_STATUS = 500;

/** The schema of a call's failure. */
const failureSchema = defineSchema(
    schema.object({
        /** The error code, such as CONFLICT. */
        code: schema.string().min(1),
        /** The HTTP status. */
        status: schema.number().int(),
        /** The readable message. */
        message: schema.string(),
        /** The structured details. */
        data: schema.json().exactOptional(),
    }),
);
/** How a call failed, as the wire sends a service error. */
export type Failure = schema.Infer<typeof failureSchema>;

/** How a call failed, as the wire sends a service error, and how a thrown value reads as one. */
export const Failure = Object.assign(failureSchema, {
    /** Describe a thrown value by its code, status, message and details, or its error name. */
    of(error: unknown): Failure {
        // read an error's code, status and details where it has them
        if (error instanceof Error) {
            const code =
                "code" in error && typeof error.code === "string" ? error.code : error.name;
            const status =
                "status" in error && typeof error.status === "number"
                    ? error.status
                    : INTERNAL_STATUS;
            const data =
                "data" in error && error.data !== undefined
                    ? { data: JsonValue.of(error.data) }
                    : {};

            return { code, status, message: error.message, ...data };
        }

        return { code: "THROWN", status: INTERNAL_STATUS, message: String(error) };
    },
});

/** How a call ended. */
export const Outcome = defineSchema(
    schema.discriminatedUnion("kind", [
        schema.object({
            /** The call succeeded. */
            kind: schema.literal("success"),
            /** The result a retry of the request replays, kept only while it may retry. */
            value: schema.json().exactOptional(),
        }),
        schema.object({
            /** The call failed, was denied or was cancelled. */
            kind: schema.enum(["failure", "denied", "cancelled"]),
            /** The failure. */
            error: failureSchema,
        }),
    ]),
);
/** How a call ended. */
export type Outcome = schema.Infer<typeof Outcome>;
