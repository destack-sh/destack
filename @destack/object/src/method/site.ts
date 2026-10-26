import { AccessContext, principal } from "@destack/access";
import type { PackageId } from "@destack/package";
import type { ObjectType } from "../object/object.ts";
import { Method } from "./method.ts";

/** Where a call is made: the store, a package's own code, another installation, or a client. */
export type CallSite =
    | { readonly kind: "store" }
    | { readonly kind: "package"; readonly packageId: PackageId }
    | { readonly kind: "installation" }
    | { readonly kind: "client" };

/** The sites calls are made from, and the methods each can call. */
export const CallSite = {
    /** The store's own code, which can call every method. */
    store: { kind: "store" } as const satisfies CallSite,

    /** A client, which can call client methods. */
    client: { kind: "client" } as const satisfies CallSite,

    /** Read the site of a package's own code, in its store or its sandbox. */
    package(packageId: PackageId): CallSite {
        return { kind: "package", packageId };
    },

    /** Read the site of a caller: another installation or a client when remote, else the store. */
    of(caller: AccessContext): CallSite {
        // take a remote installation by its token, any other remote caller as a client
        if (!AccessContext.isRemote(caller)) {
            return CallSite.store;
        } else if (caller.subject !== undefined && principal.installation.is(caller.subject)) {
            return { kind: "installation" };
        }

        return CallSite.client;
    },

    /** Decide whether a site can call a method, by who the method lets call it. */
    covers(site: CallSite, object: ObjectType, method: Method): boolean {
        // let anyone call a client method
        if (Method.isRouted(method)) {
            return true;
        }
        // let any installation's code call an installation method
        else if (method.visibility === "installation") {
            return site.kind !== "client";
        }
        // let the store and the declaring package's code call a package method
        else {
            return (
                site.kind === "store" ||
                (site.kind === "package" && site.packageId === object.package.id)
            );
        }
    },
};
