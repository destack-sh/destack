import { present, type Identifier } from "@destack/schema";
import { Scope } from "@destack/sync";
import {
    type ColumnBuilder,
    type ColumnBuilderMap,
    defineTable,
    identifier,
    text,
    type Table,
    type TableColumnMap,
} from "@destack/db";
import type { ModuleMetadata, PackageId } from "@destack/package";
import type { Field, FieldColumn, TextField } from "../field/field.ts";
import type { ClientBuilderMap, RecordBuilderMap } from "../trait/record.ts";
import { type managedColumns } from "../trait/declarable.ts";
import type { DeletionBuilderMap, deletionColumns } from "../trait/recoverable.ts";
import type { ParentBuilderMap } from "../trait/nested.ts";
import type { TraitObject } from "../trait/trait.ts";
import type { VersionBuilderMap } from "../trait/versioned.ts";
import type { approvalColumns, controlledColumns } from "../trait/controlled.ts";
import { snakeCase, type SnakeCase } from "./name.ts";
import {
    ObjectScope,
    type TraitInstance,
    type ObjectAggregate,
    type ObjectDefinition,
} from "./object.ts";

/** The column builders fields create, by property name. */
type FieldBuilderMap<Fields extends Readonly<Record<string, Field>>> = {
    [
        Property in keyof Fields as Fields[Property] extends TextField ? never : Property
    ]: Fields[Property] extends Field<infer Configuration>
        ? ColumnBuilder<
              FieldColumn<
                  Configuration["value"],
                  [Configuration["guarded"]] extends [true] ? false : Configuration["required"],
                  Configuration["default"],
                  Configuration["sensitive"]
              >
          >
        : never;
};

/** The column with each object's scope. */
type ScopeBuilderMap<Scope> = Scope extends "universe"
    ? { scope: ColumnBuilder<FieldColumn<string, true, true>> }
    : string extends Scope
      ? { scope: ColumnBuilder<FieldColumn<string, true, false>> }
      : Scope extends string
        ? { scope: ColumnBuilder<FieldColumn<Identifier<Scope>, true, false>> }
        : { scope: ColumnBuilder<FieldColumn<string, true, false>> };

/** The columns naming the declaration that manages a record. */
type ManagedBuilderMap<Declarable> = Declarable extends undefined
    ? {}
    : ReturnType<typeof managedColumns>;

/** The columns of a controlled object. */
type ControlledBuilderMap<Reconciled> = Reconciled extends { readonly approval: true }
    ? ReturnType<typeof controlledColumns> &
          ReturnType<typeof deletionColumns> &
          ReturnType<typeof approvalColumns>
    : Reconciled extends true
      ? ReturnType<typeof controlledColumns> & ReturnType<typeof deletionColumns>
      : {};

/** Every column a trait may add. */
type TraitBuilderMap = {
    parentPackageId: ColumnBuilder<FieldColumn<PackageId, true, false>>;
    parentType: ColumnBuilder<FieldColumn<string, true, false>>;
    parentId: ColumnBuilder<FieldColumn<string, true, false>>;
    deletionRequestedAt: ColumnBuilder<FieldColumn<number, false, false>>;
    deletedBy: ColumnBuilder<FieldColumn<string, false, false>>;
    purgedAt: ColumnBuilder<FieldColumn<number, false, false>>;
    number: ColumnBuilder<FieldColumn<number, true, false>>;
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
> = RecordBuilderMap<Identity, TraitOf<Traits, "key">> &
    ClientBuilderMap<Storage> &
    ScopeBuilderMap<Scope> &
    ParentBuilderMap<Identity, TraitOf<Traits, "nested">> &
    DeletionBuilderMap<TraitOf<Traits, "recoverable">> &
    VersionBuilderMap<TraitOf<Traits, "versioned">> &
    ControlledBuilderMap<TraitOf<Traits, "controlled">> &
    ManagedBuilderMap<TraitOf<Traits, "declarable">> &
    FieldBuilderMap<Fields>;

/** The table derived from an object's name, scope, fields and traits, any object's table without arguments. */
export type ObjectTable<
    Name extends string = string,
    Scope = unknown,
    Fields extends Readonly<Record<string, Field>> = {},
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
    applied: readonly TraitInstance[],
    packageId: PackageId,
    module: ModuleMetadata | undefined,
): Table {
    // describe the object to its traits, whose table exists once derived
    const name = definition.name;
    let derived: Table | undefined;
    const table = () => present(derived, `the table of ${name}`);
    const object: TraitObject = {
        name,
        identity: definition.identity ?? name,
        packageId,
        table,
        storage: definition.storage ?? "durable",
        key: definition.key,
    };

    // build the columns and gather the dependents, aggregates and tree the traits keep
    const builders = objectColumns(definition, applied, object);
    const kept = applied.map(({ trait, options }) => trait.table?.(options, object) ?? {});
    const tree = kept.find((traitTable) => traitTable.tree !== undefined)?.tree;

    // define the table
    const tableName = snakeCase(name);
    const defined = defineTable(
        tableName,
        builders,
        {
            constraints: (built) => [
                ...applied.flatMap(({ trait, options }) =>
                    trait.constraints(options, tableName, built),
                ),
                ...(definition.constraints?.(built) ?? []),
            ],
            // keep the aggregates of these objects and of their attachments
            aggregates: [
                ...declaredAggregates(definition, table),
                ...kept.flatMap((traitTable) => traitTable.aggregates ?? []),
            ],
            // keep the traits' dependents
            dependents: kept.flatMap((traitTable) => traitTable.dependents ?? []),
            // rename moved fields' columns in place and convert stored rows
            moved: { columns: movedColumns(definition) },
            ...(definition.convert === undefined ? {} : { convert: definition.convert }),
            // keep history for tracked objects and index a tree
            log: definition.tracked === undefined ? {} : { retention: "history" },
            ...(tree === undefined ? {} : { tree }),
        },
        module,
    );
    derived = defined;

    return defined;
}

/** Build an object's columns: the record trait's, the scope, the other traits' and the fields'. */
function objectColumns(
    definition: ObjectDefinition,
    applied: readonly TraitInstance[],
    object: TraitObject,
): ColumnBuilderMap {
    // require the record trait first
    const [record, ...others] = applied;
    if (record === undefined) {
        throw new TypeError(`object ${definition.name} applies no record trait`);
    }

    // build the field columns, keeping text in chunks
    const fields = Object.fromEntries(
        Object.entries(definition.fields ?? {})
            .filter(([, field]) => field.type !== "text")
            .map(([property, field]) => [
                property,
                field.column(snakeCase(property), object.identity, object.storage),
            ]),
    );

    return {
        ...record.trait.columns(record.options, object),
        scope: scopeColumn(definition.scope),
        ...Object.fromEntries(
            others.flatMap(({ trait, options }) => Object.entries(trait.columns(options, object))),
        ),
        ...fields,
    };
}

/** List the aggregates a definition declares, skipping an attachment's parent aggregates. */
function declaredAggregates(definition: ObjectDefinition, table: () => Table) {
    const isAttachment = definition.nested?.in === "any";

    return Object.entries(definition.aggregates ?? {})
        .filter(([, aggregate]) => !(aggregate.via === undefined && isAttachment))
        .map(([column, aggregate]) => ({
            into: () => holderOf(definition, aggregate, table()),
            column,
            key: aggregate.via ?? "parentId",
            ...(aggregate.function === "count"
                ? { function: aggregate.function }
                : { function: aggregate.function, value: aggregate.value }),
            ...(aggregate.where === undefined ? {} : { where: aggregate.where }),
        }));
}

/** Map each moved field to the column of its previous name. */
function movedColumns(definition: ObjectDefinition): Record<string, string> {
    return Object.fromEntries(
        Object.entries(definition.moved?.fields ?? {}).map(([field, previous]) => [
            field,
            snakeCase(previous),
        ]),
    );
}

/** Find the table with an aggregate. */
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

    return holder?.table ?? own;
}

/** Declare the scope column of objects: any scope for several scope types, the universe, or one scope type's identifiers. */
function scopeColumn(declared: ObjectScope | readonly ObjectScope[]) {
    // take any scope of several types
    if (ObjectScope.isList(declared)) {
        return text("scope").notNull();
    }
    // default to the universe
    else if (declared === Scope.universe.id) {
        return text("scope").notNull().default(Scope.universe.id);
    }
    // take one scope type's identifiers
    else {
        return identifier("scope", declared.identity).notNull();
    }
}
