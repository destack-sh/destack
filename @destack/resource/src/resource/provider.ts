import type { Identifier } from "@destack/schema";
import type { ResourceState } from "@destack/package/declare";
import type { Plan } from "./plan.ts";
import type { Resource } from "./resource.ts";

/** A resource as its space stores it. */
export interface ResourceRecord {
    /** The persistent resource identifier. */
    readonly id: Identifier<"resource">;
    /** The space containing the resource. */
    readonly spaceId: Identifier<"space">;
    /** The resource kind, such as database. */
    readonly kind: string;
    /** The declared specification. */
    readonly spec: Readonly<Record<string, unknown>>;
    /** The provider's reference once provisioned. */
    readonly reference: string | null;
}

/** Where a provider placed a resource. */
export interface Provision {
    /** The provider's stable reference, such as a file URL or bucket name. */
    readonly reference: string;
    /** The provider location, when meaningful. */
    readonly location?: string;
}

/** Infrastructure on one host for resources of one kind. */
export interface Provider<Client = unknown> {
    /** The resource kind provided. */
    readonly kind: string;
    /** The provider code recorded on provisioned resources, such as sqlite. */
    readonly code: string;
    /** Create or confirm the resource, returning the same reference each time. */
    provision(record: ResourceRecord): Promise<Provision>;
    /** Plan the steps taking the resource to the union of the desired states. */
    plan(record: ResourceRecord, desired: readonly ResourceState[]): Promise<Plan>;
    /** Apply the plan with the reviewed digest, refusing when the resource or desired states changed. */
    apply(record: ResourceRecord, desired: readonly ResourceState[], digest: string): Promise<void>;
    /** Open a client for a resource holding the declaration's desired state, refusing otherwise. */
    connect(record: ResourceRecord, declaration: Resource<Client>): Promise<Client>;
    /** Destroy the resource and everything it stores. */
    destroy(record: ResourceRecord): Promise<void>;
}
