import { ModuleMetadata, type PackageVersion } from "@destack/package";
import { InstallationDefinition } from "./installation.ts";
import { StackDefinition } from "./definition.ts";
import { type JsonObject, JsonValue, type schema } from "@destack/schema";
import { StackError } from "../error/index.ts";
import {
    ObjectType,
    type Package,
    type relationship,
    type role,
    type StackInputOf,
} from "@destack/object";
import type { setting } from "@destack/setting/object";
import type { bucket, database, route } from "../object/index.ts";

/** The collections of the stack package's own object types that `defineStack` takes. */
export interface StackInput {
    /** The databases the stack provisions. */
    readonly databases?: Readonly<Record<string, StackInputOf<typeof database>>>;
    /** The buckets the stack provisions. */
    readonly buckets?: Readonly<Record<string, StackInputOf<typeof bucket>>>;
    /** Independently configured package installations. */
    readonly installations?: Readonly<Record<string, schema.Input<typeof InstallationDefinition>>>;
    /** Roles defined in the space. */
    readonly roles?: Readonly<Record<string, StackInputOf<typeof role>>>;
    /** Roles bound and relations granted to members, groups and workloads. */
    readonly relationships?: Readonly<Record<string, StackInputOf<typeof relationship>>>;
    /** Routes under domains the space's account administers. */
    readonly routes?: Readonly<Record<string, StackInputOf<typeof route>>>;
    /** The setting values the stack places in its space. */
    readonly settings?: Readonly<Record<string, StackInputOf<typeof setting>>>;
    /** The existing records the stack takes over, by address and identifier. */
    readonly adopted?: Readonly<Record<string, string>>;
    /** The addresses of the records the stack leaves in place, unmanaged. */
    readonly released?: readonly string[];
}

/** The option listing a stack's types beside its document, which no type's plural may take. */
const STACK_TYPES_KEY = "types";

/** The keys of the collections and ownership entries the stack package reads in every document. */
const STACK_KEYS: ReadonlySet<string> = new Set([
    "databases",
    "buckets",
    "installations",
    "roles",
    "relationships",
    "routes",
    "settings",
    "adopted",
    "released",
]);

/** What a stack declares records of: a package's object types, or one object type. */
export type StackTypes = readonly (Package | ObjectType)[];

/** A stack document: the stack package's collections and one per object type it declares. */
export type StackDocument<Types extends StackTypes = readonly []> = StackInput & {
    readonly [
        Object in ObjectsOf<Types[number]> as string extends Object["plural"]
            ? never
            : Object["plural"]
    ]?: Readonly<Record<string, StackInputOf<Object>>>;
};

/** Stack documents of named object types. */
export const StackDocument = {
    /** Serialise a document to JSON, refusing a collection no named type declares. */
    of<const Types extends StackTypes>(types: Types, document: StackDocument<Types>): JsonObject {
        // read the named object types and their plurals
        const objects = types.flatMap((named) =>
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
            (key) => !STACK_KEYS.has(key) && !plurals.has(key),
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
export type StackOptions<Types extends StackTypes> = {
    /** The packages whose object types the stack declares, or single object types. */
    readonly types: Types;
} & StackDocument<Types>;

/** Define a stack's declarations of the given object types, as JSON. */
export function defineStack<const Types extends StackTypes>(
    options: StackOptions<Types>,
    module?: ModuleMetadata,
): StackDeclaration;
/** Define a stack's declarations, as the signature above types them. */
export function defineStack(
    options: StackOptions<StackTypes>,
    module?: ModuleMetadata,
): StackDeclaration {
    // split the types from the document keyed by collection
    const { types, ...document } = options;

    return {
        package: ModuleMetadata.require(module, "defineStack").package,
        definition: StackDocument.of(types, document),
    };
}
