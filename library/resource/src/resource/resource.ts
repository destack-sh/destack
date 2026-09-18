import { defineSchema, schema } from "@destack/schema";

/** A named resource declaration within a package. */
export const Resource = defineSchema(schema.object({
    /** The declaration name, independent of its export name. */
    name: schema.string().min(1),
    /** The resource kind supplied by its defining library. */
    kind: schema.string().min(1),
    /** The resource description format version. */
    version: schema.number().int().min(1),
}));

/** A named resource declaration within a package. */
export type Resource = schema.Infer<typeof Resource>;

/** Define a resource kind with a validated specification. */
export function defineResource<const Kind extends string, Spec extends schema.Schema>(
    kind: Kind,
    version: number,
    spec: Spec,
) {
    // validate the common fields before constructing the specialized schema
    Resource.pick({ kind: true, version: true }).parse({ kind, version });

    return defineSchema(Resource.extend({
        kind: schema.literal(kind),
        version: schema.literal(version),
        spec,
    }));
}
