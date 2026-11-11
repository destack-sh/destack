import { Duration, type JsonObject, JsonValue, schema } from "@destack/schema";
import {
    DeclarationName,
    Kind,
    PackageVersion,
    provisionedDescription,
    type Provisioning,
} from "@destack/package";
import type {} from "@destack/package/import-meta";
import type { ResourceState } from "@destack/package/declare";
import type { Connector } from "../provider/provider.ts";
import { ResourceHandle } from "./handle.ts";

/** A resource's identifier, whichever kind it is: the kind's name as prefix and a UUIDv7. */
export const ResourceId = schema.anyIdentifier();
/** A resource's identifier, whichever kind it is. */
export type ResourceId = schema.Infer<typeof ResourceId>;

/** A named infrastructure dependency declared by a package, as its kind describes it. */
export const ResourceDescription = provisionedDescription(schema.jsonObject());
/** A named infrastructure dependency declared by a package. */
export type ResourceDescription = schema.Infer<typeof ResourceDescription>;

/** A resource declaration as its JSON describes it. */
const RESOURCE_DECLARATION_JSON = schema.object({
    /** The declaring package. */
    package: PackageVersion,
    /** The declaration's name. */
    name: DeclarationName,
    /** The kind, as its JSON describes it. */
    kind: schema.json(),
    /** The specification. */
    spec: schema.jsonObject(),
});

/** What deleting a resource does to its content. */
export const ResourceRetention = schema.union([
    schema.literal("delete"),
    schema.literal("forever"),
    schema.object({
        /** How long the deleted resource stays restorable before its content is destroyed. */
        within: Duration.schema,
    }),
]);
/** What deleting a resource does to its content. */
export type ResourceRetention = schema.Infer<typeof ResourceRetention>;

/** Where a resource is asked to live: its provider, and the provider's location and machine. */
export const ResourcePlacement = schema.object({
    /** The provider adapter. */
    provider: schema.string().min(1),
    /** The location code accepted by the provider adapter. */
    location: schema.string().min(1).exactOptional(),
    /** The machine administering the resource. */
    machine: schema.identifier("machine").exactOptional(),
});
/** Where a resource is asked to live. */
export type ResourcePlacement = schema.Infer<typeof ResourcePlacement>;

/** User-defined labels indexed by name. */
export const TagMap = schema.record(schema.string().min(1).max(128), schema.string().max(256));
/** User-defined labels indexed by name. */
export type TagMap = schema.Infer<typeof TagMap>;

/** How a stack declares a resource: its placement, retention and tags. */
export interface ResourceProvisionOptions {
    /** The requested placement. */
    readonly placement?: ResourcePlacement;
    /** What deleting the resource does to its content. */
    readonly retention?: ResourceRetention;
    /** User-defined labels. */
    readonly tags?: TagMap;
}

/** A provisioned resource in a space. */
export const ResourceReference = schema.object({
    /** The space the resource lives in. */
    scope: schema.identifier("space"),
    /** The persistent resource identifier. */
    id: ResourceId,
});
/** A provisioned resource in a space. */
export type ResourceReference = schema.Infer<typeof ResourceReference>;

/** The specification of a bound kind's declarations, such as a secret's: empty. */
export const NO_SPEC = schema.object({});

/** An inert declaration of what a package binds, with access to a host-bound client. */
export class ResourceDeclaration<
    Client,
    Spec extends schema.Schema<JsonValue> = schema.Schema<JsonValue>,
> extends ResourceHandle<Client> {
    /** The resource kind: a provisioned kind, such as bucket, or a bound one, such as secret. */
    readonly kind: Kind<schema.Schema, Provisioning<Spec> | undefined>;
    /** The specification, empty for a bound kind. */
    readonly spec: schema.Output<Spec>;
    /** The connectors opening clients of the resource, by provider code. */
    readonly #connectors: Readonly<Record<string, Connector<Client>>>;

    /** Retain a declaration of a kind with its parsed spec, without opening a resource. */
    constructor(
        owner: PackageVersion,
        kind: Kind<schema.Schema, Provisioning<Spec> | undefined>,
        declaration: {
            readonly name: string;
            readonly spec: schema.Output<Spec>;
            readonly connectors?: Readonly<Record<string, Connector<Client>>>;
        },
    ) {
        // keep the kind, the spec and the connectors
        super(owner, DeclarationName.parse(declaration.name));
        this.kind = kind;
        this.spec = declaration.spec;
        this.#connectors = declaration.connectors ?? {};
    }

    /** Report whether a value is a resource declaration, of any client and kind. */
    static is(value: unknown): value is ResourceDeclaration<unknown> {
        return value instanceof ResourceDeclaration;
    }

    /** The connectors that open clients of this kind's resources on the running runtime. */
    get connectors(): Readonly<Record<string, Connector<Client, this>>> {
        return this.#connectors;
    }

    /** Describe the state the resource must hold, empty for resources that hold none. */
    state(): ResourceState {
        return {};
    }

    /** Describe the declaration as its kind's declarations are described: its name and spec. */
    describe(): ResourceDescription {
        return ResourceDescription.parse({ name: this.name, spec: this.spec });
    }

    /** Describe the create input of a resource of this declaration, as a stack provisions it. */
    provision(options: ResourceProvisionOptions = {}) {
        return {
            definitionPackageId: this.package.id,
            definitionVersion: this.package.version,
            definitionName: this.name,
            spec: this.spec,
            ...options,
        };
    }

    /** Describe the declaration as JSON: its declaring package, name, kind and spec. */
    toJSON(): JsonObject {
        return {
            package: {
                id: this.package.id,
                name: this.package.name,
                version: this.package.version,
            },
            name: this.name,
            kind: this.kind.toJSON(),
            spec: JsonValue.object(this.spec),
        };
    }

    /** Rebuild a declaration its JSON describes, without its connectors. */
    static fromJSON(value: unknown): ResourceDeclaration<unknown> {
        // read the package, name, kind and spec
        const {
            package: owner,
            name,
            kind: written,
            spec,
        } = RESOURCE_DECLARATION_JSON.parse(value);
        const kind = Kind.fromJSON(written);
        const parsed =
            kind.provisioned === undefined
                ? NO_SPEC.parse(spec)
                : kind.provisioned.spec.parse(spec);

        return new ResourceDeclaration(owner, kind, { name, spec: JsonValue.object(parsed) });
    }
}
