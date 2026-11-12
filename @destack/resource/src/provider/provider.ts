import type { Environment } from "@destack/environment";
import { type Digest, type JsonValue, schema } from "@destack/schema";
import { ResourceId, type ResourceDeclaration } from "../declare/declaration.ts";
import type { Kind, Provisioning } from "@destack/package";
import type { Plan } from "../plan/plan.ts";
import type { Recipient } from "@destack/identity";
import { ResourceError } from "../error/index.ts";

/** A resource as its space records it, its specification read by its kind. */
export interface ResourceRecord<
    Provisioned extends Kind<schema.Schema, Provisioning> = Kind<schema.Schema, Provisioning>,
> {
    /** The persistent resource identifier. */
    readonly id: ResourceId;
    /** The scope the resource lives in, such as its space. */
    readonly scope: string;
    /** The declared specification. */
    readonly spec: schema.Infer<Provisioned["provisioned"]["spec"]>;
    /** The provider's reference once provisioned. */
    readonly reference: string | null;
}

/** Resources as their spaces record them. */
export const ResourceRecord = {
    /** Read a stored resource of a kind, refusing an invalid specification. */
    of<Spec extends schema.Schema<JsonValue>>(
        kind: Kind<schema.Schema, Provisioning<Spec>>,
        stored: Omit<ResourceRecord, "spec"> & { readonly spec: unknown },
    ): ResourceRecord<Kind<schema.Schema, Provisioning<Spec>>> {
        return {
            id: stored.id,
            scope: stored.scope,
            spec: kind.provisioned.spec.parse(stored.spec),
            reference: stored.reference,
        };
    },

    /** Read a provisioned resource's reference, refusing an unprovisioned one. */
    reference(record: Pick<ResourceRecord, "reference">): string {
        if (record.reference === null) {
            throw ResourceError.of(
                "RESOURCE_UNAVAILABLE",
                "resource is unavailable: it is not provisioned",
            );
        }

        return record.reference;
    },
};

/** A provisioned resource a host binds to a workload, as the workload's connector opens it. */
const resourceBinding = schema.object({
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
    /** The version of the bound object the deployment runs with, such as a secret's. */
    version: schema.number().int().positive().exactOptional(),
});

/** A provisioned resource a host binds to a workload, as the workload's connector opens it. */
export const ResourceBinding = Object.assign(resourceBinding, {
    /** Read the credential and scope a host lends for a resource, refusing a binding without. */
    lent(binding: ResourceBinding): { readonly credential: string; readonly scope: string } {
        const { credential, scope } = binding;
        if (credential === undefined || scope === undefined) {
            throw ResourceError.of(
                "CREDENTIAL_BROKEN",
                `binding '${binding.resource}' lends no credential and scope`,
            );
        }

        return { credential, scope };
    },
});

/** A provisioned resource a host binds to a workload, as the workload's connector opens it. */
export type ResourceBinding = schema.Infer<typeof resourceBinding>;

/** Opens clients of one kind's resources inside a workload. */
export interface Connector<
    Client = unknown,
    Declaration extends ResourceDeclaration<Client> = ResourceDeclaration<Client>,
> {
    /** The provider code the connector connects to, such as sqlite. */
    readonly code: string;
    /** Open a client for a bound resource with its declaration, in the workload's environment. */
    connect(
        binding: ResourceBinding,
        declaration: Declaration,
        environment: Environment,
    ): Promise<Client & AsyncDisposable>;
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
    Provisioned extends Kind<schema.Schema, Provisioning> = Kind<schema.Schema, Provisioning>,
    Object = unknown,
    Handle = never,
    Table = never,
    Row extends object = never,
    Controller = never,
> = {
    /** The resource kind managed. */
    readonly kind: Provisioned;
    /** The provider code recorded on resources and bindings, such as sqlite. */
    readonly code: string;
    /** The object type the kind's resources are in their space, such as a vault. */
    readonly object: Object;
    /** Reconcile resources toward their kind's desired state. */
    readonly reconcile?: Reconciler<Provisioned>;
    /** Create and remove the resources the provider hosts. */
    readonly provision?: Provisioner<Provisioned>;
    /** Open the resources' content as a database. */
    readonly open?: Opener<Provisioned, Handle>;
    /** Resolve a reference to where a workload on this host reaches the resource. */
    readonly resolve?: (reference: string) => string;
    /** Rewrap the rows the provider binds to its host for another host. */
    readonly rewrap?: Rewrapper<Table, Row>;
    /** Refuse writes into the resources' content while a transfer copies it. */
    readonly fence?: Fence<Provisioned>;
    /** Copy the resources' content at a point in time into a content-addressed store. */
    readonly snapshot?: Snapshotter<Provisioned>;
    /** The controllers the host runs beside the provider, such as a sweep of resource content. */
    readonly controllers?: readonly Controller[];
} & (unknown extends schema.Output<Provisioned["provisioned"]["state"]>
    ? unknown
    : schema.Output<Provisioned["provisioned"]["state"]> extends { readonly [Key in string]: never }
      ? { readonly reconcile?: never }
      : { readonly reconcile: Reconciler<Provisioned> });

/** Take a resource to the desired states its kind declares. */
export interface Reconciler<
    Provisioned extends Kind<schema.Schema, Provisioning> = Kind<schema.Schema, Provisioning>,
> {
    /** Plan the steps taking the resource to the union of the desired states. */
    plan(
        record: ResourceRecord<Provisioned>,
        desired: readonly schema.Output<Provisioned["provisioned"]["state"]>[],
    ): Promise<Plan>;
    /** Apply the plan with the reviewed digest. */
    apply(
        record: ResourceRecord<Provisioned>,
        desired: readonly schema.Output<Provisioned["provisioned"]["state"]>[],
        digest: Digest,
    ): Promise<void>;
}

/** Create and remove resources the provider hosts. */
export interface Provisioner<
    Provisioned extends Kind<schema.Schema, Provisioning> = Kind<schema.Schema, Provisioning>,
> {
    /** Create or confirm the resource, returning the same reference each time. */
    provision(record: ResourceRecord<Provisioned>): Promise<Placement>;
    /** Destroy the resource and everything it stores. */
    destroy(record: ResourceRecord<Provisioned>): Promise<void>;
}

/** Open a resource's content as a database, such as a database resource's file. */
export interface Opener<
    Provisioned extends Kind<schema.Schema, Provisioning> = Kind<schema.Schema, Provisioning>,
    Handle = unknown,
> {
    /** Open the provisioned resource as its desired states describe it. */
    open(
        record: ResourceRecord<Provisioned>,
        desired: readonly schema.Output<Provisioned["provisioned"]["state"]>[],
    ): Promise<Handle>;
}

/** Content-addressed storage of blobs in packs, laid out as the storage decides. */
export interface BlobStorage {
    /** Store bodies read in order in the packs `BlobPack.groups` makes, and place them. */
    readonly write: (bodies: readonly BlobBody[]) => Promise<readonly BlobPlacement[]>;
    /** Read a range of a pack's bytes. */
    readonly slice: (pack: string, offset: number, length: number) => AsyncIterable<Uint8Array>;
    /** Delete a pack, if stored. */
    readonly delete: (pack: string) => Promise<void>;
    /** List the stored packs. */
    readonly packs: () => AsyncIterable<string>;
    /** Delete the unfinished writes interrupted writers left behind, while no writer runs. */
    readonly clean: () => Promise<void>;
}

/** Blobs by digest over a pack storage and its index, as Bazel's content-addressable storage. */
export interface ContentStore {
    /** Read a blob's bytes. */
    readonly read: (digest: Digest) => AsyncIterable<Uint8Array>;
    /** Read many blobs in order, each body read before the next blob's. */
    readonly batch: (digests: readonly Digest[]) => AsyncIterable<BlobEntry>;
    /** Store bytes under their digest, refusing another than an expected one, and answer it. */
    readonly write: (body: AsyncIterable<Uint8Array>, expected?: Digest) => Promise<Digest>;
    /** Store the blobs the store lacks among some digests, read from another store in one pack. */
    readonly fetch: (
        digests: readonly Digest[],
        source: Pick<ContentStore, "batch">,
    ) => Promise<void>;
    /** List the digests the store lacks, in the given order. */
    readonly missing: (digests: readonly Digest[]) => Promise<readonly Digest[]>;
}

/** Where a stored blob's bytes lie: their pack, their offset and their size. */
export interface BlobPlacement {
    /** The blob's digest. */
    readonly digest: Digest;
    /** The key of its bytes' pack: its own digest for a blob stored alone. */
    readonly pack: string;
    /** Where its bytes start in the pack. */
    readonly offset: number;
    /** Its size in bytes. */
    readonly size: number;
}

/** A body to store: its bytes, with the digest and size they must have where known. */
export interface BlobBody {
    /** The bytes, read before the next body's. */
    readonly body: AsyncIterable<Uint8Array>;
    /** The digest the bytes must have, absent when the storage digests them. */
    readonly digest?: Digest;
    /** The size the bytes must have, absent when the storage counts them. */
    readonly size?: number;
}

/** A blob of a batch: its digest, its size and its body, read before the next blob's. */
export interface BlobEntry {
    /** The blob's digest. */
    readonly digest: Digest;
    /** The blob's size in bytes. */
    readonly size: number;
    /** The blob's bytes. */
    readonly body: AsyncIterable<Uint8Array>;
}

/** Copy a resource's content at a point in time into a content-addressed store. */
export interface Snapshotter<
    Provisioned extends Kind<schema.Schema, Provisioning> = Kind<schema.Schema, Provisioning>,
> {
    /** Copy the resource's content into the store, and answer the digest of the copy. */
    snapshot(
        record: ResourceRecord<Provisioned>,
        desired: readonly schema.Output<Provisioned["provisioned"]["state"]>[],
        store: Pick<ContentStore, "read" | "batch" | "write" | "fetch">,
        recipient?: Recipient,
    ): Promise<Digest>;
    /** Restore the copy a digest identifies into a provisioned resource. */
    restore(
        record: ResourceRecord<Provisioned>,
        desired: readonly schema.Output<Provisioned["provisioned"]["state"]>[],
        digest: Digest,
        store: Pick<ContentStore, "read" | "batch" | "write" | "fetch">,
        recipient?: Recipient,
        base?: Digest,
    ): Promise<void>;
}

/** Refuse writes into a resource's content while a transfer copies it. */
export interface Fence<
    Provisioned extends Kind<schema.Schema, Provisioning> = Kind<schema.Schema, Provisioning>,
> {
    /** Refuse every write into the resource's content until lifted. */
    fence(record: ResourceRecord<Provisioned>): Promise<boolean>;
    /** Accept writes into the resource's content again. */
    lift(record: ResourceRecord<Provisioned>): Promise<void>;
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
