import { space } from "@destack/account/object";
import type { Controller, DatabaseHandle, Row, Table } from "@destack/db";
import type { Bindable, MethodContext, ObjectType, ResourceType } from "@destack/object";
import type { AccessContext } from "@destack/access";
import type { ObjectStore, PackageImplementation } from "@destack/object/server";
import type { Kind, Provisioning } from "@destack/package";
import type { Provider } from "@destack/resource";
import type { JsonObject, schema } from "@destack/schema";
import { StackError } from "../error/index.ts";
import type { Binding } from "./binding.ts";
import type { InstallationRollout, ResourceNeed } from "./rollout.ts";

/** A provider of a space's resources. */
export type SpaceProvider = Provider<
    Kind<schema.Schema, Provisioning>,
    ResourceType<typeof space>,
    DatabaseHandle,
    Table,
    Row,
    Controller
>;

/** A provider whose resources in-process installations open through the space's store. */
export type StoreProvider<Client = unknown> = Pick<SpaceProvider, "kind" | "code"> & {
    /** Connect an in-process installation to a binding's resource. */
    readonly connect: (
        store: Promise<ObjectStore>,
        row: InstallationRollout,
        bound: Binding,
    ) => Promise<Client>;
};

/** The providers the servers a machine runs serve their resources through. */
export const StoreProvider = {
    /** Read the providers some servers declare that installations open through the store. */
    of(servers: readonly PackageImplementation[]): StoreProvider[] {
        return servers
            .flatMap((server) => server.providers)
            .filter(
                (provider): provider is typeof provider & StoreProvider => "connect" in provider,
            );
    },
};

/** A kind of object a package serves to workloads, such as secrets. */
export interface ObjectBindable<
    Object extends ObjectType = ObjectType,
    Client = unknown,
> extends Bindable<Object> {
    /** The provider workloads read the bound objects through, such as vault. */
    readonly provider: string;
    /** The address workloads call the package serving the bound objects at. */
    readonly reference: string;
    /** Read the record an installation's own need declares, none while a binding it needs lacks. */
    readonly declare: (need: ResourceNeed, bindings: readonly Binding[]) => JsonObject | undefined;
    /** Report whether a bound target is ready to run with, reading it alone. */
    readonly ready: (
        context: MethodContext,
        bound: Binding,
        need: ResourceNeed,
    ) => Promise<boolean>;
    /** Start making a bound target ready to capture once its parent is provisioned. */
    readonly prepare: (
        context: MethodContext,
        declarer: AccessContext,
        bound: Binding,
        need: ResourceNeed,
    ) => Promise<void>;
    /** Connect an in-process installation to a binding's target at its bound version. */
    readonly connect: (
        store: Promise<ObjectStore>,
        row: InstallationRollout,
        bound: Binding,
    ) => Promise<Client>;
}

/** The kinds of objects the servers a machine runs serve to workloads. */
export const ObjectBindable = {
    /** Read the object kinds some servers declare, refusing one without a connection. */
    of(servers: readonly PackageImplementation[]): ObjectBindable[] {
        return servers
            .flatMap((server) => server.bindables)
            .map((bindable) => {
                // refuse a kind without a connection
                if (!ObjectBindable.is(bindable)) {
                    throw StackError.of(
                        "BINDABLE_UNSUPPORTED",
                        `bindable ${bindable.object.name} without a connection is not supported`,
                    );
                }

                return bindable;
            });
    },

    /** Report whether a bindable kind connects and provisions through the store. */
    is(bindable: Bindable): bindable is ObjectBindable {
        return "connect" in bindable && "declare" in bindable;
    },
};
