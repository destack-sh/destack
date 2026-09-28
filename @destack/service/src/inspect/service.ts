import { defineSchema, schema } from "@destack/schema";
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
        /** The declaration format version. */
        version: schema.literal(1),
        /** The service transport. */
        protocol: schema.literal("http"),
        /** The service's procedures. */
        api: RouterDescription,
    }),
);
/** A declared service. */
export type ServiceDescription = schema.Infer<typeof ServiceDescription>;

/** Describe a service. */
export function describeService(service: Service): ServiceDescription {
    return ServiceDescription.parse({
        name: service.name,
        version: service.version,
        protocol: service.protocol,
        api: describeRouter(service.name, service.router),
    });
}

/** Describe a router. */
export function describeRouter(name: string, service: ServiceRouter): RouterDescription {
    // describe the procedures
    const procedures = describeProcedures(service);

    return RouterDescription.parse({ name, procedures });
}
