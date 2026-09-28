import { defineSchema, schema, toJsonSchema } from "@destack/schema";
import { type AnySchema, getEventIteratorSchemaDetails, isContractProcedure } from "@orpc/contract";
import type { ServiceRouter } from "../service/index.ts";

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
            returns: JsonSchema.optional(),
        }),
    ]),
);

/** A procedure's address, payloads and errors. */
export const ProcedureDescription = defineSchema(
    schema.object({
        /** The procedure's key path. */
        name: schema.array(schema.string().min(1)),
        /** The HTTP method. */
        method: schema.string().optional(),
        /** The HTTP path. */
        path: schema.string().optional(),
        /** The OpenAPI operation identifier. */
        operationId: schema.string().optional(),
        /** The procedure annotations. */
        metadata: schema.record(schema.string(), schema.json()).optional(),
        /** The input. */
        input: PayloadDescription.optional(),
        /** The output. */
        output: PayloadDescription.optional(),
        /** The error codes and payloads. */
        errors: schema.record(
            schema.string(),
            schema.object({
                /** The HTTP status. */
                status: schema.number().int().optional(),
                /** The default error message. */
                message: schema.string().optional(),
                /** The error payload schema. */
                data: JsonSchema.optional(),
            }),
        ),
    }),
);
/** A procedure's address, payloads and errors. */
export type ProcedureDescription = schema.Infer<typeof ProcedureDescription>;

/** Describe each procedure of a router. */
export function describeProcedures(service: ServiceRouter): ProcedureDescription[] {
    // collect the procedures
    const procedures: ProcedureDescription[] = [];
    visit(service, [], new Set(), procedures);

    return procedures;
}

/** Visit a router. */
function visit(
    service: ServiceRouter,
    name: string[],
    ancestors: Set<ServiceRouter>,
    procedures: ProcedureDescription[],
): void {
    // reject a router that contains itself
    if (ancestors.has(service)) {
        throw new TypeError(`cyclic service definition: ${name.join(".")}`);
    }

    // describe a procedure
    if (isContractProcedure(service)) {
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

        // add the description
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
    // visit nested routers
    else {
        ancestors.add(service);
        for (const key of Object.keys(service).sort()) {
            visit(service[key], [...name, key], ancestors, procedures);
        }
        ancestors.delete(service);
    }
}

/** Describe a value or event iterator schema. */
export function describePayload(
    validator: AnySchema | undefined,
): schema.Infer<typeof PayloadDescription> | undefined {
    // describe no payload
    if (!validator) {
        return undefined;
    }

    // describe a stream or a value
    const iterator = getEventIteratorSchemaDetails(validator);

    return iterator
        ? {
              kind: "stream",
              yields: describeSchema(iterator.yields),
              returns: iterator.returns ? describeSchema(iterator.returns) : undefined,
          }
        : { kind: "value", schema: describeSchema(validator) };
}

/** Convert a schema to JSON Schema. */
function describeSchema(validator: AnySchema): schema.Infer<typeof JsonSchema> {
    // reject validators of other schema libraries
    if (!(validator instanceof schema.Schema)) {
        throw new TypeError("expected a Destack schema");
    }

    return JsonSchema.parse(toJsonSchema(validator));
}
