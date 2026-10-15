import { AccessContext } from "@destack/access";
import type { PackageId } from "@destack/package";
import type { ObjectType } from "../object/object.ts";
import type { Method } from "./method.ts";

/** Where a call is made: the store's own code, a client, or a package's sandboxed hook. */
export type CallSite =
    | { readonly kind: "store" }
    | { readonly kind: "client" }
    | { readonly kind: "sandbox"; readonly packageId: PackageId };

/** The sites calls are made from, and the methods each can call. */
export const CallSite = {
    /** The store's own code, which can call every method. */
    store: { kind: "store" } as const satisfies CallSite,

    /** A client, which can call public methods. */
    client: { kind: "client" } as const satisfies CallSite,

    /** Read the site of a caller: a client for a remote caller, else the store. */
    of(caller: AccessContext): CallSite {
        return AccessContext.isRemote(caller) ? CallSite.client : CallSite.store;
    },

    /** Decide whether a site can call a method: an internal one only from its store or package. */
    reaches(site: CallSite, object: ObjectType, method: Method): boolean {
        return (
            method.isInternal !== true ||
            site.kind === "store" ||
            (site.kind === "sandbox" && site.packageId === object.package.id)
        );
    },
};
