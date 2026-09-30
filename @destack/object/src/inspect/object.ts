import { describePayload, PayloadDescription } from "@destack/service/inspect";
import { Scope } from "@destack/sync";
import { PolicyDescription, describePolicy } from "@destack/access/inspect";
import { AuditActionDescription, describeAuditAction } from "@destack/audit/inspect";
import { TABLE } from "@destack/db";
import { defineSchema, schema, toJsonSchema } from "@destack/schema";
import type { Method } from "../method/method.ts";
import { METHOD_KINDS } from "../method/kind.ts";
import type { ObjectType } from "../object/object.ts";
import { AccessName } from "@destack/access";
import type { JsonValue } from "@destack/schema/json";

/** One method as the manifest describes it. */
export const MethodDescription = defineSchema(
    schema.object({
        /** The method's kind. */
        kind: schema.enum(METHOD_KINDS),
        /** The states a transition leaves and the state it enters. */
        transition: schema
            .object({ from: schema.array(schema.string()), to: schema.string() })
            .optional(),
        /** The permission the caller needs on the target, or null for none. */
        permission: schema.string().min(1).nullable(),
        /** Whether the method changes state. */
        mutates: schema.boolean(),
        /** Whether clients predict the method. */
        isPredicted: schema.boolean(),
        /** The HTTP route. */
        route: schema.object({ method: schema.string(), path: schema.string() }),
        /** The complete input, as JSON Schema. */
        input: schema.json(),
        /** The result, as JSON Schema. */
        output: PayloadDescription,
        /** The audit action recording each audited call. */
        audit: AuditActionDescription.optional(),
    }),
);
/** One method as the manifest describes it. */
export type MethodDescription = schema.Infer<typeof MethodDescription>;

/** An object type as the manifest describes it. */
export const ObjectDescription = defineSchema(
    schema.object({
        /** The singular name. */
        name: schema.string().min(1),
        /** The plural name. */
        plural: schema.string().min(1),
        /** The scope levels containing the objects. */
        scope: schema.array(AccessName),
        /** The SQL name of the table holding the records. */
        table: schema.string().min(1),
        /** The permission names callers may hold. */
        permissions: schema.array(schema.string().min(1)),
        /** The relations and permissions access evaluates. */
        policy: PolicyDescription,
        /** The schema of one stack declaration, as JSON Schema. */
        declaration: schema.json().optional(),
        /** The methods callers may call, by name. */
        methods: schema.record(schema.string(), MethodDescription),
        /** The audit action recording each watch of read-audited objects. */
        watch: AuditActionDescription.optional(),
        /** The permissions reading or writing guarded fields requires, by field. */
        fields: schema.record(
            schema.string(),
            schema.object({
                /** The permission a caller needs to read the value. */
                read: schema.string().min(1).optional(),
                /** The permission a caller needs to write the value. */
                write: schema.string().min(1).optional(),
            }),
        ),
    }),
);
/** An object type as the manifest describes it. */
export type ObjectDescription = schema.Infer<typeof ObjectDescription>;

/** Describe an object type for the manifest. */
export function describeObject(object: ObjectType): ObjectDescription {
    // describe each method from its contract
    const procedures = object.procedures as Record<
        string,
        {
            readonly "~orpc": {
                readonly route: { readonly method: string; readonly path: string };
                readonly inputSchema: schema.Schema;
                readonly outputSchema: schema.Schema;
            };
        }
    >;
    const methods: Record<string, MethodDescription> = {};
    for (const [name, declared] of Object.entries(object.methods) as [string, Method][]) {
        if (declared.isSystem) {
            continue;
        }
        const contract = procedures[name]!["~orpc"];
        methods[name] = {
            kind: declared.kind,
            permission: declared.permission,
            mutates: declared.mutates,
            isPredicted: declared.isPredicted,
            ...(declared.transition
                ? {
                      transition: {
                          from: [...declared.transition.from],
                          to: declared.transition.to,
                      },
                  }
                : {}),
            route: { method: contract.route.method, path: contract.route.path },
            input: schema.json().parse(toJsonSchema(contract.inputSchema)),
            output: describePayload(contract.outputSchema)!,
            ...(declared.mutates || object.isReadAudited
                ? {
                      audit: describeAuditAction(
                          declared.mutates || declared.target
                              ? object.audit(name)
                              : object.audit(name, "collection"),
                      ),
                  }
                : {}),
        };
    }

    return ObjectDescription.parse({
        name: object.name,
        plural: object.plural,
        scope:
            object.scope === Scope.universe.id
                ? [Scope.universe.id]
                : object.scopes.map((scope) => scope.name),
        table: object.table[TABLE].sqlName,
        permissions: [...object.permissions],
        policy: describePolicy(object.policy),
        ...(object.declarationSchema
            ? { declaration: toJsonSchema(object.declarationSchema) }
            : {}),
        methods,
        ...(object.isReadAudited
            ? { watch: describeAuditAction(object.audit("watch", "collection")) }
            : {}),
        fields: Object.fromEntries(
            Object.entries(object.fields).flatMap(([name, declared]) =>
                declared.access === undefined ? [] : [[name, { ...declared.access }]],
            ),
        ),
    });
}

/** List an object type's terms: its name, relations, permissions and methods. */
export function objectVocabulary(input: Record<string, JsonValue>): Record<string, JsonValue> {
    // define each term by the shape stored data depends on
    const description = ObjectDescription.parse(input);
    const { name, policy } = description;
    const terms: Record<string, JsonValue> = { [name]: { table: description.table } };
    for (const [relation, declared] of Object.entries(policy.relations)) {
        terms[`${name}/relation/${relation}`] = { subjects: declared.subjects };
    }
    for (const permission of Object.keys(policy.permissions)) {
        terms[`${name}/permission/${permission}`] = {};
    }
    for (const [method, declared] of Object.entries(description.methods)) {
        terms[`${name}/method/${method}`] = { kind: declared.kind };
    }

    return terms;
}
