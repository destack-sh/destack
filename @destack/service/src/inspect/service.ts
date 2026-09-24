import { defineSchema, schema } from "@destack/schema";
import type { ServiceRouter } from "../service/index.ts";
import { describeProcedures, ProcedureDescription } from "./procedure.ts";
import type { Service } from "../declare/service.ts";
import { DeclarationName } from "@destack/package";

/** Routes, payloads and errors declared by a named service. */
export const RouterDescription = defineSchema(
    schema.object({
        /** The package-local service name. */
        name: schema.string().min(1),
        /** Procedure addresses, declared payloads, and errors. */
        procedures: schema.array(ProcedureDescription),
    }),
);
/** Routes, payloads and errors declared by a named service. */
export type RouterDescription = schema.Infer<typeof RouterDescription>;

/** A declared service and its inspected API. */
export const ServiceDescription = defineSchema(
    schema.object({
        /** The package-local service name. */
        name: DeclarationName,
        /** The declaration format version. */
        version: schema.literal(1),
        /** The service transport. */
        protocol: schema.literal("http"),
        /** Procedures declared by the service's router. */
        api: RouterDescription,
    }),
);
/** A declared service and its inspected API. */
export type ServiceDescription = schema.Infer<typeof ServiceDescription>;

/** Describe a declared service and its procedures. */
export function describeService(service: Service): ServiceDescription {
    return ServiceDescription.parse({
        name: service.name,
        version: service.version,
        protocol: service.protocol,
        api: describeRouter(service.name, service.router),
    });
}

/** Describe service routes and application schemas. */
export function describeRouter(name: string, service: ServiceRouter): RouterDescription {
    // retain application schemas and declared errors
    const procedures = describeProcedures(service);
    return RouterDescription.parse({ name, procedures });
}
