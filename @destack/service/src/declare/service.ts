import { DeclarationName, declaringModule, Package, type ModuleMetadata } from "@destack/package";
import type { Declaration } from "@destack/package/declare";
import type { ServiceRouter } from "../service/service.ts";

/** A declared HTTP service and the procedures its workload implements. */
export interface Service<Router extends ServiceRouter = ServiceRouter> extends Declaration {
    /** The declaration format version. */
    readonly version: 1;
    /** The service transport. */
    readonly protocol: "http";
    /** Procedures implemented by the named service. */
    readonly router: Router;
}

/** Declare an HTTP service and its procedures for workload routing. */
export function defineService<Router extends ServiceRouter>(
    name: string,
    router: Router,
    module?: ModuleMetadata,
): Service<Router> {
    // stamp the declaring package supplied by the module transform
    const owner = Package.parse(declaringModule(module, "defineService").package);

    return Object.freeze({
        package: owner,
        name: DeclarationName.parse(name),
        version: 1,
        protocol: "http",
        router,
    });
}
