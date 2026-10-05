import { defineSchema, type Digest, schema } from "@destack/schema";
import { ResourceId, type ResourceDeclaration } from "../declare/declaration.ts";
import type { KindState, ResourceKind } from "../declare/kind.ts";
import type { Plan } from "../plan/plan.ts";
import type { Recipient } from "./recipient.ts";

/** A resource as its space records it, its specification read by its kind. */
export interface ResourceRecord<Kind extends ResourceKind = ResourceKind> {
    /** The persistent resource identifier. */
    readonly id: ResourceId;
    /** The scope the resource lives in, such as its space. */
    readonly scope: string;
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
        credential: schema.string().min(1).exactOptional(),
        /** The space keeping a bound object its service serves, such as a secret. */
        scope: schema.identifier("space").exactOptional(),
        /** The version of the bound object the deployment captured, such as a secret's. */
        version: schema.number().int().positive().exactOptional(),
    }),
);
/** A provisioned resource a host binds to a workload, as the workload's connector opens it. */
export type ResourceBinding = schema.Infer<typeof ResourceBinding>;

/** Opens clients of one kind's resources inside a workload, as its declaration supplies it for one provider. */
export interface Connector<
    Client = unknown,
    Declaration extends ResourceDeclaration<Client> = ResourceDeclaration<Client>,
> {
    /** The provider code the connector connects to, such as sqlite. */
    readonly code: string;
    /** Open a client the caller disposes for a bound resource with the declaration's desired state, refusing otherwise. */
    connect(binding: ResourceBinding, declaration: Declaration): Promise<Client & AsyncDisposable>;
}

/** Where a provider placed a resource. */
export interface Placement {
    /** The provider's stable reference, such as a file URL or bucket name. */
    readonly reference: string;
    /** The provider location, when meaningful. */
    readonly location?: string;
}

/** A technology a host manages one kind's resources with, having the capabilities its kind and technology allow. */
export type Provider<
    Kind extends ResourceKind = ResourceKind,
    Object = unknown,
    Handle = never,
    Table = never,
    Row extends object = never,
    Controller = never,
    Store = never,
> = {
    /** The resource kind managed. */
    readonly kind: Kind;
    /** The provider code recorded on resources and bindings, such as sqlite. */
    readonly code: string;
    /** The object type the kind's resources are in their space, such as a vault. */
    readonly object: Object;
    /** Reconcile resources toward their kind's desired state, required exactly of providers of kinds with one. */
    readonly reconcile?: Reconciler<Kind>;
    /** Create and remove the resources the provider hosts. */
    readonly provision?: Provisioner<Kind>;
    /** Open the resources' content as a database. */
    readonly open?: Opener<Kind, Handle>;
    /** Rewrap the rows the provider binds to its host for another host. */
    readonly rewrap?: Rewrapper<Table, Row>;
    /** Refuse writes into the resources' content while a transfer copies it. */
    readonly fence?: Fence<Kind>;
    /** Copy the resources' content at a point in time into a content-addressed store, and restore such a copy. */
    readonly snapshot?: Snapshotter<Kind, Store>;
    /** The controllers the host runs beside the provider, such as a sweep of its resources' content. */
    readonly controllers?: readonly Controller[];
} & (Kind["state"] extends schema.Schema
    ? { readonly reconcile: Reconciler<Kind> }
    : Kind["state"] extends undefined
      ? { readonly reconcile?: never }
      : unknown);

/** Take a resource to the desired states its kind declares, required of providers of kinds with a state. */
export interface Reconciler<Kind extends ResourceKind = ResourceKind> {
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
export interface Provisioner<Kind extends ResourceKind = ResourceKind> {
    /** Create or confirm the resource, returning the same reference each time. */
    provision(record: ResourceRecord<Kind>): Promise<Placement>;
    /** Destroy the resource and everything it stores. */
    destroy(record: ResourceRecord<Kind>): Promise<void>;
}

/** Open a resource's content as a database, such as a database resource's file. */
export interface Opener<Kind extends ResourceKind = ResourceKind, Handle = unknown> {
    /** Open the provisioned resource as its desired states describe it. */
    open(record: ResourceRecord<Kind>, desired: readonly KindState<Kind>[]): Promise<Handle>;
}

/** Copy a resource's content at a point in time into a content-addressed store and restore such a copy into another resource of the kind, as a storage driver takes volume snapshots. */
export interface Snapshotter<Kind extends ResourceKind = ResourceKind, Store = unknown> {
    /** Copy the resource's content into the store, wrapping its host-bound values for a recipient when given, and answer the digest naming the copy. */
    snapshot(
        record: ResourceRecord<Kind>,
        desired: readonly KindState<Kind>[],
        store: Store,
        recipient?: Recipient,
    ): Promise<Digest>;
    /** Restore the copy a digest names into a provisioned resource, unwrapping values wrapped for this host's recipient when given, onto the base copy it holds already when given. */
    restore(
        record: ResourceRecord<Kind>,
        desired: readonly KindState<Kind>[],
        digest: Digest,
        store: Store,
        recipient?: Recipient,
        base?: Digest,
    ): Promise<void>;
}

/** Refuse writes into a resource's content while a transfer copies it, as a lease fence refuses a former holder. */
export interface Fence<Kind extends ResourceKind = ResourceKind> {
    /** Refuse every write into the resource's content until lifted, returning once the writes in flight finished, and report whether this call set the fence. */
    fence(record: ResourceRecord<Kind>): Promise<boolean>;
    /** Accept writes into the resource's content again. */
    lift(record: ResourceRecord<Kind>): Promise<void>;
}

/** Rewrap the rows a provider binds to its host, such as keys wrapped under the host's root key, for another host. */
export interface Rewrapper<Table, Row extends object> {
    /** The table of the host-bound rows. */
    readonly table: Table;
    /** Wrap a row's host-bound values for the target's recipient. */
    wrap(row: Readonly<Row>, recipient: Recipient): Promise<Row>;
    /** Unwrap a row wrapped for this host's recipient, wrapping its values under this host's root key. */
    unwrap(row: Readonly<Row>, recipient: Recipient): Promise<Row>;
}
