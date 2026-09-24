import { ResourceHandle } from "@destack/resource";
import {
    DeclarationName,
    declaringModule,
    PackageId,
    type ModuleMetadata,
    type Package,
} from "@destack/package";
import { DeclarationReference, reference } from "@destack/package/declare";
import { defineSchema, schema } from "@destack/schema";
import type { Client, ServiceRouter } from "../service/service.ts";
import type { Service } from "./service.ts";

/** A named dependency on a provided service. */
export const ServiceConnectionDescription = defineSchema(
    schema.object({
        /** The immutable identity of the declaring package. */
        packageId: PackageId,
        /** The package-local connection name. */
        name: DeclarationName,
        /** The required service declaration. */
        service: DeclarationReference,
    }),
);
/** A named dependency on a provided service. */
export type ServiceConnectionDescription = schema.Infer<typeof ServiceConnectionDescription>;

/** An inert service dependency with a host-bound typed client. */
export class ServiceConnection<Router extends ServiceRouter = ServiceRouter> extends ResourceHandle<
    Client<Router>
> {
    /** The required service declaration. */
    readonly service: ServiceConnectionDescription["service"];
    /** The procedure definitions used to construct the client. */
    readonly router: Router;

    /** Retain the dependency description and its typed API without opening a connection. */
    constructor(owner: Package, declaration: ServiceConnectionDescription, router: Router) {
        // retain the provider package, service and router
        super(owner, declaration.name);
        this.service = declaration.service;
        this.router = router;
    }
}

/** Define a named dependency on a declared service, collected by package inspection. */
export function defineServiceConnection<Router extends ServiceRouter>(
    name: string,
    service: Service<Router>,
    module?: ModuleMetadata,
): ServiceConnection<Router> {
    // reference the service by its declaring package and name
    const owner = declaringModule(module, "defineServiceConnection").package;
    const declaration = ServiceConnectionDescription.parse({
        packageId: owner.id,
        name,
        service: reference(service),
    });

    return new ServiceConnection(owner, declaration, service.router);
}
