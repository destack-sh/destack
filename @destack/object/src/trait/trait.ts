import {
    none,
    type AccessExpression,
    type Policy,
    type RelationInput,
    type TableMapping,
} from "@destack/access";
import type { schema } from "@destack/schema";
import type { Column, ColumnBuilder, Table, TableConstraint, TableOptions } from "@destack/db";
import type { PackageId } from "@destack/package";
import type { Method } from "../method/method.ts";
import type {
    ObjectDefinition,
    ObjectStorage,
    ObjectTraits,
    ObjectType,
} from "../object/object.ts";

/** The object a trait applies to while its type is assembled. */
export interface TraitObject {
    /** The object's singular name. */
    readonly name: string;
    /** The prefix of the object's identifiers. */
    readonly identity: string;
    /** The package identifying the object's policy. */
    readonly packageId: PackageId;
    /** The object's table, once derived. */
    readonly table: () => Table;
    /** Where the objects live. */
    readonly storage: ObjectStorage;
    /** The objects' natural key, absent for generated identifiers. */
    readonly key: schema.Schema<string> | undefined;
}

/** The table options a trait adds. */
export type TraitTable = Pick<TableOptions<never>, "dependents" | "aggregates" | "tree"> & {
    /** The text properties the table's full-text index keeps. */
    readonly fullText?: readonly string[];
};

/** What a trait adds to an object's policy. */
export interface TraitPolicy {
    /** The attributes of the trait's columns that policy conditions read. */
    readonly attributes?: Readonly<Record<string, "string" | "number" | "boolean">>;
    /** The relations the trait's columns keep. */
    readonly relations?: Readonly<Record<string, RelationInput>>;
    /** The permissions the trait derives. */
    readonly permissions?: Readonly<Record<string, AccessExpression>>;
    /** The open relations of other policies the objects join. */
    readonly contributes?: readonly { readonly policy: Policy; readonly relation: string }[];
}

/** Where access finds the relations a trait adds. */
export type TraitMapping = Pick<TableMapping, "relations"> &
    Partial<Pick<TableMapping, "parent" | "trees">>;

/** A trait whose methods the holders of one object permission call. */
export interface Gated<Permission extends string = string> {
    /** The permission whose holders call the trait's methods. */
    readonly by: Permission;
}

/** The permission a gated trait's definition sets, undefined without the trait. */
export type GateOf<Definition> =
    Definition extends Gated<infer Permission> ? Permission : undefined;

/** A definition with trait-rewritten members, each typed as any definition's after it. */
export type Erasure<Definition, Keys extends keyof ObjectDefinition> = Omit<Definition, Keys> &
    Pick<ObjectDefinition, Keys>;

/** A capability object types opt into. */
export interface Trait<Options> {
    /** The definition key enabling the trait, absent for implied traits. */
    readonly key?: keyof ObjectTraits | "attachments";
    /** The storages whose types take the trait, every storage when absent. */
    readonly storages?: readonly ObjectStorage[];
    /** Read the trait's options from an object definition. */
    options(
        definition: ObjectDefinition<unknown, string, Readonly<Record<string, Method>>>,
    ): Options | undefined;
    /** Add the columns an object with the trait has. */
    columns(options: Options, object: TraitObject): Record<string, ColumnBuilder>;
    /** Constrain the object's derived columns. */
    constraints(
        options: Options,
        table: string,
        columns: Readonly<Record<string, Column>>,
    ): TableConstraint[];
    /** Add the dependents, aggregates and tree the object's table keeps. */
    table?(options: Options, object: TraitObject): TraitTable;
    /** Add relations, permissions and contributions to the object's policy. */
    policy?(options: Options, object: TraitObject, permissions: readonly string[]): TraitPolicy;
    /** Locate the relations the trait adds in the object's table. */
    mapping?(options: Options, object: TraitObject): TraitMapping;
    /** Derive the methods the trait gives the object. */
    methods(options: Options, declared: Readonly<Record<string, Method>>): Record<string, Method>;
    /** Refuse an assembled object type the trait cannot serve. */
    validate?(options: Options, object: ObjectType, definition: ObjectDefinition): void;
}

/** Derive the undeclared permissions a trait's methods require, granted only by roles. */
export function traitPermissions(
    declared: readonly string[],
    derived: readonly string[],
): TraitPolicy {
    const undeclared = derived.filter((name) => !declared.includes(name));

    return { permissions: Object.fromEntries(undeclared.map((name) => [name, none()])) };
}
