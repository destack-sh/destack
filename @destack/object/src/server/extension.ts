import { principal, type SignedCaller } from "@destack/access";
import { CallError } from "@destack/runtime/error";
import type { Identifier } from "@destack/schema";
import { Egress, PACKAGE_DECLARATION, PackageDescription, PackageHandle } from "@destack/runtime";
import type { KnownDeclaration } from "../object/description.ts";
import { declareInstalled } from "../inspect/object.ts";
import type { RequestContext, PackageImplementation } from "@destack/runtime/server";
import { ObjectTypeReference, type Publisher, type Subject, type Uplink } from "@destack/sync";
import type { Follow } from "../replica/shape.ts";
import type { ServerContext } from "@destack/runtime/workload";
import { copyRequest } from "@destack/runtime/request";
import type { Controller } from "@destack/runtime/control";
import { ObjectType } from "../object/object.ts";
import type { HookSandbox } from "../hook/sandbox.ts";
import type { ExternalStorage } from "./external.ts";
import type { ProvisionedOptions } from "./provisioned.ts";
import { type CallLog, ObjectStore, type StoreOptions } from "./store.ts";

/** Answer a request below an adapter's mount over the package's store, as the admitted caller. */
export type AdapterFetch = (
    request: Request,
    caller: SignedCaller | undefined,
    context: RequestContext,
    store: ObjectStore,
) => Promise<Response>;

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
    /** The providers reconciling the extension's provisioned resources. */
    readonly providers?: ProvisionedOptions["providers"];
    /** The shapes the extension's code reads replicated, with the keys it follows them at. */
    readonly follows?: readonly Follow[];
    /** The files with the extension's external objects. */
    readonly external?: ExternalStorage;
    /** The server context serving the objects inside a space's store, absent for the platform. */
    readonly server?: ServerContext;
    /** Serve what needs the space's store: adapters, controllers, access and headers. */
    readonly serve?: (store: ObjectStore) => ObjectExtensionImplementation;
}

/** What a store of composed extensions runs with beside their objects. */
export type CompositionOptions = Omit<
    StoreOptions,
    "objects" | "provisioned" | "subscriber" | "external" | "principal" | "server"
> &
    CallLog & {
        /** The followed installation's link for replicated rows' changes, else the publisher. */
        readonly uplink?: Uplink;
        /** The machine keeping the extensions' provisioned resources, which providers require. */
        readonly machine?: Identifier<"machine">;
        /** Open the publisher toward a followed scope's home, else the store's uplink. */
        readonly publisher?: (scope: string) => Publisher;
        /** The sandboxes of the installations whose packages the store serves. */
        readonly sandboxes?: readonly HookSandbox[];
    } & (
        | {
              /** The principal the store's own work runs as. */
              readonly principal: Subject;
              /** No server context: the store runs as the given principal. */
              readonly server?: never;
          }
        | {
              /** The server context serving the objects, running the store's work and replicas. */
              readonly server: ServerContext;
              /** No principal: the store runs as the installation. */
              readonly principal?: never;
          }
    );

/** Compose stores from extensions. */
export const ObjectExtension = {
    /** Serve extensions over one store, each routing its package's objects. */
    compose,
    /** Route requests below a package's mount to its adapters, admitting their declared callers. */
    adapters: adapterFetch,
    /** Take a package as an extension with its objects' handled types, refusing missing ones. */
    of(
        served: PackageHandle & { readonly objects: Readonly<Record<string, ObjectType>> },
        handled: readonly ObjectType[],
    ): ObjectExtension {
        // refuse a handled type the package does not serve
        const own = served.objects;
        const stray = handled.find((type) => !Object.values(own).some((each) => each.same(type)));
        if (stray !== undefined) {
            throw new TypeError(`${served.name} does not serve object ${stray.name}`);
        }
        const objects = Object.values(own).map(
            (type) => handled.find((each) => each.same(type)) ?? type,
        );

        return { package: served, objects };
    },

    /** Serve a package's definition: its handled objects, adapters and controllers. */
    served<const Handle extends ServedPackage>(
        handle: Handle,
        definition: ServerDefinition<Handle>,
    ): ObjectExtension {
        // implement the handle's objects, the handled ones over their declared defaults
        const { objects = [], controllers = [], providers, follows } = definition;
        const extension = ObjectExtension.of(handle, objects);
        const adapters: Readonly<Record<string, AdapterFetch>> = definition.adapters ?? {};

        return {
            ...extension,
            ...(providers === undefined ? {} : { providers }),
            ...(follows === undefined ? {} : { follows }),
            serve: (store) => ({
                adapters,
                controllers: controllers.map((controller) => controller(store)),
            }),
        };
    },

    /** Route each extension's objects to their implementation, else to the replica kept here. */
    routes(
        extensions: readonly ObjectExtension[],
        isReplicated: (type: ObjectType) => boolean,
    ): ReadonlyMap<string, { readonly type: ObjectType; readonly extension: ObjectExtension }> {
        // implement the types of each extension's package and the types its providers provision, once
        const routes = new Map<
            string,
            { readonly type: ObjectType; readonly extension: ObjectExtension }
        >();
        for (const extension of extensions) {
            const owner = extension.package;
            const provisioned = (extension.providers ?? []).map((provider) => provider.object);
            const own = extension.objects.filter(
                (type) =>
                    type.package.id === owner.id || provisioned.some((each) => each.same(type)),
            );
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

        // route another package's type to its replica here
        for (const extension of extensions) {
            for (const type of extension.objects) {
                const key = ObjectTypeReference.key(type.typeReference);
                const isRouted = routes.has(key);

                // route a replicated type to the extension
                if (!isRouted && isReplicated(type)) {
                    routes.set(key, { type, extension });
                }
                // refuse a type nothing here implements or replicates
                else if (!isRouted) {
                    throw new TypeError(
                        `${ObjectExtension.servedBy(extension)} routes ${type.name} objects of ${type.package.id}, but nothing here implements or replicates them`,
                    );
                }
            }
        }

        return routes;
    },

    /** Describe who serves an extension's objects: its installation, else its package. */
    servedBy(extension: ObjectExtension): string {
        return extension.server === undefined
            ? `package ${extension.package.name}`
            : `installation ${extension.server.installation}`;
    },

    /** Serve an installation's package, running the code of its types in its sandbox. */
    installed(sandbox: HookSandbox, known: Iterable<KnownDeclaration>): ObjectExtension[] {
        // declare the installation's types and index them beside the known ones by reference
        const { declarations, server } = sandbox;
        const platform = [...known];
        const types = declareInstalled(declarations, platform);
        const byReference = new Map(
            [...platform, ...types]
                .filter((declared) => declared instanceof ObjectType)
                .map((type) => [ObjectTypeReference.key(type.typeReference), type]),
        );
        const owner = server.build.manifest.package;
        const own = declarations.filter((entry) => entry.packageId === owner.id);

        // route each described type, implementing the package's types in the sandbox
        const route = (routes: PackageDescription["objects"]): Record<string, ObjectType> =>
            Object.fromEntries(
                Object.entries(routes).map(([name, routed]) => {
                    // refuse a route of a type nothing here declares
                    const type = byReference.get(ObjectTypeReference.key(routed.type));
                    if (type === undefined) {
                        throw new TypeError(
                            `installation ${server.installation} routes ${routed.type.type} objects of ${routed.type.packageId}, but nothing here implements or replicates them`,
                        );
                    }

                    // implement the package's types in the sandbox and route other types as declared
                    if (routed.type.packageId === owner.id) {
                        return [name, sandbox.implement(type, routed.hooks)];
                    } else if (Object.keys(routed.hooks).length > 0) {
                        throw new TypeError(
                            `installation ${server.installation} cannot hook ${type.name} of ${type.package.id}: the type's package implements it`,
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
                        `installation ${server.installation} declares adapters ${adapters.join(", ")}, but a sandbox cannot answer them`,
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
                        server,
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
    store: ObjectStore,
): PackageImplementation["fetch"] {
    // pair each declared adapter with its answer, refusing an answer of none declared
    const adapters = Object.values(handle.adapters).map((adapter) => {
        const fetch = answers[adapter.name];
        if (fetch === undefined) {
            throw new TypeError(`package ${handle.name} does not answer adapter ${adapter.name}`);
        }

        return { name: adapter.name, path: adapter.path, callers: adapter.callers, fetch };
    });
    const stray = Object.keys(answers).find((name) => handle.adapters[name] === undefined);
    if (stray !== undefined) {
        throw new TypeError(`package ${handle.name} does not declare adapter ${stray}`);
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

        // admit the signed callers of the types the adapter declares, and anyone where it declares none
        const { callers } = adapter;
        const caller = callers === undefined ? undefined : context.requireCaller();
        if (
            callers !== undefined &&
            caller !== undefined &&
            !callers.some(
                (type) =>
                    type.packageId === caller.subject.packageId &&
                    type.type === caller.subject.type,
            )
        ) {
            throw new CallError("FORBIDDEN", {
                message: `adapter ${adapter.name} answers no ${caller.subject.type}`,
            });
        }

        // answer the request below the adapter's path
        url.pathname = url.pathname.slice(adapter.path.length) || "/";

        return adapter.fetch(copyRequest(request, {}, url.href), caller, context, store);
    };
}

/** Reach a partition at its installation's root, and other scopes as given. */
function partitionPublisher(
    server: ServerContext,
    store: () => ObjectStore,
    publisher: ((scope: string) => Publisher) | undefined,
): (scope: string) => Publisher {
    // open the root's publisher once, every link of the partition's chain going through it
    let root: Publisher | undefined;

    return (scope) => {
        // reach other scopes as given
        if (scope !== server.partition) {
            return publisher?.(scope) ?? server.publisher;
        }

        // reach the partition through its installation's root
        root ??= server.publisherAt(
            Egress.address(server.name, server.space),
            store().origin.package.id,
        );

        return root;
    };
}

/** Serve extensions over one store, each routing its package's objects. */
function compose(
    extensions: readonly ObjectExtension[],
    options: CompositionOptions,
): { readonly store: ObjectStore; readonly packages: readonly PackageImplementation[] } {
    // require the machine keeping any provisioned resources, and one uplink
    const { uplink, machine, policies = [], branch, publisher, ...rest } = options;
    const { sandboxes = [], server, ...stored } = rest;
    const providers = extensions.flatMap((each) => each.providers ?? []);
    if (providers.length > 0 && machine === undefined) {
        throw new TypeError("provisioned resources need their machine");
    } else if (server !== undefined && uplink !== undefined) {
        throw new TypeError("an installation's replicas must leave through its publisher");
    }

    // serve the sandboxed installations' types, declared against what the other extensions make known
    const served = extensions.flatMap((each) => each.objects);
    const followed = extensions
        .flatMap((each) => each.follows ?? [])
        .flatMap((each) => each.shape.objects);
    const known = ObjectStore.known(served, [
        ...policies,
        ...ObjectType.referenced(served),
        ...followed,
    ]);
    const composed = [
        ...extensions,
        ...sandboxes.flatMap((sandbox) => ObjectExtension.installed(sandbox, known)),
    ];

    // serve every extension's objects, refusing branches of an object none serves
    const routes = ObjectExtension.routes(composed, (type) =>
        options.database.replicates(type.table),
    );

    // answer queries of replicated types nothing here serves over their replicated rows
    const replicas = known
        .filter((type) => type instanceof ObjectType)
        .filter(
            (type) =>
                !routes.has(ObjectTypeReference.key(type.typeReference)) &&
                options.database.replicates(type.table),
        );
    const objects = [...routes.values()].map((route) => route.type);
    if (branch !== undefined && !objects.some((object) => object.same(branch.object))) {
        throw new TypeError(
            `no extension serves ${branch.object.name}, the object of the branches`,
        );
    }
    const follows = [...(options.follows ?? []), ...composed.flatMap((each) => each.follows ?? [])];
    const external = composed.flatMap((each) =>
        each.external === undefined ? [] : [each.external],
    );

    // serve each type at its package's installation, else at the store's installation
    const installed = new Map(
        [...routes].flatMap(([key, { type, extension }]) => {
            const serving = extension.server ?? server;
            const isOwned = type.package.id === extension.package.id;

            return serving === undefined || !isOwned ? [] : [[key, serving] as const];
        }),
    );

    // reach a partition's chain at its installation's root
    const isPartition = server?.partition !== undefined && server.partition !== server.space;
    const reach =
        server !== undefined && isPartition
            ? partitionPublisher(server, () => store, publisher)
            : publisher;
    const store: ObjectStore = new ObjectStore({
        ...stored,
        installed,
        ...(replicas.length === 0 ? {} : { replicas }),
        principal:
            options.server === undefined
                ? options.principal
                : principal.installation.reference(
                      options.server.space,
                      options.server.installation,
                  ),
        ...(server?.partition === undefined ? {} : { partition: server.partition }),
        objects,
        ...(branch === undefined ? {} : { branch }),
        policies,
        ...(external.length === 0 ? {} : { external }),
        ...(follows.length === 0 ? {} : { follows }),
        ...(reach === undefined ? {} : { publisher: reach }),
        ...(providers.length === 0 || machine === undefined
            ? {}
            : { provisioned: { providers, machine } }),
        subscriber:
            server === undefined
                ? uplink === undefined
                    ? {}
                    : { uplink }
                : { uplink: server.publisher },
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
        const fetch = adapterFetch(each.package, adapters, store);

        return {
            ...implemented,
            ...(access === undefined ? {} : { access }),
            ...(responseHeaders === undefined ? {} : { responseHeaders }),
            ...(fetch === undefined ? {} : { fetch }),
        };
    });

    return { store, packages };
}

/** The code behind what a package's handle declares, mirroring the handle key for key. */
export type ServerDefinition<Handle extends ServedPackage> = {
    /** The handle's object types with handlers; the others serve their declared defaults. */
    readonly objects?: readonly ObjectType[];
    /** The controllers beside the objects' own, each built over the package's store. */
    readonly controllers?: readonly ((store: ObjectStore) => Controller)[];
    /** The providers reconciling the package's provisioned resources. */
    readonly providers?: ProvisionedOptions["providers"];
    /** The shapes the package's code reads replicated, with the keys it follows them at. */
    readonly follows?: readonly Follow[];
} & (Handle["adapters"] extends Readonly<Record<string, never>>
    ? { readonly adapters?: never }
    : {
          /** The answer of each adapter the handle declares, by name. */
          readonly adapters: { readonly [Name in keyof Handle["adapters"]]: AdapterFetch };
      });

/** A package handle with object types, as a server implements it. */
export type ServedPackage = PackageHandle & {
    /** The object types the package declares, by name. */
    readonly objects: Readonly<Record<string, ObjectType>>;
};

/** A package's server: the package, and its extension of a store for an installation. */
export interface PackageServer {
    /** The package the server serves. */
    readonly package: ServedPackage;
    /** Extend a store with the package for an installation, the platform's without one. */
    readonly extension: (context: ServerContext | undefined) => ObjectExtension;
}

/** Define a package's server from its definition, or per installation from its context. */
export function defineServer<const Handle extends ServedPackage>(
    handle: Handle,
    definition: ServerDefinition<Handle> | ((context: ServerContext) => ServerDefinition<Handle>),
): PackageServer {
    return {
        package: handle,
        extension: (context) => {
            // serve a static definition, else build the installation's from its context
            if (typeof definition !== "function") {
                const served = ObjectExtension.served(handle, definition);

                return context === undefined ? served : { ...served, server: context };
            } else if (context === undefined) {
                throw new TypeError(`package ${handle.name} serves only an installation`);
            }

            return { ...ObjectExtension.served(handle, definition(context)), server: context };
        },
    };
}
