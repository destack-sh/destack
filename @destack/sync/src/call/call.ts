import { defineSchema, schema, Version } from "@destack/schema";

/** A method call: the method, its input and the release it was made against. */
export const Call = defineSchema(
    schema.object({
        /** The object type and method, such as page.create. */
        method: schema.string().min(1),
        /** The method's input, with its scope and target. */
        input: schema.record(schema.string(), schema.json()),
        /** The release of the method's package the call was made against. */
        release: Version,
    }),
);
/** A method call: the method, its input and the release it was made against. */
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
