import { defineSchema, schema } from "@destack/schema";
import type { Service } from "../service/index.ts";
import { describeProcedures, ProcedureDescription } from "./procedure.ts";
import { ServiceDeclaration, type ServiceDefinition } from "../declare/service.ts";

/** Routes, payloads and errors declared by a named service. */
export const ServiceDescription = defineSchema(
    schema.object({
        /** The package-local service name. */
        name: schema.string().min(1),
        /** Procedure addresses, declared payloads, and errors. */
        procedures: schema.array(ProcedureDescription),
    }),
);
/** Routes, payloads and errors declared by a named service. */
export type ServiceDescription = schema.Infer<typeof ServiceDescription>;

/** An HTTP handler declaration and its inspected API. */
export const ServiceInspection = defineSchema(
    ServiceDeclaration.extend({
        /** Procedures declared by the associated router. */
        api: ServiceDescription.optional(),
    }),
);
/** An HTTP handler declaration and its inspected API. */
export type ServiceInspection = schema.Infer<typeof ServiceInspection>;

/** Describe a declared handler and its associated procedures. */
export function inspectService(definition: ServiceDefinition): ServiceInspection {
    // separate the serializable declaration from its executable router
    const { router, ...metadata } = definition;
    const declaration = ServiceDeclaration.parse(metadata);
    const api = router ? describeService(definition.name, router) : undefined;

    return { ...declaration, ...(api ? { api } : {}) };
}

/** Describe service routes and application schemas. */
export function describeService(name: string, service: Service): ServiceDescription {
    // retain application schemas and declared errors
    const procedures = describeProcedures(service);
    return ServiceDescription.parse({ name, procedures });
}
