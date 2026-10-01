import { anyIdentifier, defineSchema, identifier, schema } from "@destack/schema";
import { DeclarationName, type Package } from "@destack/package";
import type { ResourceState } from "@destack/package/declare";
import type { Connector } from "../provider/provider.ts";
import { ResourceHandle } from "./handle.ts";

/** A resource's identifier, whichever kind it is: the kind's name as prefix and a UUIDv7. */
export const ResourceId = defineSchema(anyIdentifier());
/** A resource's identifier, whichever kind it is. */
export type ResourceId = schema.Infer<typeof ResourceId>;

/** A named infrastructure dependency declared by a package. */
export const ResourceDescription = defineSchema(
    schema.object({
        /** The package-local resource name. */
        name: DeclarationName,
        /** The resource kind defined by its domain library. */
        kind: DeclarationName,
        /** The specification validated by the domain library. */
        spec: schema.record(schema.string(), schema.json()),
    }),
);
/** A named infrastructure dependency declared by a package. */
export type ResourceDescription = schema.Infer<typeof ResourceDescription>;

/** A provisioned resource in a space. */
export const ResourceReference = defineSchema(
    schema.object({
        /** The space the resource lives in. */
        scope: identifier("space"),
        /** The persistent resource identifier. */
        id: ResourceId,
    }),
);
/** A provisioned resource in a space. */
export type ResourceReference = schema.Infer<typeof ResourceReference>;

/** An inert declaration with access to a host-bound client. */
export class ResourceDeclaration<
    Client,
    Declaration extends ResourceDescription = ResourceDescription,
> extends ResourceHandle<Client> {
    /** The resource kind. */
    readonly kind: Declaration["kind"];
    /** The domain specification. */
    readonly spec: Declaration["spec"];

    /** Retain validated metadata without opening a resource. */
    constructor(owner: Package, declaration: Declaration) {
        // retain the declared kind and spec
        super(owner, declaration.name);
        this.kind = declaration.kind;
        this.spec = declaration.spec;
    }

    /** The connectors opening clients of this kind's resources on the running runtime, by provider code. */
    get connectors(): Readonly<Record<string, Connector<Client>>> {
        return {};
    }

    /** Describe the state the resource must hold, empty for resources that hold none. */
    state(): ResourceState {
        return {};
    }

    /** Serialise the declaration as its declaring package, name, kind and spec. */
    toJSON() {
        return { package: this.package, name: this.name, kind: this.kind, spec: this.spec };
    }
}
