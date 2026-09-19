import type { ResourceContext } from "../context/index.ts";
import type { ResourceDeclaration } from "./resource.ts";

/** An inert declaration with access to a host-bound client. */
export class Resource<Value, Declaration extends ResourceDeclaration = ResourceDeclaration> {
    /** The declaration name. */
    readonly name: Declaration["name"];
    /** The resource kind. */
    readonly kind: Declaration["kind"];
    /** The declaration format version. */
    readonly version: Declaration["version"];
    /** The domain specification. */
    readonly spec: Declaration["spec"];

    /** Retain validated metadata without opening a resource. */
    constructor(declaration: Declaration) {
        this.name = declaration.name;
        this.kind = declaration.kind;
        this.version = declaration.version;
        this.spec = declaration.spec;
    }

    /** Get the resource client bound to the current operation. */
    get(context: ResourceContext): Value {
        return context.get(this);
    }
}
