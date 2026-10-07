import { describePayload, PayloadDescription } from "@destack/service/inspect";
import { Scope, AccessName } from "@destack/sync";
import { PolicyDescription, describePolicy } from "@destack/access/inspect";
import { AuditActionDescription, describeAuditAction } from "@destack/audit/inspect";
import { TABLE } from "@destack/db";
import { graph } from "@destack/package";
import { defineSchema, present, schema, toJsonSchema, type JsonValue } from "@destack/schema";

import { FIELD_TYPES } from "../field/field.ts";
import { METHOD_KINDS } from "../method/kind.ts";
import { BucketName } from "../field/file.ts";
import { Presentation } from "../trait/presentable.ts";
import { ObjectSearch } from "../trait/search.ts";
import type { ObjectType } from "../object/object.ts";

/** One method as the manifest describes it. */
export const MethodDescription = defineSchema(
    schema.object({
        /** The method's kind. */
        kind: schema.enum(METHOD_KINDS),
        /** The states a transition leaves and the state it enters. */
        transition: schema
            .object({ from: schema.array(schema.string()), to: schema.string() })
            .exactOptional(),
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
        audit: AuditActionDescription.exactOptional(),
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
        /** The prefix of the objects' identifiers. */
        identity: schema.string().min(1),
        /** The scope levels the objects live in, none for any scope. */
        scope: schema.array(AccessName),
        /** The SQL name of the table with the records. */
        table: schema.string().min(1),
        /** Where the objects live beside durable ones: in instances' memory, or in files read per scope. */
        storage: schema.enum(["ephemeral", "external"]).exactOptional(),
        /** The input field naming the objects' scope, absent for objects of the universe. */
        scopeField: schema.string().min(1).exactOptional(),
        /** The SQL column of each row property. */
        columns: schema.record(schema.string(), schema.string().min(1)),
        /** The permission names callers may have. */
        permissions: schema.array(schema.string().min(1)),
        /** The relations and permissions access evaluates. */
        policy: PolicyDescription,
        /** The schema of one stack declaration, as JSON Schema. */
        declaration: schema.json().exactOptional(),
        /** The methods callers may call, by name. */
        methods: schema.record(schema.string(), MethodDescription),
        /** The audit action recording each watch of read-audited objects. */
        watch: AuditActionDescription.exactOptional(),
        /** How pickers, mentions and titles show the objects, absent for a type presenting none. */
        presentation: Presentation.exactOptional(),
        /** The fields full-text search ranks, lists filter and lists order the objects by. */
        search: ObjectSearch.exactOptional(),
        /** The fields callers read and write, by row property. */
        fields: schema.record(
            schema.string(),
            schema.object({
                /** The field's meaning beyond its stored value. */
                type: schema.enum(FIELD_TYPES),
                /** The principal kinds a principal reference or subject field accepts, by name. */
                principals: schema.array(schema.string().min(1)).exactOptional(),
                /** The bucket a file field keeps its files in, by its declaring package and name. */
                bucket: BucketName.exactOptional(),
                /** The permission a caller needs to read the value. */
                read: schema.string().min(1).exactOptional(),
                /** The permission a caller needs to write the value. */
                write: schema.string().min(1).exactOptional(),
            }),
        ),
    }),
);
/** An object type as the manifest describes it. */
export type ObjectDescription = schema.Infer<typeof ObjectDescription>;

/** Describe an object type for the manifest. */
export function describeObject(object: ObjectType): ObjectDescription {
    // describe each method from its procedure
    const methods: Record<string, MethodDescription> = {};
    for (const [name, declared] of Object.entries(object.methods)) {
        if (declared.isSystem === true) {
            continue;
        }
        methods[name] = describeMethod(object, name, declared);
    }

    return ObjectDescription.parse({
        name: object.name,
        plural: object.plural,
        identity: object.identity,
        scope:
            object.scope === Scope.universe.id
                ? [Scope.universe.id]
                : object.scopes.map((scope) => scope.name),
        table: object.table[TABLE].sqlName,
        ...(object.storage === "durable" ? {} : { storage: object.storage }),
        ...(object.route.field === undefined ? {} : { scopeField: object.route.field }),
        columns: Object.fromEntries(
            Object.entries(object.table[TABLE].columns).map(([property, column]) => [
                property,
                column.definition.name,
            ]),
        ),
        permissions: [...object.permissions],
        policy: describePolicy(object.policy),
        ...(object.lifecycle.declarationSchema
            ? { declaration: toJsonSchema(object.lifecycle.declarationSchema) }
            : {}),
        methods,
        ...(object.isReadAudited
            ? { watch: describeAuditAction(object.audit("watch", "collection")) }
            : {}),
        ...(object.presentation === undefined ? {} : { presentation: object.presentation }),
        ...(object.search === undefined ? {} : { search: object.search }),
        fields: Object.fromEntries(
            Object.entries(object.fields).map(([name, declared]) => [
                name,
                {
                    type: declared.type,
                    ...(declared.principals === undefined
                        ? {}
                        : { principals: declared.principals.map((principal) => principal.name) }),
                    ...(declared.bucket === undefined
                        ? {}
                        : { bucket: BucketName.of(declared.bucket) }),
                    ...declared.access,
                },
            ]),
        ),
    });
}

/** Describe one method of an object type from its procedure. */
function describeMethod(
    object: ObjectType,
    name: string,
    declared: ObjectType["methods"][string],
): MethodDescription {
    // read the method's procedure
    const definition = present(object.procedures[name], `the procedure of ${name}`)["~orpc"];

    return {
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
        route: {
            method: present(definition.route.method, `the route method of ${name}`),
            path: present(definition.route.path, `the route path of ${name}`),
        },
        input: schema
            .json()
            .parse(toJsonSchema(present(definition.inputSchema, `the input of ${name}`))),
        output: present(
            describePayload(present(definition.outputSchema, `the output of ${name}`)),
            `the output description of ${name}`,
        ),
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

/** List an object type's terms: its name, relations, permissions and methods. */
export function objectVocabulary(input: Record<string, JsonValue>): Record<string, JsonValue> {
    // define each term by the shape stored rows depend on
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

/** List an object type's methods as its member symbols, each reading or writing the type's table. */
export function objectSymbols(input: Record<string, JsonValue>): graph.MemberSymbol[] {
    const object = ObjectDescription.parse(input);

    return schema.array(graph.MemberSymbol).parse(
        Object.entries(object.methods).map(([name, method]) => ({
            member: { kind: "method", name, description: method },
            relationships: [
                {
                    kind: method.mutates ? "writes" : "reads",
                    symbol: { kind: "table", name: object.table },
                },
            ],
        })),
    );
}
