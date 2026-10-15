import { defineSchema, type Digest, schema } from "@destack/schema";
import { ResourceId, type ResourceDeclaration } from "../declare/declaration.ts";
import type { KindState, ResourceKind } from "../declare/kind.ts";
import type { Plan } from "../plan/plan.ts";
import type { Recipient } from "@destack/identity";

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

/** Resources as their spaces record them. */
export const ResourceRecord = {
    /** Read a provisioned resource's reference, refusing an unprovisioned one. */
    reference(record: Pick<ResourceRecord, "reference">): string {
        if (record.reference === null) {
            throw new TypeError("resource is not provisioned");
        }

        return record.reference;
    },
};

/** A provisioned resource a host binds to a workload, as the workload's connector opens it. */
export const ResourceBinding = defineSchema(
    schema.object({
        /** The bound resource's identifier, which its kind's connector reads. */
        resource: schema.string().min(1),
        /** The resource kind, such as database. */
        kind: schema.string().min(1),
        /** The provider that keeps the resource, such as sqlite. */
        provider: schema.string().min(1),
        /** The provider's reference, such as a file URL or an egress URL. */
        reference: schema.string().min(1),
        /** The credential the host lends the workload for the resource. */
        credential: schema.string().min(1).exactOptional(),
        /** The space keeping a bound object its service serves, such as a secret. */
        scope: schema.identifier("space").exactOptional(),
        /** The version of the bound object the deployment captured, such as a secret's. */
        version: schema.number().int().positive().exactOptional(),
    }),
);
/** A provisioned resource a host binds to a workload, as the workload's connector opens it. */
export type ResourceBinding = schema.Infer<typeof ResourceBinding>;

/** Opens clients of one kind's resources inside a workload. */
export interface Connector<
    Client = unknown,
    Declaration extends ResourceDeclaration<Client> = ResourceDeclaration<Client>,
> {
    /** The provider code the connector connects to, such as sqlite. */
    readonly code: string;
    /** Open a client for a bound resource with the declaration's desired state. */
    connect(binding: ResourceBinding, declaration: Declaration): Promise<Client & AsyncDisposable>;
}

/** Where a provider placed a resource. */
export interface Placement {
    /** The provider's stable reference, such as a file URL or bucket name. */
    readonly reference: string;
    /** The provider location, when meaningful. */
    readonly location?: string;
}

/** A technology a host manages one kind's resources with. */
export type Provider<
    Kind extends ResourceKind = ResourceKind,
    Object = unknown,
    Handle = never,
    Table = never,
    Row extends object = never,
    Controller = never,
> = {
    /** The resource kind managed. */
    readonly kind: Kind;
    /** The provider code recorded on resources and bindings, such as sqlite. */
    readonly code: string;
    /** The object type the kind's resources are in their space, such as a vault. */
    readonly object: Object;
    /** Reconcile resources toward their kind's desired state. */
    readonly reconcile?: Reconciler<Kind>;
    /** Create and remove the resources the provider hosts. */
    readonly provision?: Provisioner<Kind>;
    /** Open the resources' content as a database. */
    readonly open?: Opener<Kind, Handle>;
    /** Rewrap the rows the provider binds to its host for another host. */
    readonly rewrap?: Rewrapper<Table, Row>;
    /** Refuse writes into the resources' content while a transfer copies it. */
    readonly fence?: Fence<Kind>;
    /** Copy the resources' content at a point in time into a content-addressed store. */
    readonly snapshot?: Snapshotter<Kind>;
    /** The controllers the host runs beside the provider, such as a sweep of resource content. */
    readonly controllers?: readonly Controller[];
} & (unknown extends KindState<Kind>
    ? unknown
    : KindState<Kind> extends { readonly [Key in string]: never }
      ? { readonly reconcile?: never }
      : { readonly reconcile: Reconciler<Kind> });

/** Take a resource to the desired states its kind declares. */
export interface Reconciler<Kind extends ResourceKind = ResourceKind> {
    /** Plan the steps taking the resource to the union of the desired states. */
    plan(record: ResourceRecord<Kind>, desired: readonly KindState<Kind>[]): Promise<Plan>;
    /** Apply the plan with the reviewed digest. */
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

/** A content-addressed store of blobs, such as snapshots' pages. */
export interface ContentStore {
    /** Read a blob's bytes. */
    readonly read: (digest: Digest) => AsyncIterable<Uint8Array>;
    /** Keep bytes under their digest and return it. */
    readonly write: (body: AsyncIterable<Uint8Array>, expected?: Digest) => Promise<Digest>;
    /** Keep the blobs the store lacks among some digests, read from another store. */
    readonly fetch: (
        digests: readonly Digest[],
        source: Pick<ContentStore, "read">,
    ) => Promise<void>;
    /** Delete the blobs outside a retained set last used before a moment. */
    readonly sweep: (retained: ReadonlySet<Digest>, before: Date) => Promise<void>;
}

/** Copy a resource's content at a point in time into a content-addressed store. */
export interface Snapshotter<Kind extends ResourceKind = ResourceKind> {
    /** Copy the resource's content into the store, and answer the digest of the copy. */
    snapshot(
        record: ResourceRecord<Kind>,
        desired: readonly KindState<Kind>[],
        store: ContentStore,
        recipient?: Recipient,
    ): Promise<Digest>;
    /** Restore the copy a digest identifies into a provisioned resource. */
    restore(
        record: ResourceRecord<Kind>,
        desired: readonly KindState<Kind>[],
        digest: Digest,
        store: ContentStore,
        recipient?: Recipient,
        base?: Digest,
    ): Promise<void>;
}

/** Refuse writes into a resource's content while a transfer copies it. */
export interface Fence<Kind extends ResourceKind = ResourceKind> {
    /** Refuse every write into the resource's content until lifted. */
    fence(record: ResourceRecord<Kind>): Promise<boolean>;
    /** Accept writes into the resource's content again. */
    lift(record: ResourceRecord<Kind>): Promise<void>;
}

/** Rewrap the rows a provider binds to its host for another host. */
export interface Rewrapper<Table, Row extends object> {
    /** The table of the host-bound rows. */
    readonly table: Table;
    /** Seal a row's host-bound values to the target's recipient. */
    seal(row: Readonly<Row>, recipient: Recipient): Promise<Row>;
    /** Open a row sealed to this host's recipient. */
    open(row: Readonly<Row>, recipient: Recipient): Promise<Row>;
}
