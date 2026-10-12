import { type Policy, principal, type TableMapping } from "@destack/access";
import type { Table } from "@destack/db";
import type { Identifier } from "@destack/schema";
import { Egress, type Service, type ServiceRouter } from "@destack/service";
import type { Router, ServiceContext, ServiceImplementation } from "@destack/service/server";
import type { Shape, Subject, Uplink } from "@destack/sync";
import type { InstallationContext } from "@destack/service/workload";
import { ProjectionParameters } from "../replica/replica.ts";
import type { ObjectType } from "../object/object.ts";
import type { ExternalStorage } from "./external.ts";
import type { ProvisionedOptions } from "./provisioned.ts";
import { ObjectStore, type ObjectStoreOptions, type SubscriberEntry } from "./store.ts";

/** What an extension adds once the space's store exists. */
export interface ObjectExtensionImplementation extends Pick<
    ServiceImplementation,
    "access" | "responseHeaders" | "fetch" | "controllers"
> {
    /** The service's own procedures, routed beside its objects. */
    readonly procedures?: Readonly<Record<string, Router<ServiceRouter, ServiceContext>>>;
}

/** One package's contribution to a space's store. */
export interface ObjectExtension {
    /** The service, whose router reaches its objects on the space's store. */
    readonly service: Service;
    /** The handled types the extension serves, keyed by their names in camel case. */
    readonly objects: Readonly<Record<string, ObjectType>>;
    /** The policies of further types the objects authorize through, such as copied scopes. */
    readonly policies?: readonly (Policy | ObjectType | TableMapping)[];
    /** The providers reconciling the extension's provisioned resources. */
    readonly providers?: ProvisionedOptions["providers"];
    /** The shapes the extension serves and follows beside its objects' own. */
    readonly shapes?: readonly Shape[];
    /** The files with the extension's external objects. */
    readonly external?: ExternalStorage;
    /** The tables whose changes list the space's copies again. */
    readonly watches?: readonly Table[];
    /** List the copies the extension keeps in the space's database, each with its publisher. */
    subscriptions?(store: ObjectStore): Promise<readonly SubscriberEntry[]>;
    /** Serve what needs the space's store: procedures, controllers, access and headers. */
    serve?(store: ObjectStore): ObjectExtensionImplementation;
}

/** What a store of composed extensions runs with beside their objects. */
export type CompositionOptions = Omit<
    ObjectStoreOptions,
    "objects" | "provisioned" | "subscriber" | "external" | "principal" | "installation"
> & {
    /** The link to the followed service, receiving copied rows' changes, else the publisher. */
    readonly uplink?: Uplink;
    /** The machine keeping the extensions' provisioned resources, required by any provider. */
    readonly machine?: Identifier<"machine">;
    /** List the store's extra copies, such as its scope's chain, each with its publisher. */
    readonly subscriptions?: (store: ObjectStore) => Promise<readonly SubscriberEntry[]>;
} & (
        | {
              /** The principal the store's own work runs as. */
              readonly principal: Subject;
              /** No installation: the store runs as the given principal. */
              readonly installation?: never;
          }
        | {
              /** The installation serving the objects, running the store's work and copies. */
              readonly installation: InstallationContext;
              /** No principal: the store runs as the installation. */
              readonly principal?: never;
          }
    );

/** Compose stores from extensions. */
export const ObjectExtension = {
    /** Serve extensions over one store, each routing its own objects under its service. */
    compose,
    /** Take a service as an extension with its objects' handled types, refusing missing ones. */
    of(
        service: Service<ServiceRouter, Readonly<Record<string, ObjectType>>>,
        handled: readonly ObjectType[],
    ): ObjectExtension {
        // refuse a handled type the service does not serve
        const own = service.objects;
        const stray = handled.find((type) => !Object.values(own).some((each) => each.same(type)));
        if (stray !== undefined) {
            throw new TypeError(`service ${service.name} serves no object ${stray.name}`);
        }

        return {
            service,
            objects: Object.fromEntries(
                Object.entries(own).map(([key, type]) => [
                    key,
                    handled.find((each) => each.same(type)) ?? type,
                ]),
            ),
        };
    },
};

/** Send an installation's copied row changes over its publisher, signed as the installation. */
function publisherUplink(installation: InstallationContext): Uplink {
    const { publisher } = installation;

    return {
        stream: (subscription, signal) => publisher.stream(subscription, signal),
        receive: (mutation) => publisher.receive(mutation),
    };
}

/** List an installation's copies. */
async function installationSubscriptions(
    installation: InstallationContext,
    store: ObjectStore,
): Promise<readonly SubscriberEntry[]> {
    return [
        ...(await store.source.subscriptions(installation.scope, { isHome: false })).map(
            (subscription) => ({ subscription, publisher: installation.publisher }),
        ),
        ...(await store.source.projectionSubscriptions(installation.scope)).map((subscription) => {
            const { installation: keeping, service } = ProjectionParameters.parse(
                subscription.parameters,
            );

            return {
                subscription,
                publisher: installation.publisherAt(
                    Egress.address(keeping, subscription.scope),
                    service,
                ),
            };
        }),
    ];
}

/** Follow a partition's chain below an installation's root through its package's object API. */
async function partitionSubscriptions(
    installation: InstallationContext,
    partition: string,
    store: ObjectStore,
): Promise<readonly SubscriberEntry[]> {
    const root = installation.publisherAt(
        Egress.address(installation.id, installation.scope),
        store.origin.package.id,
    );

    return (await store.source.subscriptions(partition, { isHome: false })).map((subscription) => ({
        subscription,
        publisher: root,
    }));
}

/** Serve extensions over one store, each routing its own objects under its service. */
function compose<const Extensions extends readonly ObjectExtension[]>(
    extensions: Extensions,
    options: CompositionOptions,
): {
    readonly store: ObjectStore;
    readonly services: { readonly [Key in keyof Extensions]: ServiceImplementation };
};
/**
 * Serve extensions over one store, each routing its own objects under its service.
 *
 * @construct each extension gets its own service's implementation in order, as the result maps it.
 */
function compose(
    extensions: readonly ObjectExtension[],
    options: CompositionOptions,
): { readonly store: ObjectStore; readonly services: readonly ServiceImplementation[] } {
    // require the machine keeping any provisioned resources, and one uplink
    const { uplink, machine, policies = [], shapes = [], subscriptions, branch, ...rest } = options;
    const providers = extensions.flatMap((each) => each.providers ?? []);
    if (providers.length > 0 && machine === undefined) {
        throw new TypeError("provisioned resources need the machine keeping them");
    } else if (options.installation !== undefined && uplink !== undefined) {
        throw new TypeError("an installation's copies leave through its publisher alone");
    }

    // serve every extension's objects, refusing branches of an object none serves
    const { installation } = options;
    const objects = Object.fromEntries(extensions.flatMap((each) => Object.entries(each.objects)));
    if (
        branch !== undefined &&
        !Object.values(objects).some((object) => object.same(branch.object))
    ) {
        throw new TypeError(`no extension serves ${branch.object.name}, which branches name`);
    }
    const external = extensions.flatMap((each) =>
        each.external === undefined ? [] : [each.external],
    );
    const store: ObjectStore = new ObjectStore({
        ...rest,
        principal:
            installation === undefined
                ? options.principal
                : principal.installation.reference(installation.scope, installation.id),
        ...(installation?.partition === undefined ? {} : { partition: installation.partition }),
        objects,
        ...(branch === undefined ? {} : { branch }),
        policies: [...policies, ...extensions.flatMap((each) => each.policies ?? [])],
        shapes: [...shapes, ...extensions.flatMap((each) => each.shapes ?? [])],
        ...(external.length === 0 ? {} : { external }),
        ...(providers.length === 0 || machine === undefined
            ? {}
            : { provisioned: { providers, machine } }),
        subscriber: {
            ...(installation === undefined
                ? uplink === undefined
                    ? {}
                    : { uplink }
                : { uplink: publisherUplink(installation) }),
            watches: extensions.flatMap((each) => each.watches ?? []),
            subscriptions: async () =>
                (
                    await Promise.all([
                        installation === undefined
                            ? []
                            : installation.partition === undefined ||
                                installation.partition === installation.scope
                              ? installationSubscriptions(installation, store)
                              : partitionSubscriptions(installation, installation.partition, store),
                        subscriptions?.(store) ?? [],
                        ...extensions.map(
                            async (each) => (await each.subscriptions?.(store)) ?? [],
                        ),
                    ])
                ).flat(),
        },
    });

    // serve what each extension needs the store for, then route its objects beside its service's own procedures, the store's own controllers with the first
    const services = extensions.map((each, index) => {
        // implement the extension's service with what it serves
        const { procedures = {}, controllers = [], ...added } = each.serve?.(store) ?? {};
        const own = index === 0 ? store.controllers() : [];
        const implemented = store.implement(
            each.service,
            Object.values(each.objects),
            [...own, ...controllers],
            procedures,
        );

        return { ...implemented, ...added };
    });

    return { store, services };
}
