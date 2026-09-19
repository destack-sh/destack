import { defineSchema, schema } from "@destack/schema";
import { ResourceName } from "@destack/resource";

/** An HTTP service handled by a workload's exported function. */
export const ServiceDeclaration = defineSchema(schema.object({
    /** The package-local service name. */
    name: ResourceName,
    /** The declaration format version. */
    version: schema.literal(1),
    /** The exported function accepting a Request and returning a Response. */
    handler: schema.string().min(1),
    /** The service transport. */
    protocol: schema.literal("http"),
}));
/** An HTTP service handled by a workload's exported function. */
export type ServiceDeclaration = schema.Infer<typeof ServiceDeclaration>;

/** Declare an HTTP service for workload routing. */
export function defineService(declaration: ServiceDeclaration): ServiceDeclaration {
    return ServiceDeclaration.parse(declaration);
}
