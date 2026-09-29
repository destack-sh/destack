import type { Identifier } from "@destack/schema";
import { Scope } from "@destack/sync";
import {
    type Column,
    type ColumnBuilder,
    defineTable,
    identifier,
    text,
    type Table,
    type TableColumnMap,
} from "@destack/db";
import type { ModuleMetadata, PackageId } from "@destack/package";
import type { Field, TextField } from "../field/field.ts";
import type { ClientBuilderMap, RecordBuilderMap } from "../trait/record.ts";
import { type managedColumns } from "../trait/declarable.ts";
import type { DeletionBuilderMap, deletionColumns } from "../trait/recoverable.ts";
import type { ParentBuilderMap } from "../trait/nested.ts";
import type { AddressedBuilderMap } from "../trait/addressed.ts";
import type { VersionBuilderMap } from "../trait/versioned.ts";
import type { controlledColumns } from "../trait/controlled.ts";
import { snakeCase, type SnakeCase } from "./name.ts";
import type { AppliedTrait, ObjectAggregate, ObjectDefinition, ObjectType } from "./object.ts";
import type { ScopeIdentity } from "../method/procedure.ts";

/** The column builders fields create, by property name. */
type FieldBuilderMap<Fields extends Readonly<Record<string, Field>>> = {
    [
        Property in keyof Fields as Fields[Property] extends TextField ? never : Property
    ]: Fields[Property] extends Field<infer Value, infer Required, infer Default>
        ? ColumnBuilder<Value, Required, Default>
        : never;
};

/** The column holding each object's scope. */
type ScopeBuilderMap<Scope> = Scope extends "universe"
    ? { scope: ColumnBuilder<string, true, true> }
    : Scope extends ObjectType
      ? { scope: ColumnBuilder<Identifier<ScopeIdentity<Scope>>, true, false> }
      : { scope: ColumnBuilder<string, true, false> };

/** The columns naming the declaration that manages a record. */
type ManagedBuilderMap<Declarable> = Declarable extends undefined
    ? {}
    : ReturnType<typeof managedColumns>;

/** The columns of a controlled object. */
type ReconciledBuilderMap<Reconciled> = Reconciled extends true
    ? ReturnType<typeof controlledColumns> & ReturnType<typeof deletionColumns>
    : {};

/** Every column a trait may add. */
type TraitBuilderMap = {
    parentPackageId: ColumnBuilder<PackageId, true, false>;
    parentType: ColumnBuilder<string, true, false>;
    parentId: ColumnBuilder<string, true, false>;
    deletionRequestedAt: ColumnBuilder<number, false, false>;
    purgedAt: ColumnBuilder<number, false, false>;
    number: ColumnBuilder<number, true, false>;
} & ReturnType<typeof controlledColumns> &
    ReturnType<typeof managedColumns>;

/** The columns a definition's constraints name. */
export type ConstraintColumns<
    Name extends string,
    Scope,
    Fields extends Readonly<Record<string, Field>>,
    Identity extends string = Name,
    Storage = "durable",
> = TableColumnMap<
    RecordBuilderMap<Identity> &
        ClientBuilderMap<Storage> &
        ScopeBuilderMap<Scope> &
        TraitBuilderMap &
        FieldBuilderMap<Fields>,
    SnakeCase<Name>
>;

/** The option a definition gives a trait, undefined without the trait. */
export type TraitOf<Traits, Name extends string> = Traits extends {
    readonly [Key in Name]: infer Option;
}
    ? Option
    : undefined;

/** Every column of an object's derived table. */
export type ObjectBuilderMap<
    Name extends string,
    Scope,
    Fields extends Readonly<Record<string, Field>>,
    Traits = {},
    Identity extends string = Name,
    Storage = "durable",
> = RecordBuilderMap<Identity> &
    ClientBuilderMap<Storage> &
    ScopeBuilderMap<Scope> &
    ParentBuilderMap<Identity, TraitOf<Traits, "nested">> &
    DeletionBuilderMap<TraitOf<Traits, "recoverable">> &
    AddressedBuilderMap<TraitOf<Traits, "addressed">> &
    VersionBuilderMap<TraitOf<Traits, "versioned">> &
    ReconciledBuilderMap<TraitOf<Traits, "controlled">> &
    ManagedBuilderMap<TraitOf<Traits, "declarable">> &
    FieldBuilderMap<Fields>;

/** The table derived from an object's name, scope, fields and traits. */
export type ObjectTable<
    Name extends string,
    Scope,
    Fields extends Readonly<Record<string, Field>>,
    Traits = {},
    Identity extends string = Name,
    Storage = "durable",
> = Table<
    SnakeCase<Name>,
    TableColumnMap<
        ObjectBuilderMap<Name, Scope, Fields, Traits, Identity, Storage>,
        SnakeCase<Name>
    >
> &
    TableColumnMap<
        ObjectBuilderMap<Name, Scope, Fields, Traits, Identity, Storage>,
        SnakeCase<Name>
    >;

/** Derive an object's table from its traits, scope and fields. */
export function deriveTable(
    definition: ObjectDefinition,
    applied: readonly AppliedTrait[],
    packageId: PackageId,
    module: ModuleMetadata | undefined,
): Table {
    // hold each object's scope
    const name = definition.name;
    const scoped = {
        scope:
            definition.scope === Scope.universe.id
                ? text("scope").notNull().default(Scope.universe.id)
                : Array.isArray(definition.scope)
                  ? text("scope").notNull()
                  : identifier("scope", (definition.scope as ObjectType).identity).notNull(),
    };

    // build the field columns
    let derived: Table | undefined;
    const identity = definition.identity ?? name;
    const object = {
        name,
        identity,
        packageId,
        table: () => derived!,
        storage: definition.storage ?? "durable",
    };
    const [record, ...others] = applied;
    const fields = Object.fromEntries(
        Object.entries(definition.fields ?? {})
            .filter(([, declared]) => declared.type !== "text")
            .map(([property, declared]) => [
                property,
                declared.column(snakeCase(property), identity, object.storage),
            ]),
    );

    // gather the dependents, aggregates and tree the traits keep
    const isAttachment = definition.nested?.in === "any";
    const kept = applied.map(({ trait, options }) => trait.table?.(options, object) ?? {});
    const tree = kept.find((table) => table.tree !== undefined)?.tree;

    // define the table
    const tableName = snakeCase(name);
    derived = defineTable(
        tableName,
        {
            ...record!.trait.columns(record!.options, object),
            ...scoped,
            ...Object.assign(
                {},
                ...others.map(({ trait, options }) => trait.columns(options, object)),
            ),
            ...fields,
        } as never,
        {
            constraints: ((built: Record<string, Column>) => [
                ...applied.flatMap(({ trait, options }) =>
                    trait.constraints(options, tableName, built),
                ),
                ...(definition.constraints?.(built as never) ?? []),
            ]) as never,
            // keep the aggregates of these objects and of their attachments
            aggregates: [
                ...Object.entries(definition.aggregates ?? {})
                    .filter(([, aggregate]) => !(aggregate.via === undefined && isAttachment))
                    .map(([name, aggregate]) => ({
                        into: () => holderOf(definition, aggregate, derived!),
                        column: name,
                        key: aggregate.via ?? "parentId",
                        function: aggregate.function,
                        ...(aggregate.value === undefined ? {} : { value: aggregate.value }),
                        ...(aggregate.where === undefined ? {} : { where: aggregate.where }),
                    })),
                ...kept.flatMap((table) => table.aggregates ?? []),
            ],
            // keep the traits' dependents
            dependents: kept.flatMap((table) => table.dependents ?? []),
            // place the objects in their tier
            ...(definition.tier === undefined ? {} : { tier: definition.tier }),
            // keep history for tracked objects, and index a tree
            log: (definition.tracked === undefined ? {} : { retention: "history" }) as never,
            ...(tree === undefined ? {} : { tree }),
        },
        module,
    ) as Table;

    return derived;
}

/** Find the table holding an aggregate. */
function holderOf(definition: ObjectDefinition, aggregate: ObjectAggregate, own: Table): Table {
    // follow the reference field or the parent
    const field = aggregate.via === undefined ? undefined : definition.fields?.[aggregate.via];
    const parent = definition.nested?.in;
    const holder = field?.target?.() ?? (typeof parent === "string" ? undefined : parent);
    if (aggregate.via !== undefined && field?.target === undefined) {
        throw new TypeError(
            `aggregate of ${definition.name} follows no reference ${aggregate.via}`,
        );
    }
    if (aggregate.via === undefined && definition.nested === undefined) {
        throw new TypeError(`aggregate of ${definition.name} needs a parent or a reference`);
    }

    return (holder?.table as Table | undefined) ?? own;
}
