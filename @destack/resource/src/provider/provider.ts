import { defineSchema, type Digest, schema, type Identifier } from "@destack/schema";
import { ResourceId, type ResourceDeclaration } from "../declare/declaration.ts";
import type { KindState, ResourceKind } from "../declare/kind.ts";
import type { Plan } from "../plan/plan.ts";
import type { Recipient } from "./recipient.ts";

/** A resource as its space records it, its specification read by its kind. */
export interface ResourceRecord<Kind extends ResourceKind = ResourceKind> {
    /** The persistent resource identifier. */
    readonly id: ResourceId;
    /** The space the resource lives in. */
    readonly scope: Identifier<"space">;
    /** The declared specification. */
    readonly spec: schema.Infer<Kind["spec"]>;
    /** The provider's reference once provisioned. */
    readonly reference: string | null;
}

/** A provisioned resource a host binds to a workload, as the workload's connector opens it. */
export const ResourceBinding = defineSchema(
    schema.object({
        /** The resource. */
        resource: ResourceId,
        /** The resource kind, such as database. */
        kind: schema.string().min(1),
        /** The provider holding the resource, whose connector the workload opens it with, such as sqlite. */
        provider: schema.string().min(1),
        /** The provider's reference, such as a file URL or an egress URL. */
        reference: schema.string().min(1),
        /** The credential the host lends the workload for the resource, absent when the reference needs none. */
        credential: schema.string().min(1).optional(),
    }),
);
/** A provisioned resource a host binds to a workload, as the workload's connector opens it. */
export type ResourceBinding = schema.Infer<typeof ResourceBinding>;

/** Opens clients of one kind's resources inside a workload, as its declaration supplies it for one provider. */
export interface Connector<Client = unknown> {
    /** The provider code the connector connects to, such as sqlite. */
    readonly code: string;
    /** Open a client for a bound resource holding the declaration's desired state, refusing otherwise. */
    connect(binding: ResourceBinding, declaration: ResourceDeclaration<Client>): Promise<Client>;
}

/** Where a provider placed a resource. */
export interface Placement {
    /** The provider's stable reference, such as a file URL or bucket name. */
    readonly reference: string;
    /** The provider location, when meaningful. */
    readonly location?: string;
}

/** A technology a host manages one kind's resources with, having the capabilities its kind and technology allow. */
export type Provider<Kind extends ResourceKind = ResourceKind, Object = unknown> = {
    /** The resource kind managed. */
    readonly kind: Kind;
    /** The provider code recorded on resources and bindings, such as sqlite. */
    readonly code: string;
    /** The object type the kind's resources are in their space, such as a vault. */
    readonly object: Object;
} & (Kind["state"] extends schema.Schema ? Reconcile<Kind> : unknown) &
    (Provision<Kind> | { readonly provision?: never; readonly destroy?: never });

/** Take a resource to the desired states its kind declares, required of providers of kinds with a state. */
export interface Reconcile<Kind extends ResourceKind = ResourceKind> {
    /** Plan the steps taking the resource to the union of the desired states. */
    plan(record: ResourceRecord<Kind>, desired: readonly KindState<Kind>[]): Promise<Plan>;
    /** Apply the plan with the reviewed digest, refusing when the resource or desired states changed. */
    apply(
        record: ResourceRecord<Kind>,
        desired: readonly KindState<Kind>[],
        digest: Digest,
    ): Promise<void>;
}

/** Create and remove resources the provider hosts. */
export interface Provision<Kind extends ResourceKind = ResourceKind> {
    /** Create or confirm the resource, returning the same reference each time. */
    provision(record: ResourceRecord<Kind>): Promise<Placement>;
    /** Destroy the resource and everything it stores. */
    destroy(record: ResourceRecord<Kind>): Promise<void>;
}

/** Open a resource's content as a database, such as a database resource's file. */
export interface Open<Kind extends ResourceKind = ResourceKind, Handle = unknown> {
    /** Open the provisioned resource as its desired states describe it. */
    open(record: ResourceRecord<Kind>, desired: readonly KindState<Kind>[]): Promise<Handle>;
}

/** Rewrap the rows a provider binds to its host, such as keys wrapped under the host's root key, for another host. */
export interface Rewrap<Table = unknown, Row extends object = Record<string, unknown>> {
    /** The table of the host-bound rows. */
    readonly table: Table;
    /** Wrap a row's host-bound values for the target's recipient. */
    wrap(row: Readonly<Row>, recipient: Recipient): Promise<Row>;
    /** Unwrap a row wrapped for this host's recipient, wrapping its values under this host's root key. */
    unwrap(row: Readonly<Row>, recipient: Recipient): Promise<Row>;
}

/** The fields of any provider the capability checks read, whatever its kind and object. */
type ProviderShape = { readonly kind: ResourceKind; readonly code: string };

/** The capabilities of providers. */
export const Provider = {
    /** Report whether a provider reconciles its resources toward a desired state. */
    reconciles<Value extends ProviderShape>(provider: Value): provider is Value & Reconcile {
        return "plan" in provider && "apply" in provider;
    },

    /** Report whether a provider opens its resources' content as a database. */
    opens<Value extends ProviderShape>(provider: Value): provider is Value & Open {
        return "open" in provider;
    },

    /** Report whether a provider rewraps host-bound rows for another host. */
    rewraps<Value extends ProviderShape>(provider: Value): provider is Value & Rewrap {
        return "table" in provider && "wrap" in provider && "unwrap" in provider;
    },

    /** Report whether a provider creates and removes the resources it hosts. */
    provisions<Value extends ProviderShape>(provider: Value): provider is Value & Provision {
        return "provision" in provider && "destroy" in provider;
    },
};
