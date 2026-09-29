import type { Identifier, schema } from "@destack/schema";
import type { ResourceState } from "@destack/package/declare";
import type { Chunk, Copy } from "./copy.ts";
import type { Plan } from "./plan.ts";
import type { Resource, ResourceKind } from "./resource.ts";

/** A resource as its space stores it. */
export interface ResourceRecord {
    /** The persistent resource identifier. */
    readonly id: Identifier<"resource">;
    /** The space the resource lives in. */
    readonly scope: Identifier<"space">;
    /** The resource kind, such as database. */
    readonly kind: string;
    /** The declared specification. */
    readonly spec: Readonly<Record<string, unknown>>;
    /** The provider's reference once provisioned. */
    readonly reference: string | null;
}

/** What a host lends the providers it opens for one workload. */
export interface ProviderContext {
    /** Read the credential the workload's calls through its host carry. */
    credential(): string;
}

/** Where a provider placed a resource. */
export interface Provision {
    /** The provider's stable reference, such as a file URL or bucket name. */
    readonly reference: string;
    /** The provider location, when meaningful. */
    readonly location?: string;
}

/** A technology providing resources of one kind on a host: it connects them, and has the capabilities its kind and technology allow. */
export type Provider<
    Client = unknown,
    Kind extends ResourceKind = ResourceKind,
    Facet = never,
> = Connector<Client, Kind, Facet> &
    (Kind["state"] extends schema.Schema ? Reconciling : unknown) &
    (Provisioning | { readonly provision?: never; readonly destroy?: never }) &
    (Copying | { readonly export?: never; readonly import?: never });

/** The part of every provider that opens clients. */
interface Connector<Client, Kind extends ResourceKind, Facet> {
    /** The resource kind provided. */
    readonly kind: Kind["name"];
    /** The provider code recorded on bindings, such as sqlite. */
    readonly code: string;
    /** The object type sharing the resource's identity in its space, such as a vault. */
    readonly facet?: Facet;
    /** Open a client for a resource holding the declaration's desired state, refusing otherwise. */
    connect(record: ResourceRecord, declaration: Resource<Client>): Promise<Client>;
}

/** Take a resource to the desired state its kind declares, required of providers of kinds with a state. */
export interface Reconciling {
    /** Plan the steps taking the resource to the union of the desired states. */
    plan(record: ResourceRecord, desired: readonly ResourceState[]): Promise<Plan>;
    /** Apply the plan with the reviewed digest, refusing when the resource or desired states changed. */
    apply(record: ResourceRecord, desired: readonly ResourceState[], digest: string): Promise<void>;
}

/** Create and remove resources the provider hosts. */
export interface Provisioning {
    /** Create or confirm the resource, returning the same reference each time. */
    provision(record: ResourceRecord): Promise<Provision>;
    /** Destroy the resource and everything it stores. */
    destroy(record: ResourceRecord): Promise<void>;
}

/** Move a resource's content out of and into the provider. */
export interface Copying {
    /**
     * Read the source's content after a cursor as chunks, repeatably, ending once the stage's content is read.
     *
     * The live stage reads what the source holds now; the fenced stage also reads what changed since and what only a fenced source keeps still.
     */
    export(copy: Copy, after: string | undefined, signal: AbortSignal): AsyncIterable<Chunk>;
    /** Write one exported chunk into the target's resource, idempotently, so that a repeated export converges. */
    import(copy: Copy, chunk: Chunk): Promise<void>;
}

/** The capabilities of providers. */
export const Provider = {
    /** Report whether a provider reconciles its resources toward a desired state. */
    reconciles<Value extends Provider<unknown, ResourceKind, unknown>>(
        provider: Value,
    ): provider is Value & Reconciling {
        return "plan" in provider && "apply" in provider;
    },

    /** Report whether a provider creates and removes the resources it hosts. */
    provisions<Value extends Provider<unknown, ResourceKind, unknown>>(
        provider: Value,
    ): provider is Value & Provisioning {
        return provider.provision !== undefined && provider.destroy !== undefined;
    },

    /** Report whether a provider moves its resources' content out and in. */
    copies<Value extends Provider<unknown, ResourceKind, unknown>>(
        provider: Value,
    ): provider is Value & Copying {
        return provider.export !== undefined && provider.import !== undefined;
    },
};
