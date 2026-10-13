import { type Policy, principal, type TableMapping } from "@destack/access";
import type { Table } from "@destack/db";
import type { Identifier } from "@destack/schema";
import { Egress, PACKAGE_DECLARATION, PackageDescription, PackageHandle } from "@destack/runtime";
import type { KnownDeclaration } from "../object/description.ts";
import { declareInstalled } from "../inspect/object.ts";
import type { RequestContext, PackageImplementation } from "@destack/runtime/server";
import { ObjectTypeReference, type Shape, type Subject, type Uplink } from "@destack/sync";
import type { InstallationContext } from "@destack/runtime/workload";
import { copyRequest } from "@destack/runtime/request";
import { ProjectionParameters } from "../replica/replica.ts";
import { ObjectType } from "../object/object.ts";
import type { SandboxHooks } from "../hook/sandbox.ts";
import type { ExternalStorage } from "./external.ts";
import type { ProvisionedOptions } from "./provisioned.ts";
import { ObjectStore, type ObjectStoreOptions, type SubscriberEntry } from "./store.ts";

/** Answer a request to an adapter, its path relative to the adapter's mount. */
export type AdapterFetch = (request: Request, context: RequestContext) => Promise<Response>;

/** What an extension adds once the space's store exists. */
export interface ObjectExtensionImplementation extends Pick<
    PackageImplementation,
    "access" | "responseHeaders" | "controllers"
> {
    /** Answer each adapter the package declares, by adapter name. */
    readonly adapters?: Readonly<Record<string, AdapterFetch>>;
}

/** One package's contribution to a space's store. */
export interface ObjectExtension {
    /** The package whose objects and adapters the extension serves. */
    readonly package: PackageHandle;
    /** The handled types the extension serves. */
    readonly objects: readonly ObjectType[];
    /** The policies of further types the objects authorize through, such as copied scopes. */
    readonly policies?: readonly (Policy | ObjectType | TableMapping)[];
    /** The providers reconciling the extension's provisioned resources. */
    readonly providers?: ProvisionedOptions["providers"];
    /** The shapes the extension serves and follows beside its objects' shapes. */
    readonly shapes?: readonly Shape[];
    /** The files with the extension's external objects. */
    readonly external?: ExternalStorage;
    /** The tables whose changes list the space's copies again. */
    readonly watches?: readonly Table[];
    /** The installation serving the objects inside a space's store, absent for the platform. */
    readonly installation?: InstallationContext;
    /** List the copies the extension keeps in the space's database, each with its publisher. */
    subscriptions?(store: ObjectStore): Promise<readonly SubscriberEntry[]>;
    /** Serve what needs the space's store: adapters, controllers, access and headers. */
    serve?(store: ObjectStore): ObjectExtensionImplementation;
}

/** What a store of composed extensions runs with beside their objects. */
export type CompositionOptions = Omit<
    ObjectStoreOptions,
    "objects" | "provisioned" | "subscriber" | "external" | "principal" | "installation"
> & {
    /** The followed installation's link for copied rows' changes, else the publisher. */
    readonly uplink?: Uplink;
    /** The machine keeping the extensions' provisioned resources, required by any provider. */
    readonly machine?: Identifier<"machine">;
    /** List the store's extra copies, such as its scope's chain, each with its publisher. */
    readonly subscriptions?: (store: ObjectStore) => Promise<readonly SubscriberEntry[]>;
    /** The sandboxes of the installations whose packages the store serves. */
    readonly sandboxes?: readonly SandboxHooks[];
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
    /** Serve extensions over one store, each routing its package's objects. */
    compose,
    /** Take a package as an extension with its objects' handled types, refusing missing ones. */
    of(
        served: PackageHandle & { readonly objects: Readonly<Record<string, ObjectType>> },
        handled: readonly ObjectType[],
    ): ObjectExtension {
        // refuse a handled type the package does not serve
        const own = served.objects;
        const stray = handled.find((type) => !Object.values(own).some((each) => each.same(type)));
        if (stray !== undefined) {
            throw new TypeError(`${served.name} serves no object ${stray.name}`);
        }
        const objects = Object.values(own).map(
            (type) => handled.find((each) => each.same(type)) ?? type,
        );

        return { package: served, objects };
    },

    /** Route each extension's objects to their one implementation, else to the copy kept here. */
    routes(
        extensions: readonly ObjectExtension[],
        isCopied: (type: ObjectType) => boolean,
    ): ReadonlyMap<string, { readonly type: ObjectType; readonly extension: ObjectExtension }> {
        // implement the types of each extension's package once
        const routes = new Map<
            string,
            { readonly type: ObjectType; readonly extension: ObjectExtension }
        >();
        for (const extension of extensions) {
            const owner = extension.package;
            const own = extension.objects.filter((type) => type.package.id === owner.id);
            for (const type of own) {
                const key = ObjectTypeReference.key(type.typeReference);
                const earlier = routes.get(key);
                if (earlier !== undefined) {
                    throw new TypeError(
                        `${ObjectExtension.servedBy(earlier.extension)} and ${ObjectExtension.servedBy(extension)} both implement ${type.name} objects of ${type.package.id}`,
                    );
                }
                routes.set(key, { type, extension });
            }
        }

        // route another package's type to its copy here, refusing one nothing here implements or copies
        for (const extension of extensions) {
            for (const type of extension.objects) {
                const key = ObjectTypeReference.key(type.typeReference);
                const isRouted = routes.has(key);

                // route a copied type to the extension
                if (!isRouted && isCopied(type)) {
                    routes.set(key, { type, extension });
                }
                // refuse a type nothing here implements or copies
                else if (!isRouted) {
                    throw new TypeError(
                        `${ObjectExtension.servedBy(extension)} routes ${type.name} objects of ${type.package.id}, which nothing here implements or copies`,
                    );
                }
            }
        }

        return routes;
    },

    /** Describe who serves an extension's objects: its installation, else its package. */
    servedBy(extension: ObjectExtension): string {
        return extension.installation === undefined
            ? `package ${extension.package.name}`
            : `installation ${extension.installation.id}`;
    },

    /** Serve an installation's package, running the code of its types in its sandbox. */
    installed(sandbox: SandboxHooks, known: Iterable<KnownDeclaration>): ObjectExtension[] {
        // declare the installation's types and index them beside the known ones by reference
        const { declarations, installation } = sandbox;
        const platform = [...known];
        const types = declareInstalled(declarations, platform);
        const byReference = new Map(
            [...platform, ...types]
                .filter((declared) => declared instanceof ObjectType)
                .map((type) => [ObjectTypeReference.key(type.typeReference), type]),
        );
        const owner = installation.build.manifest.package;
        const own = declarations.filter((entry) => entry.packageId === owner.id);

        // route each described type, implementing the package's types in the sandbox
        const route = (routes: PackageDescription["objects"]): Record<string, ObjectType> =>
            Object.fromEntries(
                Object.entries(routes).map(([name, routed]) => {
                    // refuse a route of a type nothing here declares
                    const type = byReference.get(ObjectTypeReference.key(routed.type));
                    if (type === undefined) {
                        throw new TypeError(
                            `installation ${installation.id} routes ${routed.type.type} objects of ${routed.type.packageId}, which nothing here implements or copies`,
                        );
                    }

                    // implement the package's types in the sandbox and route other types as declared
                    if (routed.type.packageId === owner.id) {
                        return [name, sandbox.implement(type, routed.hooks)];
                    } else if (Object.keys(routed.hooks).length > 0) {
                        throw new TypeError(
                            `installation ${installation.id} hooks ${type.name} of ${type.package.id}, which the type's package implements`,
                        );
                    }

                    return [name, type];
                }),
            );

        // serve the package's objects, refusing adapters a sandbox has no code to answer
        const packages = own
            .filter((entry) => entry.kind === PACKAGE_DECLARATION)
            .map((entry) => PackageDescription.parse(entry.description))
            .flatMap((described): ObjectExtension[] => {
                // refuse adapters, and leave a package of no objects unserved
                const adapters = Object.keys(described.adapters);
                if (adapters.length > 0) {
                    throw new TypeError(
                        `installation ${installation.id} declares adapters ${adapters.join(", ")}, which a sandbox does not answer`,
                    );
                } else if (Object.keys(described.objects).length === 0) {
                    return [];
                }

                // implement the package's objects as the installation
                const objects = route(described.objects);
                const since = described.since === undefined ? {} : { since: described.since };

                return [
                    {
                        package: PackageHandle.of(owner, { objects, ...since }),
                        objects: Object.values(objects),
                        installation,
                    },
                ];
            });

        return packages;
    },
};

/** Route requests below a package's mount to the adapters answering their paths. */
function adapterFetch(
    handle: PackageHandle,
    answers: Readonly<Record<string, AdapterFetch>>,
): PackageImplementation["fetch"] {
    // pair each declared adapter with its answer, refusing an answer of none declared
    const adapters = Object.values(handle.adapters).map((adapter) => {
        const fetch = answers[adapter.name];
        if (fetch === undefined) {
            throw new TypeError(`package ${handle.name} answers no adapter ${adapter.name}`);
        }

        return { path: adapter.path, fetch };
    });
    const stray = Object.keys(answers).find((name) => handle.adapters[name] === undefined);
    if (stray !== undefined) {
        throw new TypeError(`package ${handle.name} declares no adapter ${stray}`);
    } else if (adapters.length === 0) {
        return undefined;
    }

    return async (request, context) => {
        // find the adapter whose path the request is at or below
        const url = new URL(request.url);
        const adapter = adapters.find(
            ({ path }) => url.pathname === path || url.pathname.startsWith(`${path}/`),
        );
        if (adapter === undefined) {
            return undefined;
        }

        // answer the request below the adapter's path
        url.pathname = url.pathname.slice(adapter.path.length) || "/";

        return adapter.fetch(copyRequest(request, {}, url.href), context);
    };
}

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
        ...(await store.source.subscriptions(installation.scope, { kind: "follower" })).map(
            (subscription) => ({ subscription, publisher: installation.publisher }),
        ),
        ...(await projectionSubscriptions(installation, store.objects, store)),
    ];
}

/** List the projections an installation follows into its space for the types it serves. */
async function projectionSubscriptions(
    installation: InstallationContext,
    objects: readonly ObjectType[],
    store: ObjectStore,
): Promise<readonly SubscriberEntry[]> {
    // follow each source installation's projected rows through the installation's egress
    const subscriptions = await store.source.projectionSubscriptions(installation.scope, objects);

    return subscriptions.map((subscription) => {
        const { installation: keeping, packageId } = ProjectionParameters.parse(
            subscription.parameters,
        );

        return {
            subscription,
            publisher: installation.publisherAt(
                Egress.address(keeping, subscription.scope),
                packageId,
            ),
        };
    });
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

    return (await store.source.subscriptions(partition, { kind: "follower" })).map(
        (subscription) => ({
            subscription,
            publisher: root,
        }),
    );
}

/** Serve extensions over one store, each routing its package's objects. */
function compose<const Extensions extends readonly ObjectExtension[]>(
    extensions: Extensions,
    options: CompositionOptions,
): {
    readonly store: ObjectStore;
    readonly packages: { readonly [Key in keyof Extensions]: PackageImplementation };
};
/**
 * Serve extensions over one store, each routing its package's objects.
 *
 * @construct each extension gets its package's implementation, in order.
 */
function compose(
    extensions: readonly ObjectExtension[],
    options: CompositionOptions,
): { readonly store: ObjectStore; readonly packages: readonly PackageImplementation[] } {
    // require the machine keeping any provisioned resources, and one uplink
    const { uplink, machine, policies = [], shapes = [], subscriptions, branch, ...rest } = options;
    const { sandboxes = [], installation, ...stored } = rest;
    const providers = extensions.flatMap((each) => each.providers ?? []);
    if (providers.length > 0 && machine === undefined) {
        throw new TypeError("provisioned resources need the machine keeping them");
    } else if (installation !== undefined && uplink !== undefined) {
        throw new TypeError("an installation's copies leave through its publisher alone");
    }

    // serve the sandboxed installations' types, declared against what the other extensions make known
    const known = ObjectStore.known(
        extensions.flatMap((each) => each.objects),
        [...policies, ...extensions.flatMap((each) => each.policies ?? [])],
    );
    const composed = [
        ...extensions,
        ...sandboxes.flatMap((sandbox) => ObjectExtension.installed(sandbox, known)),
    ];

    // serve every extension's objects, refusing branches of an object none serves
    const routes = ObjectExtension.routes(composed, (type) => options.database.copies(type.table));
    const objects = [...routes.values()].map((route) => route.type);
    if (branch !== undefined && !objects.some((object) => object.same(branch.object))) {
        throw new TypeError(`no extension serves ${branch.object.name}, which branches name`);
    }
    const external = composed.flatMap((each) =>
        each.external === undefined ? [] : [each.external],
    );

    // serve each type at its package's installation, else at the store's installation
    const installed = new Map(
        [...routes].flatMap(([key, { type, extension }]) => {
            const serving = extension.installation ?? installation;
            const isOwned = type.package.id === extension.package.id;

            return serving === undefined || !isOwned ? [] : [[key, serving] as const];
        }),
    );
    const store: ObjectStore = new ObjectStore({
        ...stored,
        installed,
        principal:
            options.installation === undefined
                ? options.principal
                : principal.installation.reference(
                      options.installation.scope,
                      options.installation.id,
                  ),
        ...(installation?.partition === undefined ? {} : { partition: installation.partition }),
        objects,
        ...(branch === undefined ? {} : { branch }),
        policies: [...policies, ...composed.flatMap((each) => each.policies ?? [])],
        shapes: [...shapes, ...composed.flatMap((each) => each.shapes ?? [])],
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
            watches: composed.flatMap((each) => each.watches ?? []),
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
                        ...composed.map(async (each) => (await each.subscriptions?.(store)) ?? []),
                        ...composed.map(async ({ installation: serving, objects: served }) =>
                            serving === undefined
                                ? []
                                : projectionSubscriptions(serving, served, store),
                        ),
                    ])
                ).flat(),
        },
    });

    // implement each extension's package, giving the first the store's controllers
    const packages = composed.map((each, index) => {
        // implement the extension's package with what it serves
        const {
            controllers = [],
            adapters = {},
            access,
            responseHeaders,
        } = each.serve?.(store) ?? {};
        const own = index === 0 ? store.controllers() : [];
        const implemented = store.implement(each.package, each.objects, [...own, ...controllers]);

        // answer the package's adapters below its mount
        const fetch = adapterFetch(each.package, adapters);

        return {
            ...implemented,
            ...(access === undefined ? {} : { access }),
            ...(responseHeaders === undefined ? {} : { responseHeaders }),
            ...(fetch === undefined ? {} : { fetch }),
        };
    });

    return { store, packages };
}
