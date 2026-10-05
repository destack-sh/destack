import { schema, toJsonSchema, type JsonValue } from "@destack/schema";
import { type AnySchema, getEventIteratorSchemaDetails, isContractProcedure } from "@orpc/contract";
import type { ServiceRouter } from "../service/index.ts";
import { PayloadDescription, ProcedureDescription } from "../service/description.ts";

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
        const definition = definitionOf(service);
        const errors = Object.fromEntries(
            Object.entries(definition.errorMap).map(([code, error]) => {
                return [
                    code,
                    schema.defined({
                        status: error.status,
                        message: error.message,
                        data: error.data === undefined ? undefined : describeSchema(error.data),
                    }),
                ];
            }),
        );

        // add the description
        procedures.push(
            ProcedureDescription.parse(
                schema.defined({
                    name,
                    method: definition.route.method,
                    path: definition.route.path,
                    operationId: definition.route.operationId,
                    metadata: Object.keys(definition.meta).length > 0 ? definition.meta : undefined,
                    input: describePayload(definition.inputSchema),
                    output: describePayload(definition.outputSchema),
                    errors,
                }),
            ),
        );
    }
    // visit nested routers
    else {
        ancestors.add(service);
        const children = Object.entries(service).toSorted(([left], [right]) =>
            left < right ? -1 : 1,
        );
        for (const [key, child] of children) {
            visit(child, [...name, key], ancestors, procedures);
        }
        ancestors.delete(service);
    }
}

/** A procedure's definition as oRPC keeps it under `~orpc`, its validators Standard Schemas. */
const Definition = schema.looseObject({
    /** The route the procedure answers. */
    route: schema.looseObject({
        /** The HTTP method. */
        method: schema.string().exactOptional(),
        /** The path template. */
        path: schema.string().exactOptional(),
        /** The operation's identifier. */
        operationId: schema.string().exactOptional(),
    }),
    /** The procedure's metadata. */
    meta: schema.record(schema.string(), schema.json()),
    /** The procedure's declared errors by code. */
    errorMap: schema.record(
        schema.string(),
        schema.looseObject({
            /** The HTTP status. */
            status: schema.number().int().exactOptional(),
            /** The default message. */
            message: schema.string().exactOptional(),
            /** The error data's schema. */
            data: schema.standard().exactOptional(),
        }),
    ),
    /** The input's schema. */
    inputSchema: schema.standard().exactOptional(),
    /** The output's schema. */
    outputSchema: schema.standard().exactOptional(),
});

/** Read a procedure's definition, refusing one oRPC did not build. */
function definitionOf(procedure: object) {
    return Definition.parse(Object.getOwnPropertyDescriptor(procedure, "~orpc")?.value);
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
              ...(iterator.returns ? { returns: describeSchema(iterator.returns) } : {}),
          }
        : { kind: "value", schema: describeSchema(validator) };
}

/** Convert a schema to JSON Schema. */
function describeSchema(validator: AnySchema): Record<string, JsonValue> {
    // reject validators of other schema libraries
    if (!(validator instanceof schema.Schema)) {
        throw new TypeError("expected a Destack schema");
    }

    return schema.record(schema.string(), schema.json()).parse(toJsonSchema(validator));
}
