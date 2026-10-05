import { defineSchema, schema, Version } from "@destack/schema";
import { DeclarationName } from "@destack/package";

/** A JSON Schema. */
const JsonSchema = schema.record(schema.string(), schema.json());

/** A procedure's value or event-stream schema. */
export const PayloadDescription = defineSchema(
    schema.union([
        schema.object({
            /** A single validated value. */
            kind: schema.literal("value"),
            /** The value's JSON Schema. */
            schema: JsonSchema,
        }),
        schema.object({
            /** An event iterator. */
            kind: schema.literal("stream"),
            /** The yielded event schema. */
            yields: JsonSchema,
            /** The return value schema. */
            returns: JsonSchema.exactOptional(),
        }),
    ]),
);

/** A procedure's address, payloads and errors. */
export const ProcedureDescription = defineSchema(
    schema.object({
        /** The procedure's key path. */
        name: schema.array(schema.string().min(1)),
        /** The HTTP method. */
        method: schema.string().exactOptional(),
        /** The HTTP path. */
        path: schema.string().exactOptional(),
        /** The OpenAPI operation identifier. */
        operationId: schema.string().exactOptional(),
        /** The procedure annotations. */
        metadata: schema.record(schema.string(), schema.json()).exactOptional(),
        /** The input. */
        input: PayloadDescription.exactOptional(),
        /** The output. */
        output: PayloadDescription.exactOptional(),
        /** The error codes and payloads. */
        errors: schema.record(
            schema.string(),
            schema.object({
                /** The HTTP status. */
                status: schema.number().int().exactOptional(),
                /** The default error message. */
                message: schema.string().exactOptional(),
                /** The error payload schema. */
                data: JsonSchema.exactOptional(),
            }),
        ),
    }),
);
/** A procedure's address, payloads and errors. */
export type ProcedureDescription = schema.Infer<typeof ProcedureDescription>;

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
        since: Version.exactOptional(),
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
