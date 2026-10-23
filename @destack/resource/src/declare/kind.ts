import { schema, type JsonValue } from "@destack/schema";
import { DeclarationName, ModuleMetadata, type Package } from "@destack/package";
import type { ResourceState } from "@destack/package/declare";
import type { ResourceRecord } from "../provider/provider.ts";
import { ResourceDescription } from "./declaration.ts";

/** The state of a kind without a desired state: the empty object every declaration reports. */
export const NO_STATE = schema.object({});

/** Whether Destack creates a kind's resources, or they live outside it, as third-party APIs do. */
export const ResourceManagement = schema.enum(["internal", "external"]);
/** Whether Destack creates a kind's resources, or they live outside it. */
export type ResourceManagement = schema.Infer<typeof ResourceManagement>;

/** The desired state a kind's providers reconcile resources toward, empty for kinds without one. */
export type KindState<Kind extends ResourceKind> = schema.Output<Kind["state"]>;

/** A kind of resource: its specification, and the desired state its providers reconcile, if any. */
export class ResourceKind<
    Name extends string = string,
    Spec extends schema.Schema<JsonValue> = schema.Schema<JsonValue>,
    State extends schema.Schema = schema.Schema,
> {
    /** The declaring package. */
    readonly package: Package;
    /** The kind's name, such as database. */
    readonly name: Name;
    /** The schema of a declaration's specification. */
    readonly spec: Spec;
    /** The schema of the desired state its providers reconcile toward, empty without one. */
    readonly state: State;
    /** Whether Destack creates its resources, or they live outside it. */
    readonly management: ResourceManagement;
    /** The schema of a declaration of this kind. */
    readonly description;

    /** Define the kind. */
    constructor(
        owner: Package,
        name: Name,
        spec: Spec,
        state: State,
        management: ResourceManagement,
    ) {
        // require a declaration name and retain the schemas
        DeclarationName.parse(name);
        this.package = owner;
        this.name = name;
        this.spec = spec;
        this.state = state;
        this.management = management;

        // describe declarations of the kind
        this.description = ResourceDescription.extend({ kind: schema.literal(name), spec });
    }

    /** Read a stored resource of this kind, refusing an invalid specification. */
    record(
        stored: Omit<ResourceRecord, "spec"> & { readonly spec: unknown },
    ): ResourceRecord<this> {
        return {
            id: stored.id,
            scope: stored.scope,
            spec: this.spec.parse(stored.spec),
            reference: stored.reference,
        };
    }

    /** Whether the kind's providers reconcile toward a desired state. */
    get isStateful(): boolean {
        const state: schema.Schema = this.state;

        return state !== NO_STATE;
    }

    /** Read stored desired states of this kind, refusing invalid ones. */
    states(stored: readonly ResourceState[]): schema.Output<State>[] {
        return stored.map((entry) => this.state.parse(entry));
    }
}

/** Define a resource kind: its specification, desired state, and whether Destack creates it. */
export function defineResourceKind<
    const Name extends string,
    Spec extends schema.Schema<JsonValue>,
    State extends schema.Schema = typeof NO_STATE,
>(
    name: Name,
    options: {
        readonly spec: Spec;
        readonly state?: State;
        readonly management?: ResourceManagement;
    },
    module?: ModuleMetadata,
): ResourceKind<Name, Spec, State | typeof NO_STATE> {
    const owner = ModuleMetadata.require(module, "defineResourceKind").package;

    return new ResourceKind(
        owner,
        name,
        options.spec,
        options.state ?? NO_STATE,
        options.management ?? "internal",
    );
}
