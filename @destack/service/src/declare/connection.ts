import { ResourceHandle, ResourceName } from "@destack/resource";
import { DeclarationReference } from "@destack/package/workload";
import { PackageId } from "@destack/package";
import { defineSchema, schema } from "@destack/schema";
import type { Client, Service } from "../service/service.ts";

/** A named dependency on a provided service. */
export const ServiceConnectionDeclaration = defineSchema(
    schema.object({
        /** The immutable identity of the declaring package. */
        packageId: PackageId,
        /** The package-local connection name. */
        name: ResourceName,
        /** The required service declaration. */
        service: DeclarationReference,
    }),
);
/** A named dependency on a provided service. */
export type ServiceConnectionDeclaration = schema.Infer<typeof ServiceConnectionDeclaration>;

/** An inert service dependency with a host-bound typed client. */
export class ServiceConnection<Router extends Service = Service> extends ResourceHandle<
    Client<Router>
> {
    /** The immutable identity of the declaring package. */
    readonly packageId: PackageId;
    /** The required service declaration. */
    readonly service: ServiceConnectionDeclaration["service"];
    /** The procedure definitions used to construct the client. */
    readonly router: Router;

    /** Retain the dependency description and its typed API without opening a connection. */
    constructor(declaration: ServiceConnectionDeclaration, router: Router) {
        super(declaration.name);
        this.packageId = declaration.packageId;
        this.service = declaration.service;
        this.router = router;
    }
}

/** Define a service dependency collected by package inspection. */
export function defineServiceConnection<Router extends Service>(
    declaration: ServiceConnectionDeclaration,
    router: Router,
): ServiceConnection<Router> {
    return new ServiceConnection(ServiceConnectionDeclaration.parse(declaration), router);
}
