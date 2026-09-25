import type { Package } from "@destack/package";
import type { ResourceContext } from "../context/index.ts";
import type { ResourceDescription } from "./resource.ts";

/** A named declaration whose client is selected by the invocation's host. */
export class ResourceHandle<Client> {
    /** The package declaring the handle, supplied by the module transform. */
    readonly package: Package;
    /** The package-local resource name. */
    readonly name: ResourceDescription["name"];

    /** Retain the declaring package and its package-local binding name. */
    constructor(owner: Package, name: ResourceDescription["name"]) {
        this.package = owner;
        this.name = name;
    }

    /** Get the client bound to the current operation. */
    get(context: ResourceContext): Client {
        return context.get(this);
    }
}
