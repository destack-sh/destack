import { defineSchema, schema } from "@destack/schema";
import { ResourceName } from "@destack/resource";
import type { Service } from "../service/service.ts";

/** An HTTP service handled by a workload's exported function. */
export const ServiceDeclaration = defineSchema(
    schema.object({
        /** The package-local service name. */
        name: ResourceName,
        /** The declaration format version. */
        version: schema.literal(1),
        /** The exported function accepting a Request and returning a Response. */
        handler: schema.string().min(1),
        /** The service transport. */
        protocol: schema.literal("http"),
    }),
);
/** An HTTP service handled by a workload's exported function. */
export type ServiceDeclaration = schema.Infer<typeof ServiceDeclaration>;

/** An HTTP declaration and its optional typed router. */
export interface ServiceDefinition<Router extends Service = Service> extends ServiceDeclaration {
    /** Procedures used by the declared handler. */
    router?: Router;
}

/** Declare an HTTP service for workload routing. */
export function defineService<Router extends Service = Service>(
    declaration: ServiceDeclaration,
    router?: Router,
): ServiceDefinition<Router> {
    return { ...ServiceDeclaration.parse(declaration), ...(router ? { router } : {}) };
}
