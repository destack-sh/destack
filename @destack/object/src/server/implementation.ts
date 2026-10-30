import type { AccessContext } from "@destack/access";
import type { PackageHandle } from "../declare/package.ts";
import type { PackageHandler } from "./http.ts";
import type { RequestContext } from "./context.ts";
import type { Controller } from "@destack/db";
import {
    type BaseServerContext,
    type PackageContext,
    ServerContext,
} from "../workload/workload.ts";
import type { ObjectType } from "../object/object.ts";
import type { Follow } from "../replica/shape.ts";
import type { ExternalStorage } from "./external.ts";
import type { ResourceControllerOptions } from "./provisioned.ts";
import type { ObjectStore } from "./store.ts";
import type { Bindable } from "../installation/bindable.ts";
import { Installation } from "../installation/installation.ts";
import type { ObjectReference } from "@destack/sync";
import type { DigestCrypto } from "@destack/schema";

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
    /** The providers of the resource kinds the package provides to every space serving it. */
    readonly providers?: ResourceControllerOptions["providers"];
    /** The kinds of objects the package lets installations bind, such as secrets. */
    readonly bindables?: readonly Bindable[];
    /** The shapes the package's code reads replicated, with the keys it follows them at. */
    readonly follows?: readonly Follow[];
    /** The on-demand follows of the rows standing for callers, acquired where they are missing. */
    readonly standing?: readonly Extract<Follow, { readonly onDemand: unknown }>[];
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
    readonly definition: (
        context: PackageContext | BaseServerContext | undefined,
    ) => ImplementationInput;
    /** Serve the package as an installation inside a store of other packages. */
    readonly at: (installation: ServerContext) => PackageImplementation;
    /** Serve the package as its base installation in a scope. */
    readonly base: (scope: ObjectReference, crypto: DigestCrypto) => Promise<PackageImplementation>;
    /** The providers of the resource kinds the package provides to every installation. */
    readonly providers: NonNullable<ImplementationInput["providers"]>;
    /** The kinds of objects the package lets every installation bind. */
    readonly bindables: readonly Bindable[];
}

/** Implement a package from its code, or per installation from its context. */
export function implementPackage<const Handle extends ObjectPackage>(
    handle: Handle,
    definition:
        | ImplementationInput<Handle>
        | ((context: PackageContext) => ImplementationInput<Handle>),
): PackageImplementation {
    // serve a static definition, else build an installed installation's from its context
    const read = (context: PackageContext | BaseServerContext | undefined): ImplementationInput => {
        // serve a static definition as it is
        if (typeof definition !== "function") {
            return definition;
        }
        // refuse serving a per-installation definition without an installation
        else if (context === undefined) {
            throw new TypeError(`package ${handle.name} can only serve an installation`);
        }
        // refuse one not installed yet
        else if (!ServerContext.isInstalled(context)) {
            throw new TypeError(`package ${handle.name} can only serve an installed installation`);
        }

        // refuse providers or bindables that differ by installation
        const installed = definition(context);
        if (installed.providers !== undefined || installed.bindables !== undefined) {
            throw new TypeError(
                `package ${handle.name} must declare its providers and bindables for every installation`,
            );
        }

        return installed;
    };
    const serve = (installation: ServerContext | undefined): PackageImplementation => ({
        package: handle,
        installation,
        definition: read,
        at: (bound) => serve(bound),
        base: async (scope, crypto) => serve(await Installation.base(scope, handle, crypto)),
        providers: typeof definition === "function" ? [] : (definition.providers ?? []),
        bindables: typeof definition === "function" ? [] : (definition.bindables ?? []),
    });

    return serve(undefined);
}
