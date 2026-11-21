import { ModuleMetadata, type PackageVersion } from "@destack/package";
import { InstallationDefinition } from "./installation.ts";
import { StackDefinition } from "./definition.ts";
import { type JsonObject, JsonValue, type schema } from "@destack/schema";
import { StackError } from "../error/index.ts";
import { ObjectType, type Package, relationship, role, type StackInputOf } from "@destack/object";
import { setting } from "@destack/setting/object";
import { bucket, database, installation, route } from "../object/index.ts";

/** The entries of a stack document beside its collections of records. */
export interface StackInput {
    /** Independently configured package installations. */
    readonly installations?: Readonly<Record<string, schema.Input<typeof InstallationDefinition>>>;
    /** The existing records the stack takes over, by address and identifier. */
    readonly adopted?: Readonly<Record<string, string>>;
    /** The addresses of the records the stack leaves in place, unmanaged. */
    readonly released?: readonly string[];
}

/** The object types every stack declares records of without listing them. */
type BaseTypes = readonly [
    typeof database,
    typeof bucket,
    typeof route,
    typeof role,
    typeof relationship,
    typeof setting,
];

/** The option listing a stack's types beside its document, which no type's plural may take. */
const STACK_TYPES_KEY = "types";

/** The entries of a stack document about which records it owns, beside its collections. */
const OWNERSHIP_KEYS: ReadonlySet<string> = new Set(["adopted", "released"]);

/** What a stack declares records of: a package's object types, or one object type. */
export type StackRecordType = Package | ObjectType;

/** A stack document: its installations and ownership, and a collection per declared object type. */
export type StackDocument<Types extends readonly StackRecordType[] = readonly []> = StackInput & {
    readonly [
        Object in ObjectsOf<[...BaseTypes, ...Types][number]> as string extends Object["plural"]
            ? never
            : Object["plural"]
    ]?: Readonly<Record<string, StackInputOf<Object>>>;
};

/** Stack documents of named object types. */
export const StackDocument = {
    /** Serialise a document to JSON, refusing a collection no named type declares. */
    of<const Types extends readonly StackRecordType[]>(
        types: Types,
        document: StackDocument<Types>,
    ): JsonObject {
        // read the named object types beside the ones every stack declares, and their plurals
        const objects = [...baseTypes(), ...types].flatMap((named) =>
            ObjectType.is(named)
                ? [named]
                : Object.values(named.objects).filter((object) => ObjectType.is(object)),
        );
        const plurals = new Set(objects.map((object) => object.plural));

        // refuse a type whose plural is the reserved types option, and another collection
        const reserved = objects.find((object) => object.plural === STACK_TYPES_KEY);
        if (reserved !== undefined) {
            throw StackError.of(
                "DEFINITION_INVALID",
                `type '${reserved.name}' is invalid: its plural '${STACK_TYPES_KEY}' is the option listing a stack's types`,
            );
        }
        const unknown = Object.keys(document).find(
            (key) => !OWNERSHIP_KEYS.has(key) && !plurals.has(key),
        );
        if (unknown !== undefined) {
            throw StackError.of(
                "DEFINITION_INVALID",
                `type '${unknown}' is invalid: no named type has it`,
            );
        }

        return StackDefinition.parse(JsonValue.of(document));
    },
};

/** List the object types every stack declares records of, with its installations. */
function baseTypes(): StackRecordType[] {
    return [database, bucket, route, role, relationship, setting, installation];
}

/** The object types of a package or an object type itself. */
type ObjectsOf<Named> = Named extends ObjectType
    ? Named
    : Named extends { readonly objects: infer Objects }
      ? Extract<Objects[keyof Objects], ObjectType>
      : never;

/** A stack's declarations with the package declaring them. */
export interface StackDeclaration {
    /** The declaring package, supplied by the module transform. */
    readonly package: PackageVersion;
    /** The definition as JSON, keyed by collection. */
    readonly definition: JsonObject;
}

/** A stack as declared: its object types, and its document keyed by their plurals. */
export type StackOptions<Types extends readonly StackRecordType[]> = {
    /** The packages whose object types the stack declares, or single object types. */
    readonly types: Types;
} & StackDocument<Types>;

/** Define a stack's declarations of the given object types, as JSON. */
export function defineStack<const Types extends readonly StackRecordType[]>(
    options: StackOptions<Types>,
    module?: ModuleMetadata,
): StackDeclaration;
/** Define a stack's declarations, as the signature above types them. */
export function defineStack(
    options: StackOptions<readonly StackRecordType[]>,
    module?: ModuleMetadata,
): StackDeclaration {
    // split the types from the document keyed by collection
    const { types, ...document } = options;

    return {
        package: ModuleMetadata.require(module, "defineStack").package,
        definition: StackDocument.of(types, document),
    };
}
