import { defineSchema, schema, Version } from "@destack/schema";
import { ObjectTypeReference } from "../scope/reference.ts";

/** A UUID of version 7 or 8, whose leading 48 bits a request identifier fills with its time. */
const TIME_ORDERED_UUID = /^.{14}[78]/u;

/** A method call: the type, the method, its input and the release it was made against. */
export const Call = Object.assign(
    defineSchema(
        schema.object({
            /** The package-qualified object type. */
            type: ObjectTypeReference,
            /** The method's name, such as create. */
            method: schema.string().min(1),
            /** The method's input, with its scope and target. */
            input: schema.jsonObject(),
            /** The release of the method's package the call was made against. */
            release: Version,
        }),
    ),
    {
        /** Label a call's method for messages and telemetry, such as note.create. */
        label(call: Pick<Call, "type" | "method">): string {
            return `${call.type.type}.${call.method}`;
        },
    },
);
/** A method call: its type, method, input and release, typed by the result it answers. */
export type Call<Result = unknown> = schema.Infer<typeof Call> & {
    /** The result the call answers, as a type only. */
    readonly result?: Result;
};

/** Calls a client makes atomically. */
export const Mutation = defineSchema(
    schema.object({
        /** The request identifier: a UUIDv7 a caller created, or a UUIDv8 it derived. */
        id: schema.uuid().regex(TIME_ORDERED_UUID),
        /** The calls in order. */
        calls: schema.array(Call).min(1),
    }),
);
/** Calls a client makes atomically. */
export type Mutation = schema.Infer<typeof Mutation>;
