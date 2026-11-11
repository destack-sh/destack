import type { PackageVersion } from "@destack/package";
import type { ResourceContext } from "../context/index.ts";
import type { ResourceDescription } from "./declaration.ts";

/** A named declaration whose client is selected by the invocation's host. */
export class ResourceHandle<Client> {
    /** The package declaring the handle, supplied by the module transform. */
    readonly package: PackageVersion;
    /** The package-local resource name. */
    readonly name: ResourceDescription["name"];

    /** Retain the declaring package and its package-local binding name. */
    constructor(owner: PackageVersion, name: ResourceDescription["name"]) {
        this.package = owner;
        this.name = name;
    }

    /** Get the client bound to the current operation. */
    get(context: ResourceContext): Client {
        return context.get(this);
    }
}
