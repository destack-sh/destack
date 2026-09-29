import { defineSchema, identifier, schema } from "@destack/schema";
import { DeclarationName, type Package } from "@destack/package";
import { ResourceHandle } from "./handle.ts";
import type { Provider, ProviderContext } from "./provider.ts";
import type { ResourceState } from "@destack/package/declare";

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
        id: identifier("resource"),
    }),
);
/** A provisioned resource in a space. */
export type ResourceReference = schema.Infer<typeof ResourceReference>;

/** An inert declaration with access to a host-bound client. */
export class Resource<
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

    /** Open the providers holding resources of this kind on the running runtime, by provider code. */
    get providers(): Readonly<
        Record<string, (reference: URL, context: ProviderContext) => Promise<Provider>>
    > {
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

/** A kind of resource: its specification, and the desired state its providers reconcile, if any. */
export class ResourceKind<
    Name extends string = string,
    Spec extends schema.Schema = schema.Schema,
    State extends schema.Schema | undefined = schema.Schema | undefined,
> {
    /** The kind's name, such as database. */
    readonly name: Name;
    /** The schema of a declaration's specification. */
    readonly spec: Spec;
    /** The schema of the desired state its providers reconcile resources toward, absent for kinds without one. */
    readonly state: State;
    /** The schema of a declaration of this kind. */
    readonly description;

    /** Define the kind. */
    constructor(name: Name, spec: Spec, state: State) {
        // retain the schemas
        this.name = DeclarationName.parse(name) as Name;
        this.spec = spec;
        this.state = state;

        // describe declarations of the kind
        this.description = defineSchema(
            ResourceDescription.extend({ kind: schema.literal(name), spec }),
        );
    }
}

/** Define a resource kind, with the desired state its providers reconcile or without one. */
export function defineResourceKind<
    const Name extends string,
    Spec extends schema.Schema,
    State extends schema.Schema | undefined = undefined,
>(
    name: Name,
    options: { readonly spec: Spec; readonly state?: State },
): ResourceKind<Name, Spec, State> {
    return new ResourceKind(name, options.spec, options.state as State);
}
