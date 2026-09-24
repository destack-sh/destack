import { defineSchema, schema } from "@destack/schema";
import { DeclarationName, type Package } from "@destack/package";
import { ResourceHandle } from "./handle.ts";

/** A named infrastructure dependency declared by a package. */
export const ResourceDescription = defineSchema(
    schema.object({
        /** The package-local resource name. */
        name: DeclarationName,
        /** The resource kind defined by its domain library. */
        kind: DeclarationName,
        /** The declaration format version. */
        version: schema.number().int().positive(),
        /** The specification validated by the domain library. */
        spec: schema.record(schema.string(), schema.json()),
    }),
);
/** A named infrastructure dependency declared by a package. */
export type ResourceDescription = schema.Infer<typeof ResourceDescription>;

/** An inert declaration with access to a host-bound client. */
export class Resource<
    Handle,
    Declaration extends ResourceDescription = ResourceDescription,
> extends ResourceHandle<Handle> {
    /** The resource kind. */
    readonly kind: Declaration["kind"];
    /** The declaration format version. */
    readonly version: Declaration["version"];
    /** The domain specification. */
    readonly spec: Declaration["spec"];

    /** Retain validated metadata without opening a resource. */
    constructor(owner: Package, declaration: Declaration) {
        // retain the declared kind, version and spec
        super(owner, declaration.name);
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
    ResourceDescription.pick({ kind: true, version: true }).parse({ kind, version });

    return defineSchema(
        ResourceDescription.extend({
            kind: schema.literal(kind),
            version: schema.literal(version),
            spec,
        }),
    );
}
