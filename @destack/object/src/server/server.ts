import type { AccessContext } from "@destack/access";
import type { PackageHandle } from "@destack/runtime";
import type { PackageImplementation, RequestContext } from "@destack/runtime/server";
import type { Controller } from "@destack/runtime/control";
import type { ServerContext } from "@destack/runtime/workload";
import type { ObjectType } from "../object/object.ts";
import type { Follow } from "../replica/shape.ts";
import type { ExternalStorage } from "./external.ts";
import type { ResourceControllerOptions } from "./provisioned.ts";
import type { ObjectStore } from "./store.ts";

/** Answer a request below an adapter's mount over the package's store, as the authorized caller. */
export type AdapterFetch = (
    request: Request,
    caller: AccessContext,
    context: RequestContext,
    store: ObjectStore,
) => Promise<Response>;

/** A package handle with object types, as a server implements it. */
export type ServedPackage = PackageHandle & {
    /** The object types the package declares, by name. */
    readonly objects: Readonly<Record<string, ObjectType>>;
};

/** The code behind what a package's handle declares, mirroring the handle key for key. */
export type ServerDefinition<Handle extends ServedPackage = ServedPackage> = {
    /** The handle's object types with handlers; the others serve their declared defaults. */
    readonly objects?: readonly ObjectType[];
    /** The controllers beside the objects' own, each built over the package's store. */
    readonly controllers?: readonly ((store: ObjectStore) => Controller)[];
    /** The providers reconciling the package's provisioned resources. */
    readonly providers?: ResourceControllerOptions["providers"];
    /** The shapes the package's code reads replicated, with the keys it follows them at. */
    readonly follows?: readonly Follow[];
    /** The files with the package's external objects. */
    readonly external?: ExternalStorage;
    /** The package's own access decisions beside its objects'. */
    readonly access?: PackageImplementation["access"];
    /** The headers every response of the package carries. */
    readonly responseHeaders?: PackageImplementation["responseHeaders"];
} & (string extends keyof Handle["adapters"]
    ? {
          /** The answer of each adapter the package declares, by name. */
          readonly adapters?: Readonly<Record<string, AdapterFetch>>;
      }
    : Handle["adapters"] extends Readonly<Record<string, never>>
      ? { readonly adapters?: never }
      : {
            /** The answer of each adapter the handle declares, by name. */
            readonly adapters: { readonly [Name in keyof Handle["adapters"]]: AdapterFetch };
        });

/** A package's server: the package, its definition, and the installation it serves as, if bound. */
export interface PackageServer {
    /** The package the server serves. */
    readonly package: ServedPackage;
    /** The installation the server serves as, the store's own when absent. */
    readonly installation: ServerContext | undefined;
    /** Read the package's definition for an installation, the platform's without one. */
    readonly definition: (context: ServerContext | undefined) => ServerDefinition;
    /** Serve the package as an installation inside a store of other packages. */
    readonly at: (installation: ServerContext) => PackageServer;
}

/** Define a package's server from its definition, or per installation from its context. */
export function defineServer<const Handle extends ServedPackage>(
    handle: Handle,
    definition: ServerDefinition<Handle> | ((context: ServerContext) => ServerDefinition<Handle>),
): PackageServer {
    // serve a static definition, else build the installation's from its context
    const read = (context: ServerContext | undefined): ServerDefinition => {
        if (typeof definition !== "function") {
            return definition;
        } else if (context === undefined) {
            throw new TypeError(`package ${handle.name} serves only an installation`);
        }

        return definition(context);
    };
    const serve = (installation: ServerContext | undefined): PackageServer => ({
        package: handle,
        installation,
        definition: read,
        at: (bound) => serve(bound),
    });

    return serve(undefined);
}
