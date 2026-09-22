import { defineSchema, schema, toJsonSchema } from "@destack/schema";
import { type AnySchema, getEventIteratorSchemaDetails, isContractProcedure } from "@orpc/contract";
import type { Service } from "../service/index.ts";

/** A JSON Schema describing a procedure value. */
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
            /** The final return value schema. */
            returns: JsonSchema.optional(),
        }),
    ]),
);

/** A procedure's address, payloads, and declared errors. */
export const ProcedureDescription = defineSchema(
    schema.object({
        /** The procedure's key path within the service. */
        name: schema.array(schema.string().min(1)),
        /** The explicitly declared HTTP method. */
        method: schema.string().optional(),
        /** The explicitly declared HTTP path. */
        path: schema.string().optional(),
        /** The explicitly declared OpenAPI operation identifier. */
        operationId: schema.string().optional(),
        /** Serializable procedure annotations, including authentication and audit requirements. */
        metadata: schema.record(schema.string(), schema.json()).optional(),
        /** The accepted input, when a validator is declared. */
        input: PayloadDescription.optional(),
        /** The returned output, when a validator is declared. */
        output: PayloadDescription.optional(),
        /** Declared error codes and their payloads. */
        errors: schema.record(
            schema.string(),
            schema.object({
                /** The explicitly declared HTTP status. */
                status: schema.number().int().optional(),
                /** The default error message. */
                message: schema.string().optional(),
                /** The error payload schema. */
                data: JsonSchema.optional(),
            }),
        ),
    }),
);
/** A procedure's address, payloads, and declared errors. */
export type ProcedureDescription = schema.Infer<typeof ProcedureDescription>;

/** Describe each procedure without executing its handler. */
export function describeProcedures(service: Service): ProcedureDescription[] {
    // collect declared procedures in stable key order
    const procedures: ProcedureDescription[] = [];
    visit(service, [], new Set(), procedures);

    return procedures;
}

/** Traverse nested routers while rejecting recursive router objects. */
function visit(
    service: Service,
    name: string[],
    ancestors: Set<Service>,
    procedures: ProcedureDescription[],
): void {
    if (ancestors.has(service)) {
        throw new TypeError(`Cyclic service definition: ${name.join(".")}`);
    }

    // preserve explicitly declared schemas independently of generated OpenAPI defaults
    if (isContractProcedure(service)) {
        // describe declared error payloads
        const definition = service["~orpc"];
        const errors = Object.fromEntries(
            Object.entries(definition.errorMap).map(([code, value]) => {
                const error = value as { status?: number; message?: string; data?: AnySchema };

                return [
                    code,
                    {
                        status: error.status,
                        message: error.message,
                        data: error.data ? describeSchema(error.data) : undefined,
                    },
                ];
            }),
        );

        // retain routes, access annotations, and input/output schemas
        procedures.push(
            ProcedureDescription.parse({
                name,
                method: definition.route.method,
                path: definition.route.path,
                operationId: definition.route.operationId,
                metadata: Object.keys(definition.meta).length > 0 ? definition.meta : undefined,
                input: describePayload(definition.inputSchema),
                output: describePayload(definition.outputSchema),
                errors,
            }),
        );
    }
    // traverse nested routers while detecting ancestor cycles
    else {
        ancestors.add(service);
        for (const key of Object.keys(service).sort()) {
            visit(service[key], [...name, key], ancestors, procedures);
        }
        ancestors.delete(service);
    }
}

/** Describe a declared value validator or event iterator. */
function describePayload(
    validator: AnySchema | undefined,
): schema.Infer<typeof PayloadDescription> | undefined {
    if (!validator) {
        return undefined;
    }

    // distinguish streamed events from single response values
    const iterator = getEventIteratorSchemaDetails(validator);

    return iterator
        ? {
              kind: "stream",
              yields: describeSchema(iterator.yields),
              returns: iterator.returns ? describeSchema(iterator.returns) : undefined,
          }
        : { kind: "value", schema: describeSchema(validator) };
}

/** Require the portable schema definitions used throughout Destack. */
function describeSchema(validator: AnySchema): schema.Infer<typeof JsonSchema> {
    if (!(validator instanceof schema.Schema)) {
        throw new TypeError("expected a Destack schema");
    }

    return JsonSchema.parse(toJsonSchema(validator));
}
