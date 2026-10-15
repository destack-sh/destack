import { defineSchema, schema, Version } from "@destack/schema";
import { ObjectTypeReference } from "../scope/reference.ts";

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
/** A method call: the type, the method, its input and the release it was made against. */
export type Call = schema.Infer<typeof Call>;

/** Calls a client makes atomically. */
export const Mutation = defineSchema(
    schema.object({
        /** The request identifier. */
        id: schema.uuidv7(),
        /** The calls in order. */
        calls: schema.array(Call).min(1),
    }),
);
/** Calls a client makes atomically. */
export type Mutation = schema.Infer<typeof Mutation>;
