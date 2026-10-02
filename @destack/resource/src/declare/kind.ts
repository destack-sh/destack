import { defineSchema, schema, type JsonValue } from "@destack/schema";
import { DeclarationName } from "@destack/package";
import type { ResourceState } from "@destack/package/declare";
import type { ResourceRecord } from "../provider/provider.ts";
import { ResourceDescription } from "./declaration.ts";

/** The desired state a kind's providers reconcile resources toward, none for kinds without one. */
export type KindState<Kind extends ResourceKind> = Kind["state"] extends schema.Schema
    ? schema.Infer<Kind["state"]>
    : never;

/** A kind of resource: its specification, and the desired state its providers reconcile, if any. */
export class ResourceKind<
    Name extends string = string,
    Spec extends schema.Schema<JsonValue> = schema.Schema<JsonValue>,
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
        // require a declaration name and retain the schemas
        DeclarationName.parse(name);
        this.name = name;
        this.spec = spec;
        this.state = state;

        // describe declarations of the kind
        this.description = defineSchema(
            ResourceDescription.extend({ kind: schema.literal(name), spec }),
        );
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

    /** Read stored desired states of this kind, refusing invalid ones, and any but empty ones of a kind without a state. */
    states(stored: readonly ResourceState[]): KindState<this>[];
    /** Parse each stored state by the kind's state schema, whose parsed value is the kind's state type. */
    states(stored: readonly ResourceState[]): unknown[] {
        // read none for a kind without a state, whose bindings require nothing
        const state = this.state;
        if (state === undefined) {
            if (stored.some((entry) => Object.keys(entry).length > 0)) {
                throw new TypeError(`resources of kind ${this.name} hold no desired state`);
            }

            return [];
        }

        return stored.map((entry) => state.parse(entry));
    }
}

/** Define a resource kind without a desired state. */
export function defineResourceKind<
    const Name extends string,
    Spec extends schema.Schema<JsonValue>,
>(name: Name, options: { readonly spec: Spec }): ResourceKind<Name, Spec, undefined>;
/** Define a resource kind with the desired state its providers reconcile. */
export function defineResourceKind<
    const Name extends string,
    Spec extends schema.Schema<JsonValue>,
    State extends schema.Schema,
>(
    name: Name,
    options: { readonly spec: Spec; readonly state: State },
): ResourceKind<Name, Spec, State>;
/** Define a resource kind, with the desired state its providers reconcile or without one. */
export function defineResourceKind(
    name: string,
    options: { readonly spec: schema.Schema<JsonValue>; readonly state?: schema.Schema },
): ResourceKind {
    return new ResourceKind(name, options.spec, options.state);
}
