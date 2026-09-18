import { defineSchema, schema } from "@destack/schema";
import { createDocument, type DocumentOptions } from "../openapi/index.ts";
import type { Service } from "../service/index.ts";
import { describeProcedures, ProcedureDescription } from "./procedure.ts";

/** The generated OpenAPI document for a named service. */
export const ServiceDescription = defineSchema(schema.object({
    /** The package-local service name. */
    name: schema.string().min(1),
    /** The service description format version. */
    version: schema.literal(1),
    /** Procedure addresses, declared payloads, and errors. */
    procedures: schema.array(ProcedureDescription),
    /** The OpenAPI 3.1 document, including routes, schemas, and errors. */
    document: schema.record(schema.string(), schema.json()),
}));
/** The generated OpenAPI document for a named service. */
export type ServiceDescription = schema.Infer<typeof ServiceDescription>;

/** Inspect service routes and schemas using the same generator as HTTP documentation. */
export async function describeService(
    name: string,
    service: Service,
    options: DocumentOptions,
): Promise<ServiceDescription> {
    const procedures = describeProcedures(service);
    const document = await createDocument(service, options);

    // serialize OpenAPI optional properties using the generator's JSON representation
    const serialized = JSON.parse(JSON.stringify(document));

    return ServiceDescription.parse({ name, version: 1, procedures, document: serialized });
}
