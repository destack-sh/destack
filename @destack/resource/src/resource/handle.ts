import type { ResourceContext } from "../context/index.ts";
import type { ResourceDeclaration } from "./resource.ts";

/** A named declaration whose client is selected by the invocation's host. */
export class ResourceHandle<Client> {
    /** Retain the package-local binding name. */
    constructor(readonly name: ResourceDeclaration["name"]) {}

    /** Get the client bound to the current operation. */
    get(context: ResourceContext): Client {
        return context.get(this);
    }
}
