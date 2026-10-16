import { TABLE } from "@destack/db";
import { graph, type PackageId } from "@destack/package";
import { schema, type JsonValue } from "@destack/schema";
import { ObjectDescription, type KnownDeclaration } from "../object/description.ts";
import { ObjectType } from "../object/object.ts";

/** The kind of the declarations `defineObject` makes, as manifests list them. */
export const OBJECT_DECLARATION = "object";

/** A declaration an installation's release makes, as the space stores it. */
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

/** Declare the object types an installation describes, against the declarations known here. */
export function declareInstalled(
    declarations: readonly InstalledDeclaration[],
    known: Iterable<KnownDeclaration>,
): ObjectType[] {
    const descriptions = declarations
        .filter((entry) => entry.kind === OBJECT_DECLARATION)
        .map((entry) => ObjectDescription.parse(entry.description));

    return ObjectType.ofAll(descriptions, known);
}

/** List an object type's terms: its name, relations, permissions and methods. */
export function objectVocabulary(object: ObjectType): Record<string, JsonValue> {
    // define each term by the shape stored rows depend on
    const { name } = object;
    const policy = object.policy.definition;
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
        Object.entries(object.describeMethods()).map(([name, described]) => ({
            member: { kind: "method", name, description: described },
            relationships: [
                {
                    kind: described.mutates ? "writes" : "reads",
                    symbol: { kind: "table", name: table },
                },
            ],
        })),
    );
}
