import type { ResourceContext } from "../context/index.ts";
import type { ResourceDeclaration } from "./resource.ts";

/** A named declaration whose client is selected by the invocation's host. */
export class ResourceHandle<Client> {
    /** Package-local resource name. */
    readonly name: ResourceDeclaration["name"];

    /** Retain the package-local binding name. */
    constructor(name: ResourceDeclaration["name"]) {
        this.name = name;
    }

    /** Get the client bound to the current operation. */
    get(context: ResourceContext): Client {
        return context.get(this);
    }
}
