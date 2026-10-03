import { defineSchema, Duration, schema } from "@destack/schema";
import { DeclarationName, type Package } from "@destack/package";
import type { ResourceState } from "@destack/package/declare";
import type { Connector } from "../provider/provider.ts";
import { ResourceHandle } from "./handle.ts";

/** A resource's identifier, whichever kind it is: the kind's name as prefix and a UUIDv7. */
export const ResourceId = defineSchema(schema.anyIdentifier());
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

/** What deleting a resource does to its content: destroy it, keep it for a window, or keep it until destroyed by hand. */
export const ResourceRetention = defineSchema(
    schema.union([
        schema.literal("delete"),
        schema.literal("forever"),
        schema.object({
            /** How long the deleted resource stays restorable before its content is destroyed. */
            within: Duration.schema,
        }),
    ]),
);
/** What deleting a resource does to its content. */
export type ResourceRetention = schema.Infer<typeof ResourceRetention>;

/** Where a resource is asked to live: its provider, and the provider's location and host. */
export const ResourcePlacement = defineSchema(
    schema.object({
        /** The provider adapter. */
        provider: schema.string().min(1),
        /** The location code accepted by the provider adapter. */
        location: schema.string().min(1).exactOptional(),
        /** The host administering the resource. */
        host: schema.identifier("host").exactOptional(),
    }),
);
/** Where a resource is asked to live. */
export type ResourcePlacement = schema.Infer<typeof ResourcePlacement>;

/** A provisioned resource in a space. */
export const ResourceReference = defineSchema(
    schema.object({
        /** The space the resource lives in. */
        scope: schema.identifier("space"),
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
    get connectors(): Readonly<Record<string, Connector<Client, this>>> {
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
