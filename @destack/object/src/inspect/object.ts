import { describePolicy } from "@destack/access/inspect";
import { TABLE } from "@destack/db";
import { graph, type PackageId } from "@destack/package";
import { RESOURCE_DECLARATION, ResourceDeclaration, ResourceDescription } from "@destack/resource";
import { present, schema, toJsonSchema, type JsonObject, type JsonValue } from "@destack/schema";
import { ObjectDescription, type KnownDeclaration } from "../object/description.ts";
import { ObjectType } from "../object/object.ts";
import type { FileBucket } from "../field/file.ts";

/** The kind of the declarations `defineObject` makes, as manifests list them. */
export const OBJECT_DECLARATION = "object";

/** A declaration an installation's release makes, as the space keeps it. */
export interface InstalledDeclaration {
    /** The declaration's kind. */
    readonly kind: string;
    /** The declaring package. */
    readonly packageId: PackageId;
    /** The declaration's description. */
    readonly description: JsonValue;
}

/** Describe an object type for the manifest, as the data it is declared from. */
export function describeObject(object: ObjectType): ObjectDescription {
    return object.describe();
}

/** Declare the object types an installation describes, against the platform's declarations. */
export function declareInstalled(
    declarations: readonly InstalledDeclaration[],
    platform: Iterable<KnownDeclaration>,
): ObjectType[] {
    // read the object types, and the packages their descriptions name
    const descriptions = declarations
        .filter((entry) => entry.kind === OBJECT_DECLARATION)
        .map((entry) => ObjectDescription.parse(entry.description));
    const packages = new Map(descriptions.map((each) => [each.package.id, each.package]));

    // build the buckets of those packages, which file fields refer to
    const buckets = declarations
        .filter((entry) => entry.kind === RESOURCE_DECLARATION)
        .flatMap((entry): FileBucket[] => {
            // read each resource's base shape, which every kind's description extends
            const { kind } = ResourceDescription.loose().parse(entry.description);
            const owner = packages.get(entry.packageId);
            if (kind !== "bucket" || owner === undefined) {
                return [];
            }

            // keep a bucket, whose description is the base shape alone
            const bucket = ResourceDescription.parse(entry.description);

            return [new ResourceDeclaration(owner, { ...bucket, kind: "bucket" })];
        });

    return ObjectType.ofAll(descriptions, [...platform, ...buckets]);
}

/** List an object type's terms: its name, relations, permissions and methods. */
export function objectVocabulary(object: ObjectType): Record<string, JsonValue> {
    // define each term by the shape stored rows depend on
    const { name } = object;
    const policy = describePolicy(object.policy);
    const terms: Record<string, JsonValue> = { [name]: { table: object.table[TABLE].sqlName } };
    for (const [relation, declared] of Object.entries(policy.relations)) {
        terms[`${name}/relation/${relation}`] = { subjects: declared.subjects };
    }
    for (const permission of Object.keys(policy.permissions)) {
        terms[`${name}/permission/${permission}`] = {};
    }
    for (const [method, declared] of Object.entries(object.methods)) {
        if (declared.isInternal !== true) {
            terms[`${name}/method/${method}`] = { kind: declared.kind };
        }
    }

    return terms;
}

/** List an object type's methods as member symbols that read or write the type's table. */
export function objectSymbols(object: ObjectType): graph.MemberSymbol[] {
    const table = object.table[TABLE].sqlName;

    return schema.array(graph.MemberSymbol).parse(
        Object.entries(object.methods)
            .filter(([, declared]) => declared.isInternal !== true)
            .map(([name, declared]) => ({
                member: { kind: "method", name, description: describeMethod(object, name) },
                relationships: [
                    {
                        kind: declared.mutates ? "writes" : "reads",
                        symbol: { kind: "table", name: table },
                    },
                ],
            })),
    );
}

/** Describe a method as callers reach it: its kind, permission, route and input as JSON Schema. */
function describeMethod(object: ObjectType, name: string): JsonObject {
    // read the method and the procedure serving it
    const declared = present(object.methods[name], `the method ${name}`);
    const procedure = present(object.procedures[name], `the procedure of ${name}`)["~orpc"];
    const input = schema.instanceof(schema.Schema).parse(procedure.inputSchema);

    return {
        kind: declared.kind,
        permission: declared.permission,
        mutates: declared.mutates,
        route: {
            method: present(procedure.route.method, `the route method of ${name}`),
            path: present(procedure.route.path, `the route path of ${name}`),
        },
        input: toJsonSchema(input),
    };
}
