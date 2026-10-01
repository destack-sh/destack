import { defineSchema, schema, type Identifier } from "@destack/schema";
import { ResourceId } from "./resource.ts";
import type { Chunk, Copy } from "./copy.ts";
import type { Plan } from "./plan.ts";
import type { ResourceDeclaration, ResourceKind } from "./resource.ts";

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

/** The desired state a kind's providers reconcile resources toward, none for kinds without one. */
export type KindState<Kind extends ResourceKind> = Kind["state"] extends schema.Schema
    ? schema.Infer<Kind["state"]>
    : never;

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
export interface Provision {
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
} & (Kind["state"] extends schema.Schema ? Reconciling<Kind> : unknown) &
    (Provisioning<Kind> | { readonly provision?: never; readonly destroy?: never }) &
    (Copying<Kind> | { readonly export?: never; readonly import?: never });

/** Take a resource to the desired states its kind declares, required of providers of kinds with a state. */
export interface Reconciling<Kind extends ResourceKind = ResourceKind> {
    /** Plan the steps taking the resource to the union of the desired states. */
    plan(record: ResourceRecord<Kind>, desired: readonly KindState<Kind>[]): Promise<Plan>;
    /** Apply the plan with the reviewed digest, refusing when the resource or desired states changed. */
    apply(
        record: ResourceRecord<Kind>,
        desired: readonly KindState<Kind>[],
        digest: string,
    ): Promise<void>;
}

/** Create and remove resources the provider hosts. */
export interface Provisioning<Kind extends ResourceKind = ResourceKind> {
    /** Create or confirm the resource, returning the same reference each time. */
    provision(record: ResourceRecord<Kind>): Promise<Provision>;
    /** Destroy the resource and everything it stores. */
    destroy(record: ResourceRecord<Kind>): Promise<void>;
}

/** Move a resource's content out of and into the provider. */
export interface Copying<Kind extends ResourceKind = ResourceKind> {
    /**
     * Read the source's content after a cursor as chunks, repeatably, ending once the stage's content is read.
     *
     * The live stage reads what the source holds now; the fenced stage also reads what changed since and what only a fenced source keeps still.
     */
    export(copy: Copy<Kind>, after: string | undefined, signal: AbortSignal): AsyncIterable<Chunk>;
    /** Write one exported chunk into the target's resource, idempotently, so that a repeated export converges. */
    import(copy: Copy<Kind>, chunk: Chunk): Promise<void>;
}

/** The fields of any provider the capability checks read, whatever its kind and object. */
type ProviderShape = { readonly kind: ResourceKind; readonly code: string };

/** The capabilities of providers. */
export const Provider = {
    /** Report whether a provider reconciles its resources toward a desired state. */
    reconciles<Value extends ProviderShape>(provider: Value): provider is Value & Reconciling {
        return "plan" in provider && "apply" in provider;
    },

    /** Report whether a provider creates and removes the resources it hosts. */
    provisions<Value extends ProviderShape>(provider: Value): provider is Value & Provisioning {
        return "provision" in provider && "destroy" in provider;
    },

    /** Report whether a provider moves its resources' content out and in. */
    copies<Value extends ProviderShape>(provider: Value): provider is Value & Copying {
        return "export" in provider && "import" in provider;
    },
};
