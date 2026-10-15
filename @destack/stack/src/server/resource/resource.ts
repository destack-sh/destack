import { space } from "@destack/account/object";
import { aligned, Digest, type Identifier, present, schema } from "@destack/schema";
import {
    type DatabaseConnection,
    type DatabaseHandle,
    type Row,
    Snapshot,
    type Table,
} from "@destack/db";
import {
    DesiredState,
    type ObjectType,
    Provisioned,
    type ResourceType,
    type ResourceRow,
} from "@destack/object";
import { ResourceController, type ServerDefinition } from "@destack/object/server";
import {
    CONNECTION_PROVIDER,
    type ResourceKind,
    type ResourceRecord,
    type Provider,
    type Rewrapper,
} from "@destack/resource";
import type { ResourceState } from "@destack/package/declare";
import { CallError } from "@destack/runtime/error";
import { StackError } from "../../error/index.ts";
import {
    HttpsKind,
    INSTALLATION_PROVIDER,
    InstallationBindingSpec,
    InstallationKind,
} from "@destack/runtime/declare";
import type { Controller } from "@destack/runtime/control";
import { dependency, https } from "../../object/index.ts";

/** A provider of a space's resources. */
export type SpaceProvider = Provider<
    ResourceKind,
    ResourceType<typeof space>,
    DatabaseHandle,
    Table,
    Row,
    Controller
>;

/** The provider of installations' dependencies, connecting over HTTP at their addresses. */
export const installationProvider = {
    kind: InstallationKind,
    code: INSTALLATION_PROVIDER,
    object: dependency,
    provision: {
        provision: async (resource) => {
            // connect to the binding's address
            const { address } = InstallationBindingSpec.parse(resource.spec);
            if (address === undefined) {
                throw new StackError(
                    "INVALID_DEFINITION",
                    `dependency ${resource.id} has no address: bind it in the stack`,
                );
            }

            return { reference: address };
        },
        destroy: async () => {},
    },
} satisfies Provider<typeof InstallationKind, ResourceType<typeof space>>;

/** The provider of HTTPS APIs. */
export const httpsProvider = {
    kind: HttpsKind,
    code: CONNECTION_PROVIDER,
    object: https,
} satisfies Provider<typeof HttpsKind, ResourceType<typeof space>>;

/** A provider that snapshots its resources. */
export type SnapshotProvider = SpaceProvider & {
    readonly snapshot: NonNullable<SpaceProvider["snapshot"]>;
};

/** A provider that provisions and snapshots its resources, restoring copies here. */
export type RestoreProvider = SnapshotProvider & {
    readonly provision: NonNullable<SpaceProvider["provision"]>;
};

/** A resource of a space: its row and its kind's object type. */
export interface Resource {
    /** The kind's object type. */
    readonly object: ResourceType<typeof space>;
    /** The resource's row. */
    readonly row: ResourceRow;
}

/** A machine's providers by resource kind, in preference order. */
export class ProviderIndex {
    /** The machine serving the providers' resources. */
    readonly machine: Identifier<"machine">;
    /** The providers by kind, in preference order. */
    readonly byKind: ReadonlyMap<string, readonly SpaceProvider[]>;
    /** The resource types the space's other packages provision. */
    readonly packaged: readonly ResourceType<typeof space>[];

    /** Index the machine's and the other packages' providers by kind, in order. */
    constructor(
        machine: Identifier<"machine">,
        providers: readonly SpaceProvider[],
        packages: readonly {
            readonly objects: readonly ObjectType[];
            readonly providers?: ServerDefinition["providers"];
        }[] = [],
    ) {
        // take the other packages' provisioned objects and their providers
        const packaged = packages
            .flatMap((entry) => entry.objects)
            .filter((type) => Provisioned.is(type));
        const others = packages.flatMap((entry) =>
            (entry.providers ?? []).map((provider): SpaceProvider => {
                const object = present(
                    packaged.find((type) => type.same(provider.object)),
                    `the object of provider ${provider.code}`,
                );
                const { kind, code, provision, reconcile, fence, snapshot } = provider;

                return {
                    kind,
                    code,
                    object,
                    ...(provision === undefined ? {} : { provision }),
                    ...(reconcile === undefined ? {} : { reconcile }),
                    ...(fence === undefined ? {} : { fence }),
                    ...(snapshot === undefined ? {} : { snapshot }),
                };
            }),
        );

        // group the providers by kind and refuse two serving one kind's objects differently
        const byKind = new Map<string, SpaceProvider[]>();
        for (const provider of [...providers, ...others]) {
            const grouped = byKind.get(provider.kind.name) ?? [];
            const [first] = grouped;
            if (first !== undefined && first.object !== provider.object) {
                throw new TypeError(
                    `providers ${first.code} and ${provider.code} serve ${provider.kind.name} resources differently`,
                );
            }
            byKind.set(provider.kind.name, [...grouped, provider]);
        }

        // keep the machine, its providers and the other packages' objects
        this.machine = machine;
        this.byKind = byKind;
        this.packaged = packaged;
    }

    /** List the object each kind the machine itself serves is, once each. */
    provided(): readonly ResourceType<typeof space>[] {
        return [
            ...new Set([...this.byKind.values()].map((providers) => aligned(providers, 0).object)),
        ].filter((object) => !this.packaged.includes(object));
    }

    /** List the object of every kind the spaces use. */
    objects(): readonly ResourceType<typeof space>[] {
        return [...this.provided(), ...this.packaged];
    }

    /** Find the providers of the kind whose resources an object type is. */
    of(object: ObjectType): readonly SpaceProvider[] {
        return (
            [...this.byKind.values()].find((providers) =>
                aligned(providers, 0).object.same(object),
            ) ?? []
        );
    }

    /** Read the kind whose resources an object type is, refusing an object of no kind. */
    kind(object: ObjectType): ResourceKind {
        return present(object.provisioned, `the kind of ${object.name}`).kind;
    }

    /** Find the provider under a code keeping an object's resources. */
    snapshotting(object: ObjectType, code: string | null): SnapshotProvider {
        const provider = this.of(object).find((entry) => entry.code === code);
        if (provider?.snapshot === undefined) {
            throw new TypeError(`provider ${code} of ${object.name} snapshots nothing`);
        }

        return { ...provider, snapshot: provider.snapshot };
    }

    /** Select a provider here restoring an object's resources. */
    restoring(object: ObjectType, row: ResourceRow): RestoreProvider {
        const provider = this.select(object, { ...row, provider: null });
        if (provider?.provision === undefined || provider.snapshot === undefined) {
            throw new TypeError(`no provider of ${object.name} here restores it`);
        }

        return { ...provider, provision: provider.provision, snapshot: provider.snapshot };
    }

    /** Select the provider a resource requests, or the first of its kind. */
    select(object: ObjectType, row: ResourceRow): SpaceProvider | undefined {
        return ResourceController.select(this.of(object), row);
    }

    /** Destroy a resource through the provider of its kind under a code. */
    async destroy(
        code: string,
        stored: Omit<ResourceRecord, "spec"> & { readonly spec: unknown },
    ): Promise<void> {
        const object = present(
            this.objects().find((type) => type.identifies(stored.id)),
            `the kind of resource ${stored.id}`,
        );
        const provider = present(
            this.of(object).find((entry) => entry.code === code),
            `provider ${code} of resource ${stored.id}`,
        );
        await provider.provision?.destroy(provider.kind.record(stored));
    }

    /** List the rewrapping of the providers moving machine-bound rows to another machine. */
    machineBound(): Rewrapper<Table, Row>[] {
        return [...this.byKind.values()].flat().flatMap((provider) => provider.rewrap ?? []);
    }

    /** Find the kind's object storing a resource in the space's database. */
    async find(
        reader: Pick<Snapshot, "read">,
        scope: Identifier<"space">,
        id: string,
    ): Promise<Resource | undefined> {
        // pick the kind its identifier has
        const object = this.objects().find((each) => each.identifies(id));
        if (object === undefined) {
            return undefined;
        }

        // read the resource in the space
        const row = await reader.read(object.table).findFirst({
            where: {
                id: schema.identifier(object.identity).parse(id),
            },
        });

        return row === undefined || row.scope !== scope ? undefined : { object, row };
    }
}

/** Read the desired states a resource applied, refusing a resource that has not applied them. */
export async function appliedStates(
    database: DatabaseConnection,
    row: ResourceRow,
): Promise<ResourceState[]> {
    // require the recorded digest of its desired states and block a transfer while a plan waits for approval
    const desired = [...row.states, ...row.drainingStates];
    if (row.appliedState !== (await Digest.json(desired)) || row.plan !== null) {
        throw new CallError("PRECONDITION_FAILED", {
            message: `resource ${row.id} has not applied its desired states: approve or reject its plan first`,
        });
    }

    return DesiredState.read(Snapshot.live(database), row.scope, desired);
}
