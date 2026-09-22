import { defineSchema, schema } from "@destack/schema";
import { ResourceHandle } from "./handle.ts";

/** A declaration name within a package. */
export const ResourceName = defineSchema(schema.string().regex(/^[a-z][a-z0-9-]*$(?![\s\S])/));
/** A declaration name within a package. */
export type ResourceName = schema.Infer<typeof ResourceName>;

/** A named infrastructure dependency declared by a package. */
export const ResourceDeclaration = defineSchema(
    schema.object({
        /** The package-local resource name. */
        name: ResourceName,
        /** The resource kind defined by its domain library. */
        kind: ResourceName,
        /** The declaration format version. */
        version: schema.number().int().positive(),
        /** The specification validated by the domain library. */
        spec: schema.record(schema.string(), schema.json()),
    }),
);
/** A named infrastructure dependency declared by a package. */
export type ResourceDeclaration = schema.Infer<typeof ResourceDeclaration>;

/** An inert declaration with access to a host-bound client. */
export class Resource<
    Handle,
    Declaration extends ResourceDeclaration = ResourceDeclaration,
> extends ResourceHandle<Handle> {
    /** The resource kind. */
    readonly kind: Declaration["kind"];
    /** The declaration format version. */
    readonly version: Declaration["version"];
    /** The domain specification. */
    readonly spec: Declaration["spec"];

    /** Retain validated metadata without opening a resource. */
    constructor(declaration: Declaration) {
        super(declaration.name);
        this.kind = declaration.kind;
        this.version = declaration.version;
        this.spec = declaration.spec;
    }
}

/** Define a resource declaration with a concrete specification. */
export function defineResourceSchema<const Kind extends string, Spec extends schema.Schema>(
    kind: Kind,
    version: number,
    spec: Spec,
) {
    ResourceDeclaration.pick({ kind: true, version: true }).parse({ kind, version });

    return defineSchema(
        ResourceDeclaration.extend({
            kind: schema.literal(kind),
            version: schema.literal(version),
            spec,
        }),
    );
}
