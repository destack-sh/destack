import type { AccessContext } from "@destack/access";
import type { PackageHandle } from "../declare/package.ts";
import type { PackageHandler } from "./http.ts";
import type { RequestContext } from "./context.ts";
import type { Controller } from "@destack/db";
import type { ServerContext } from "../workload/workload.ts";
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

/** A package handle whose objects are object types, as a store serves them. */
export type ObjectPackage = PackageHandle & {
    /** The object types the package declares, by name. */
    readonly objects: Readonly<Record<string, ObjectType>>;
};

/** The code behind what a package's handle declares, mirroring the handle key for key. */
export type ImplementationInput<Handle extends ObjectPackage = ObjectPackage> = {
    /** The handle's object types with handlers; the others serve their declared defaults. */
    readonly objects?: readonly ObjectType[];
    /** The controllers beside the objects' controllers, each built over the package's store. */
    readonly controllers?: readonly ((store: ObjectStore) => Controller)[];
    /** The providers reconciling the package's provisioned resources. */
    readonly providers?: ResourceControllerOptions["providers"];
    /** The shapes the package's code reads replicated, with the keys it follows them at. */
    readonly follows?: readonly Follow[];
    /** The files with the package's external objects. */
    readonly external?: ExternalStorage;
    /** The package's access decisions beside its objects'. */
    readonly access?: PackageHandler["access"];
    /** The headers every response of the package includes. */
    readonly responseHeaders?: PackageHandler["responseHeaders"];
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

/** A package's implementation: the package, its code, and the installation it serves as. */
export interface PackageImplementation {
    /** The implemented package. */
    readonly package: ObjectPackage;
    /** The installation it serves as, the store's when absent. */
    readonly installation: ServerContext | undefined;
    /** Read the package's code for an installation, the platform's without one. */
    readonly definition: (context: ServerContext | undefined) => ImplementationInput;
    /** Serve the package as an installation inside a store of other packages. */
    readonly at: (installation: ServerContext) => PackageImplementation;
}

/** Implement a package from its code, or per installation from its context. */
export function implementPackage<const Handle extends ObjectPackage>(
    handle: Handle,
    definition:
        | ImplementationInput<Handle>
        | ((context: ServerContext) => ImplementationInput<Handle>),
): PackageImplementation {
    // serve a static definition, else build the installation's from its context
    const read = (context: ServerContext | undefined): ImplementationInput => {
        if (typeof definition !== "function") {
            return definition;
        } else if (context === undefined) {
            throw new TypeError(`package ${handle.name} serves only an installation`);
        }

        return definition(context);
    };
    const serve = (installation: ServerContext | undefined): PackageImplementation => ({
        package: handle,
        installation,
        definition: read,
        at: (bound) => serve(bound),
    });

    return serve(undefined);
}
