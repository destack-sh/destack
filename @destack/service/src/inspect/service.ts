import { defineSchema, schema, Version } from "@destack/schema";
import type { ServiceRouter } from "../service/index.ts";
import { describeProcedures, ProcedureDescription } from "./procedure.ts";
import type { Service } from "../declare/service.ts";
import { DeclarationName } from "@destack/package";

/** The procedures of a service. */
export const RouterDescription = defineSchema(
    schema.object({
        /** The package-local service name. */
        name: schema.string().min(1),
        /** The procedures. */
        procedures: schema.array(ProcedureDescription),
    }),
);
/** The procedures of a service. */
export type RouterDescription = schema.Infer<typeof RouterDescription>;

/** A declared service. */
export const ServiceDescription = defineSchema(
    schema.object({
        /** The package-local service name. */
        name: DeclarationName,
        /** The oldest caller release the service serves, every release when absent. */
        since: Version.optional(),
        /** The service transport. */
        protocol: schema.literal("http"),
        /** The service's procedures. */
        api: RouterDescription,
        /** The route names of the object types the service serves. */
        objects: schema.array(schema.string().min(1)),
        /** The service's own route names beside its objects. */
        routes: schema.array(schema.string().min(1)),
    }),
);
/** A declared service. */
export type ServiceDescription = schema.Infer<typeof ServiceDescription>;

/** Describe a service. */
export function describeService(service: Service): ServiceDescription {
    // name the routes the objects derive, shared ones included
    const derived = new Set(Object.keys(service.objects));
    for (const routed of Object.values(service.objects)) {
        for (const route of Object.keys(routed.shared)) {
            derived.add(route);
        }
    }

    return ServiceDescription.parse({
        name: service.name,
        ...(service.since === undefined ? {} : { since: service.since }),
        protocol: service.protocol,
        api: describeRouter(service.name, service.router),
        objects: Object.keys(service.objects),
        routes: Object.keys(service.router).filter((route) => !derived.has(route)),
    });
}

/** Describe a router. */
export function describeRouter(name: string, service: ServiceRouter): RouterDescription {
    // describe the procedures
    const procedures = describeProcedures(service);

    return RouterDescription.parse({ name, procedures });
}
