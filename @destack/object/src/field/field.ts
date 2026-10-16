import { PackageId } from "@destack/package";
import { DeclarationReference } from "@destack/package/declare";
import {
    bigint,
    binary,
    boolean,
    ColumnBuilder,
    TABLE,
    integer,
    json,
    real,
    text,
    type Column,
    type ColumnDefinition,
    type Condition,
    type ColumnValue,
    type SQL,
    type Sealing,
    type Select,
    type Table,
    textEnum,
} from "@destack/db";
import { Message } from "@destack/locale";
import { canonicalize, schema, type Identifier, type JsonValue } from "@destack/schema";
import type { BuilderCall } from "../object/description.ts";
import type { ObjectConstraint } from "../object/constraint.ts";
import type { IdentityOf, ObjectStorage, ObjectType } from "../object/object.ts";
import { Policy } from "@destack/access";
import { FractionalIndex } from "./fractional-index.ts";
import { type FileBucket, ObjectFile } from "./file.ts";

/** The meanings of fields beyond their stored values. */
export const FIELD_TYPES = [
    "string",
    "integer",
    "bigint",
    "number",
    "boolean",
    "binary",
    "time",
    "enum",
    "json",
    "message",
    "reference",
    "subject",
    "object",
    "fractionalIndex",
    "file",
    "state",
    "text",
] as const;

/** The meaning of a field beyond its stored value. */
export type FieldType = (typeof FIELD_TYPES)[number];

/** The field type of each builder: an aggregate's value type, else the builder's own. */
const BUILDER_TYPES: Readonly<Record<FieldSource["builder"], FieldType>> = {
    string: "string",
    integer: "integer",
    bigint: "bigint",
    number: "number",
    binary: "binary",
    boolean: "boolean",
    time: "time",
    fractionalIndex: "fractionalIndex",
    text: "text",
    count: "integer",
    sum: "number",
    min: "number",
    max: "number",
    enum: "enum",
    json: "json",
    message: "message",
    file: "file",
    subject: "subject",
    object: "object",
    state: "state",
    reference: "reference",
};

/** The extra permissions reading or writing one field requires. */
export interface FieldAccess {
    /** The permission a caller needs to read the value. */
    readonly read?: string;
    /** The permission a caller needs to write the value. */
    readonly write?: string;
}

/** A state machine a field follows. */
export interface StateMachine {
    /** The state new objects start in. */
    readonly initial: string;
    /** Transitions by method name. */
    readonly transitions: Readonly<
        Record<
            string,
            {
                /** The states the transition leaves. */
                readonly from: readonly string[];
                /** The state the transition enters. */
                readonly to: string;
                /** The object permission the caller needs. */
                readonly permission: string;
                /** The fields the transition writes beside the state, from its call's input. */
                readonly fields?: readonly string[];
                /** Whether the transition stays off public routes, run only by the store's code. */
                readonly isInternal?: boolean;
            }
        >
    >;
}

/** The builder call of a field: the builder's name and its arguments as written. */
export type FieldSource =
    | { readonly builder: "string"; readonly arguments: readonly [schema.Schema<string>] }
    | {
          readonly builder:
              | "integer"
              | "bigint"
              | "number"
              | "binary"
              | "boolean"
              | "time"
              | "fractionalIndex"
              | "text"
              | "message"
              | "count"
              | "sum"
              | "min"
              | "max";
          readonly arguments: readonly [];
      }
    | { readonly builder: "enum"; readonly arguments: readonly [readonly [string, ...string[]]] }
    | { readonly builder: "json"; readonly arguments: readonly [schema.Schema] }
    | { readonly builder: "file"; readonly arguments: readonly [FileBucket] }
    | { readonly builder: "subject"; readonly arguments: readonly (Policy | ObjectType)[] }
    | { readonly builder: "object"; readonly arguments: readonly (ObjectType | Policy)[] }
    | { readonly builder: "state"; readonly arguments: readonly [StateMachine] }
    | {
          readonly builder: "reference";
          readonly arguments: readonly [
              target: ObjectType | Policy | (() => ObjectType | Policy),
              options?: ReferenceOptions,
          ];
      };

/** The modifiers a field's description records after its builder call, in decoding order. */
export const FIELD_MODIFIERS = [
    "optional",
    "default",
    "sensitive",
    "personal",
    "caller",
    "guard",
    "guardWrite",
    "relate",
] as const;

/** Every state a state machine lists. */
export type StateOf<Machine extends StateMachine> =
    | Machine["initial"]
    | Machine["transitions"][keyof Machine["transitions"]]["from"][number]
    | Machine["transitions"][keyof Machine["transitions"]]["to"];

/** The aggregate function maintaining a field. */
export type AggregateFunction = "count" | "sum" | "min" | "max";

/** A field's column: not null if required, with any default, sensitive if classified so. */
export type FieldColumn<
    Value extends ColumnValue,
    Required extends boolean,
    Default extends boolean,
    Sensitive extends boolean = false,
> = ColumnDefinition<Value> &
    (Required extends true ? { readonly nullable: false } : {}) &
    (Default extends true ? { readonly default: Value | SQL } : {}) &
    (Sensitive extends true ? { readonly classification: "sensitive" } : {});

/** A field's signature: its value and its flags. */
export interface FieldConfiguration {
    /** The value the field stores. */
    readonly value: ColumnValue;
    /** Whether every object must have a value. */
    readonly required: boolean;
    /** Whether creation supplies a value the caller omits. */
    readonly default: boolean;
    /** Whether reading the value requires its own permission. */
    readonly guarded: boolean;
    /** Whether callers write the value: always, only at creation, or never. */
    readonly written: boolean | "create";
    /** Whether the value is sensitive. */
    readonly sensitive: boolean;
    /** Whether creation fills the value with the calling principal, outside internal creations. */
    readonly caller: boolean;
    /** How the value refers to another object: not, by identifier, or by object reference. */
    readonly reference: ReferenceKind;
    /** The relation a reference declares, absent for other fields. */
    readonly relation: string;
}

/** A field's flags as the values it stores, each typed by its signature's flag. */
interface FieldFlags<Configuration extends FieldConfiguration = FieldConfiguration> {
    /** Whether every object must have a value. */
    readonly required: Configuration["required"];
    /** Whether creation supplies a value the caller omits. */
    readonly isDefault: Configuration["default"];
    /** Whether reading the value requires its own permission. */
    readonly guarded: Configuration["guarded"];
    /** Whether callers write the value. */
    readonly written: Configuration["written"];
    /** Whether the value is sensitive. */
    readonly isSensitive: Configuration["sensitive"];
    /** Whether creation fills the value with the calling principal. */
    readonly isCallerFilled: Configuration["caller"];
    /** How the value refers to another object: not, by identifier, or by object reference. */
    readonly reference: Configuration["reference"];
}

/** The flags of a required, written, unguarded field. */
const WRITTEN = {
    required: true,
    isDefault: false,
    guarded: false,
    written: true,
    isSensitive: false,
    isCallerFilled: false,
    reference: "none",
} as const;

/** The signature of a required, written, unguarded field, each absent flag at its default. */
type FieldConfigurationOf<
    Declared extends Partial<FieldConfiguration> & { readonly value: ColumnValue },
> = {
    readonly value: Declared["value"];
    readonly required: Declared extends { readonly required: infer Flag } ? Flag : true;
    readonly default: Declared extends { readonly default: infer Flag } ? Flag : false;
    readonly guarded: Declared extends { readonly guarded: infer Flag } ? Flag : false;
    readonly written: Declared extends { readonly written: infer Flag } ? Flag : true;
    readonly sensitive: Declared extends { readonly sensitive: infer Flag } ? Flag : false;
    readonly caller: Declared extends { readonly caller: infer Flag } ? Flag : false;
    readonly reference: Declared extends { readonly reference: infer Kind } ? Kind : "none";
    readonly relation: never;
};

/** A field of a declared signature, each absent flag at its default. */
export type FieldOf<
    Declared extends Partial<FieldConfiguration> & { readonly value: ColumnValue },
> = Field<FieldConfigurationOf<Declared>>;

/** A field's signature with some properties changed. */
type Override<
    Configuration extends FieldConfiguration,
    Changes extends Partial<FieldConfiguration>,
> = {
    readonly [Key in keyof FieldConfiguration]: Key extends keyof Changes
        ? Changes[Key]
        : Configuration[Key];
};

/** A field's signature with its value and relation, its flags the types of the stored values. */
type FlagConfiguration<Configuration extends FieldConfiguration, Flags extends FieldFlags> = {
    readonly value: Configuration["value"];
    readonly relation: Configuration["relation"];
    readonly required: Flags["required"];
    readonly default: Flags["isDefault"];
    readonly guarded: Flags["guarded"];
    readonly written: Flags["written"];
    readonly sensitive: Flags["isSensitive"];
    readonly caller: Flags["isCallerFilled"];
    readonly reference: Flags["reference"];
};

/** How a field refers to another object: not, by identifier, or by object reference. */
export type ReferenceKind = "none" | "identifier" | "object";

/** The column suffixes of an object reference beside its identifier. */
const REFERENCE_PARTS = ["PackageId", "Type", "Scope"] as const;

/** The fields keyed by column, each reference expanded into its columns. */
export type FieldsByColumn<Fields> = {
    readonly [
        Key in keyof Fields & string as Fields[Key] extends Field<infer Configuration>
            ? Configuration["reference"] extends "identifier" | "object"
                ? `${Key}Id`
                : Key
            : Key
    ]: Fields[Key] extends Field<infer Configuration>
        ? Configuration["reference"] extends "identifier" | "object"
            ? Field<Override<Configuration, { readonly relation: Key }>>
            : Fields[Key]
        : Fields[Key];
} & {
    readonly [Part in ReferencePartOf<Fields> as `${Part["key"]}${Part["part"]}`]: Field<
        Override<
            Part["configuration"],
            {
                readonly value: Part["part"] extends "PackageId" ? PackageId : string;
                readonly reference: "none";
            }
        >
    >;
};

/** Each object reference field's columns beside its identifier, as key, part and signature. */
type ReferencePartOf<Fields> = {
    [Key in keyof Fields & string]: Fields[Key] extends Field<infer Configuration>
        ? Configuration["reference"] extends "object"
            ? {
                  readonly key: Key;
                  readonly part: (typeof REFERENCE_PARTS)[number];
                  readonly configuration: Configuration;
              }
            : never
        : never;
}[keyof Fields & string];

/** Build the column of a field of a value. */
type BuildColumn<Value extends ColumnValue> = (
    name: string,
    owner: string,
    storage: ObjectStorage,
) => ColumnBuilder<ColumnDefinition<Value>>;

/** What a field has beside its flags. */
interface FieldOptions<Configuration extends FieldConfiguration> {
    /** The builder call the field came from. */
    readonly source: FieldSource;
    /** Create the nullable column storing the field. */
    readonly build: (name: string, owner: string, storage: ObjectStorage) => ColumnBuilder;
    /** The relation a reference declares. */
    readonly relation?: Configuration["relation"];
    /** The value creation supplies when the caller omits the field. */
    readonly initial?: { readonly value: Configuration["value"] };
    /** Whether the value is personal data. */
    readonly isPersonal?: boolean;
    /** How a sensitive value is sealed at rest, randomly unless a lookup finds it by value. */
    readonly sealing?: Sealing;
    /** The permissions reading or writing the value requires. */
    readonly access?: FieldAccess;
}

/** A field of an object. */
export class Field<Configuration extends FieldConfiguration = FieldConfiguration> {
    /** The builder call the field came from, which sets its meaning, target and storage. */
    readonly source: FieldSource;
    /** The relation a reference declares, absent until its object keys it. */
    readonly relation: Configuration["relation"] | undefined;
    /** Whether every object must have a value. */
    readonly required: Configuration["required"];
    /** Whether creation supplies a value the caller omits. */
    readonly isDefault: Configuration["default"];
    /** Whether reading the value requires its own permission. */
    readonly guarded: Configuration["guarded"];
    /** Whether callers write the value. */
    readonly written: Configuration["written"];
    /** Whether the value is sensitive. */
    readonly isSensitive: Configuration["sensitive"];
    /** Whether creation fills the value with the calling principal. */
    readonly isCallerFilled: Configuration["caller"];
    /** How the value refers to another object: not, by identifier, or by object reference. */
    readonly reference: Configuration["reference"];
    /** The value creation supplies when the caller omits the field. */
    readonly initial: { readonly value: Configuration["value"] } | undefined;
    /** Whether the value is personal data, exported and erased with its subject. */
    readonly isPersonal: boolean;
    /** How a sensitive value is sealed at rest, randomly unless a lookup finds it by value. */
    readonly sealing: Sealing;
    /** The permissions reading or writing the value requires. */
    readonly access: FieldAccess | undefined;
    /** Create the nullable column storing the field. */
    readonly #build: (name: string, owner: string, storage: ObjectStorage) => ColumnBuilder;
    /** What a reference refers to, absent for other fields. */
    readonly #reference: ReferenceTarget | undefined;
    /** The referenced object type, resolved on each call, absent for other fields. */
    readonly #target: (() => ObjectType) | undefined;

    /** Retain the field's meaning, storage and flags. */
    constructor(definition: FieldOptions<Configuration> & FieldFlags<Configuration>) {
        // retain the source, the modifiers and the column builder
        this.source = definition.source;
        this.relation = definition.relation;
        this.initial = definition.initial;
        this.isPersonal = definition.isPersonal ?? false;
        this.sealing = definition.sealing ?? "random";
        this.access = definition.access;
        this.#build = definition.build;

        // read what a reference refers to once
        this.#reference = ReferenceTarget.of(definition.source);
        this.#target = this.#reference?.kind === "object" ? this.#reference.target : undefined;

        // retain the flags
        this.required = definition.required;
        this.isDefault = definition.isDefault;
        this.guarded = definition.guarded;
        this.written = definition.written;
        this.isSensitive = definition.isSensitive;
        this.isCallerFilled = definition.isCallerFilled;
        this.reference = definition.reference;
    }

    /** The field's meaning, as its builder declares it. */
    get type(): FieldType {
        return BUILDER_TYPES[this.source.builder];
    }

    /** The referenced object type, for references to objects. */
    get target(): (() => ObjectType) | undefined {
        return this.#target;
    }

    /** The principal kinds a principal reference or subject field accepts. */
    get principals(): readonly Policy[] | undefined {
        // accept a subject field's kinds, or a principal reference's kind
        const { source } = this;
        if (source.builder === "subject") {
            return source.arguments.length === 0
                ? undefined
                : source.arguments.map((kind) => Field.principal(kind));
        }

        return this.#reference?.kind === "principal" ? [this.#reference.target] : undefined;
    }

    /** The aggregate function maintaining the value. */
    get aggregate(): AggregateFunction | undefined {
        const { builder } = this.source;

        return builder === "count" || builder === "sum" || builder === "min" || builder === "max"
            ? builder
            : undefined;
    }

    /** The state machine the value follows. */
    get machine(): StateMachine | undefined {
        return this.source.builder === "state" ? this.source.arguments[0] : undefined;
    }

    /** The bucket a file field stores its files in, by its declaring package and name. */
    get bucket(): DeclarationReference | undefined {
        return this.source.builder === "file"
            ? DeclarationReference.of(this.source.arguments[0])
            : undefined;
    }

    /** Create the column storing the field, its definition following the field's flags. */
    column(name: string, owner: string, storage: ObjectStorage): ColumnBuilder {
        // build the nullable column
        let column: ColumnBuilder = this.#build(name, owner, storage);

        // mark sensitive and personal values
        if (this.isSensitive) {
            column = column.sensitive(this.sealing);
        } else if (this.isPersonal) {
            column = column.personal();
        }

        // supply the default
        if (this.initial) {
            column = column.default(this.initial.value);
        }

        // require a value, except where replicas hold a guarded value concealed
        if (this.required && !this.guarded) {
            column = column.notNull();
        }

        return column;
    }

    /** Allow the field to be absent. */
    optional(): Field<Override<Configuration, { readonly required: false }>> {
        return this.#with({ ...this.#flags, required: false });
    }

    /** Supply a value when creation omits the field. */
    default(
        value: Configuration["value"],
    ): Field<Override<Configuration, { readonly default: true }>> {
        return this.#with({ ...this.#flags, isDefault: true }, { initial: { value } });
    }

    /** Seal the value at rest, out of logs, audit details, sync and fingerprints. */
    sensitive(
        sealing: Sealing = "random",
    ): Field<Override<Configuration, { readonly sensitive: true }>> {
        return this.#with({ ...this.#flags, isSensitive: true }, { sealing });
    }

    /** Mark the value as personal data, exported and erased with its subject. */
    personal(): Field<Configuration> {
        return new Field<Configuration>({ ...this.#options, ...this.#flags, isPersonal: true });
    }

    /** Write the value only at creation, leaving it out of updates. */
    immutable(): Field<Override<Configuration, { readonly written: "create" }>> {
        return this.#with({ ...this.#flags, written: "create" });
    }

    /** Fill the field with the principal creating the object. */
    caller(): Field<
        Override<
            Configuration,
            { readonly default: true; readonly written: false; readonly caller: true }
        >
    > {
        if (this.principals === undefined && this.type !== "subject") {
            throw new TypeError(
                "only principal references and subject fields can default to the calling principal",
            );
        }

        return this.#with({
            ...this.#flags,
            isDefault: true,
            written: false,
            isCallerFilled: true,
        });
    }

    /** Require a permission to read the value, and optionally another to write it. */
    guard(access: {
        readonly read: string;
        readonly write?: string;
    }): Field<Override<Configuration, { readonly guarded: true }>> {
        return this.#with(
            { ...this.#flags, guarded: true },
            { access: { ...this.access, ...access } },
        );
    }

    /** Require a permission to write the value, leaving reads to the object's permissions. */
    guardWrite(permission: string): Field<Configuration> {
        return new Field<Configuration>({
            ...this.#options,
            ...this.#flags,
            access: { ...this.access, write: permission },
        });
    }

    /** Build the field a builder call makes on its own, before any modifier. */
    static #replay(source: FieldSource): Field {
        // refuse a builder call making no field
        const built: unknown = Reflect.apply(field[source.builder], field, source.arguments);
        if (!Field.is(built)) {
            throw new TypeError(`field builder ${source.builder} makes no field`);
        }

        return built;
    }

    /** Decide whether a value is a field. */
    static is(value: unknown): value is Field {
        return value instanceof Field;
    }

    /** Describe the field as builder calls: its builder's, then each effective modifier's. */
    describe(): readonly [FieldSource, ...BuilderCall[]] {
        // compare with the field the builder call makes on its own
        const plain = Field.#replay(this.source);
        const { initial, access } = this;
        const isDefaulted =
            initial !== undefined &&
            !this.isCallerFilled &&
            (plain.initial === undefined ||
                canonicalize(initial.value) !== canonicalize(plain.initial.value));
        const modifiers: BuilderCall[] = [
            ...(plain.required && !this.required ? [{ builder: "optional", arguments: [] }] : []),
            ...(isDefaulted ? [{ builder: "default", arguments: [initial.value] }] : []),
            ...(this.isSensitive && !plain.isSensitive
                ? [
                      {
                          builder: "sensitive",
                          arguments: this.sealing === "random" ? [] : [this.sealing],
                      },
                  ]
                : []),
            ...(this.isPersonal && !plain.isPersonal
                ? [{ builder: "personal", arguments: [] }]
                : []),
            ...(this.isCallerFilled && !plain.isCallerFilled
                ? [{ builder: "caller", arguments: [] }]
                : []),
        ];

        // guard reads and writes, and declare the reference's relation
        const guarded =
            access?.read !== undefined
                ? [{ builder: "guard", arguments: [access] }]
                : access?.write !== undefined
                  ? [{ builder: "guardWrite", arguments: [access.write] }]
                  : [];
        const related =
            this.relation === undefined ? [] : [{ builder: "relate", arguments: [this.relation] }];

        return [this.source, ...modifiers, ...guarded, ...related];
    }

    /** Read the relation under a property: a reference's declared one, else the property. */
    relationAt(property: string): string {
        return this.relation ?? property;
    }

    /** Declare the relation a reference has, as its object keys it under `<relation>Id`. */
    relate<const Relation extends string>(
        relation: Relation,
    ): Field<Override<Configuration, { readonly relation: Relation }>> {
        return new Field<Override<Configuration, { readonly relation: Relation }>>({
            ...this.#options,
            ...this.#flags,
            relation,
        });
    }

    /** List the fields an object has for this one under a property. */
    expand(property: string): Readonly<Record<string, Field>> {
        // store an identifier under `<property>Id`
        if (this.reference === "identifier") {
            return { [`${property}Id`]: this.relate(property) };
        }
        // store an object reference as its package, type, scope and identifier
        else if (this.reference === "object") {
            const { packages, types } = this.#referred();
            const columns = Field.columnsOf(property);

            return {
                [columns.packageId]: this.#part(packages),
                [columns.type]: this.#part(types),
                [columns.scope]: this.#part(schema.string().min(1)),
                [columns.id]: this.relate(property),
            };
        }

        return { [property]: this };
    }

    /** List the columns of an object reference under a property. */
    static columnsOf(property: string): {
        readonly packageId: string;
        readonly type: string;
        readonly scope: string;
        readonly id: string;
    } {
        return {
            packageId: `${property}PackageId`,
            type: `${property}Type`,
            scope: `${property}Scope`,
            id: `${property}Id`,
        };
    }

    /** List the constraints of an object reference under a property: its index and its pairs. */
    constraints(property: string): Readonly<Record<string, ObjectConstraint>> {
        // constrain only object references
        if (this.reference !== "object") {
            return {};
        }

        // index the reference's columns
        const { packageId, type, scope, id } = Field.columnsOf(property);
        const index: ObjectConstraint = { kind: "index", on: [packageId, type, scope, id] };

        // allow only the declared types' package and type pairs, or no reference when optional
        const pairs: Condition[] = this.#pairs().map(([pairPackage, pairType]) => ({
            [packageId]: pairPackage,
            [type]: pairType,
        }));
        const absent: Condition = {
            [packageId]: { isNull: true },
            [type]: { isNull: true },
            [scope]: { isNull: true },
            [id]: { isNull: true },
        };
        const pair: ObjectConstraint = {
            kind: "check",
            condition: { OR: this.required ? pairs : [...pairs, absent] },
        };

        return pairs.length > 1
            ? { [property]: index, [`${property}Pair`]: pair }
            : { [property]: index };
    }

    /** List the package and type pairs an object reference allows, none for any type. */
    #pairs(): readonly (readonly [string, string])[] {
        const { source } = this;
        const kinds =
            source.builder === "subject"
                ? source.arguments.map((kind) => Field.principal(kind))
                : source.builder === "object"
                  ? source.arguments.map((kind) => (kind instanceof Policy ? kind : kind.policy))
                  : [];

        return kinds.map((kind) => [kind.definition.packageId, kind.name]);
    }

    /** Read the packages and types an object reference allows, any for none. */
    #referred(): {
        readonly packages: schema.Schema<string>;
        readonly types: schema.Schema<string>;
    } {
        // allow any package and type without declared types
        const pairs = this.#pairs();
        const [first, ...others] = pairs;
        if (first === undefined) {
            return { packages: PackageId, types: schema.string().min(1) };
        }

        // allow the declared packages and types
        const packages = [...new Set([first[0], ...others.map(([pairPackage]) => pairPackage)])];
        const types = [...new Set([first[1], ...others.map(([, pairType]) => pairType)])];

        return { packages: schema.enum(packages), types: schema.enum(types) };
    }

    /** Derive a column of an object reference beside its identifier, with its flags. */
    #part(validator: schema.Schema<string>): Field {
        return new Field({
            source: { builder: "string", arguments: [validator] },
            build: (name) => Field.validated(name, validator),
            isPersonal: this.isPersonal,
            ...(this.access === undefined ? {} : { access: this.access }),
            ...this.#flags,
            reference: "none",
        });
    }

    /** Read what the field has beside its flags. */
    get #options(): FieldOptions<Configuration> {
        return {
            source: this.source,
            build: this.#build,
            ...(this.relation === undefined ? {} : { relation: this.relation }),
            ...(this.initial === undefined ? {} : { initial: this.initial }),
            isPersonal: this.isPersonal,
            sealing: this.sealing,
            ...(this.access === undefined ? {} : { access: this.access }),
        };
    }

    /** Read the field's flags. */
    get #flags(): FieldFlags<Configuration> {
        return {
            required: this.required,
            isDefault: this.isDefault,
            guarded: this.guarded,
            written: this.written,
            isSensitive: this.isSensitive,
            isCallerFilled: this.isCallerFilled,
            reference: this.reference,
        };
    }

    /** Derive a field with the given flags, its signature the types of those flags. */
    #with<const Flags extends FieldFlags>(
        flags: Flags,
        change: Pick<FieldOptions<Configuration>, "initial" | "access" | "sealing"> = {},
    ): Field<FlagConfiguration<Configuration, Flags>> {
        return new Field<FlagConfiguration<Configuration, Flags>>({
            ...this.#options,
            ...change,
            required: flags.required,
            isDefault: flags.isDefault,
            guarded: flags.guarded,
            written: flags.written,
            isSensitive: flags.isSensitive,
            isCallerFilled: flags.isCallerFilled,
            reference: flags.reference,
        });
    }

    /** Build a required, written field of one type from a nullable column. */
    static required<Value extends ColumnValue>(
        source: FieldSource,
        build: BuildColumn<Value>,
    ): FieldOf<{ value: Value }> {
        return new Field<FieldConfigurationOf<{ value: Value }>>({ source, build, ...WRITTEN });
    }

    /** Declare a field an aggregate maintains, starting at a value. */
    static aggregated(
        aggregate: "count" | "sum",
        build: (name: string) => ColumnBuilder<ColumnDefinition<number>>,
        initial: number,
    ): FieldOf<{ value: number; default: true; written: false }> {
        return new Field<FieldConfigurationOf<{ value: number; default: true; written: false }>>({
            source: { builder: aggregate, arguments: [] },
            build,
            initial: { value: initial },
            ...WRITTEN,
            isDefault: true,
            written: false,
        });
    }

    /** Declare a field an extreme maintains, absent until a belonging object has a value. */
    static extreme(
        aggregate: "min" | "max",
        build: (name: string) => ColumnBuilder<ColumnDefinition<number>>,
    ): FieldOf<{ value: number; required: false; written: false }> {
        return new Field<FieldConfigurationOf<{ value: number; required: false; written: false }>>({
            source: { builder: aggregate, arguments: [] },
            build,
            ...WRITTEN,
            required: false,
            written: false,
        });
    }

    /** Key an identifier column to its type's table, unkeyed in memory and in the event log. */
    static keyed<Definition extends ColumnDefinition>(
        column: ColumnBuilder<Definition>,
        target: () => ObjectType,
        storage: ObjectStorage,
        options: ReferenceOptions,
    ): ColumnBuilder<Definition> {
        // leave references in memory and in the event log without a foreign key
        if (storage === "ephemeral" || storage === "events") {
            return column;
        }

        return column.references(() => idColumn(target().table), {
            onDelete:
                options.delete === "cascade"
                    ? "cascade"
                    : options.delete === "null"
                      ? "set null"
                      : "no action",
        });
    }

    /** Read a subject's principal kind: the kind, or the kind an object type represents. */
    static principal(kind: Policy | ObjectType): Policy {
        // take a kind as named
        if (kind instanceof Policy) {
            return kind;
        }
        // refuse an object type representing no principal kind
        else if (kind.represents === undefined) {
            throw new TypeError(`object ${kind.name} lacks a principal kind it represents`);
        }

        return kind.policy;
    }

    /** Define the nullable text column a string schema validates, typed by the schema's output. */
    static validated<Validator extends schema.Schema<string>>(
        name: string,
        validator: Validator,
    ): ColumnBuilder<ColumnDefinition<schema.Output<Validator>, string>> {
        return new ColumnBuilder<ColumnDefinition<schema.Output<Validator>, string>>({
            name,
            kind: "text",
            types: { sqlite: "text", postgresql: "text" },
            schema: validator,
            json: validator,
            nullable: true,
            toJson: (value) => value,
            fromJson: (value) => validator.parse(value),
            encode: (value) => validator.parse(value),
            decode: (value) => validator.parse(value),
        });
    }
}

/** The options of a reference field. */
interface ReferenceOptions {
    /** What deleting the referenced object does to this one. */
    readonly delete?: "restrict" | "cascade" | "null";
}

/** The identifier of an object type's objects. */
export type IdentifierOf<Target extends ObjectType> = Identifier<IdentityOf<Target>>;

/** The value identifying an object of a type: its natural key or generated identifier. */
export type ReferenceOf<Target extends ObjectType> = Table extends Target["table"]
    ? IdentifierOf<Target>
    : Select<Target["table"]> extends { readonly id: infer Id extends string }
      ? Id
      : IdentifierOf<Target>;

/** The value a reference stores: a principal's identifier, else an object's identifier or key. */
type ReferenceValue<Target> =
    Target extends Policy<string, infer Kind>
        ? Identifier<Kind>
        : Target extends ObjectType
          ? ReferenceOf<Target>
          : never;

/** Reference an object in the same space by identifier, or a principal of a global kind. */
function reference<const Target extends ObjectType | Policy>(
    target: Target | (() => Target),
    options?: ReferenceOptions,
): FieldOf<{ value: ReferenceValue<Target>; reference: "identifier" }> {
    // refuse a principal kind living in scopes, which a subject field stores
    if (target instanceof Policy && target.definition.isGlobal !== true) {
        throw new TypeError(
            `principal kind ${target.name} lives in scopes: use field.subject(principal.${target.name})`,
        );
    }

    return new Field<
        FieldConfigurationOf<{ value: ReferenceValue<Target>; reference: "identifier" }>
    >({
        source: {
            builder: "reference",
            arguments: options === undefined ? [target] : [target, options],
        },
        build: (name, _owner, storage) => {
            // store a principal's global identifier, else key the object's identifier to its table
            const given: ObjectType | Policy | (() => ObjectType | Policy) = target;
            if (given instanceof Policy) {
                return text(name).validate(schema.identifier(given.name));
            }
            const resolve = ReferenceTarget.object(given);

            return Field.keyed(
                Field.validated(
                    name,
                    schema.lazy(() => resolve().keyed ?? resolve().idSchema),
                ),
                resolve,
                storage,
                options ?? {},
            );
        },
        ...WRITTEN,
        reference: "identifier",
    });
}

/** A reference target: a global principal kind, an object type now or later, or any scope. */
type ReferenceTarget =
    | { readonly kind: "principal"; readonly target: Policy }
    | { readonly kind: "object"; readonly target: () => ObjectType };

/** Read what a reference's arguments refer to. */
const ReferenceTarget = {
    /** Read what a field's builder call refers to, absent for a field of another builder. */
    of(source: FieldSource): ReferenceTarget | undefined {
        if (source.builder !== "reference") {
            return undefined;
        }
        const [target] = source.arguments;

        return target instanceof Policy
            ? { kind: "principal", target }
            : { kind: "object", target: ReferenceTarget.object(target) };
    },

    /** Resolve an object type now or later, refusing a principal kind resolved later. */
    object(target: ObjectType | (() => ObjectType | Policy)): () => ObjectType {
        // resolve a later target on each call
        if (typeof target !== "function") {
            return () => target;
        }

        return () => {
            const resolved = target();
            if (resolved instanceof Policy) {
                throw new TypeError(
                    `a reference to principal kind ${resolved.name} takes the kind itself`,
                );
            }

            return resolved;
        };
    },
};

/** Read the identifier column of an object type's table, which its references point at. */
export function idColumn(table: Table): Column {
    return table[TABLE].column("id");
}

/** Declare text a string schema validates, typed by the schema's output, any string without one. */
function string<Validator extends schema.Schema<string> = schema.Schema<string>>(
    validator?: Validator,
): FieldOf<{ value: schema.Output<Validator> }> {
    const validated = validator ?? schema.string();

    return new Field<FieldConfigurationOf<{ value: schema.Output<Validator> }>>({
        source: { builder: "string", arguments: [validated] },
        build: (name) => Field.validated(name, validated),
        ...WRITTEN,
    });
}

/** A field following a state machine, with the machine it follows. */
export class StateField<Machine extends StateMachine = StateMachine> extends Field<
    FieldConfigurationOf<{ value: StateOf<Machine>; default: true; written: false }>
> {
    /** The state machine the field follows. */
    readonly #machine: Machine;

    /** Declare the field, listing every state the machine has, the initial one first. */
    constructor(machine: Machine) {
        // list the states, the initial one first
        const others = new Set<string>();
        for (const transition of Object.values(machine.transitions)) {
            transition.from.forEach((name) => others.add(name));
            others.add(transition.to);
        }
        others.delete(machine.initial);
        const names: [string, ...string[]] = [machine.initial, ...others];

        // start in the initial state, which only transitions change
        super({
            source: { builder: "state", arguments: [machine] },
            initial: { value: machine.initial },
            build: (name) => textEnum(name, names),
            ...WRITTEN,
            isDefault: true,
            written: false,
        });
        this.#machine = machine;
    }

    /** The state machine the field follows. */
    override get machine(): Machine {
        return this.#machine;
    }
}

/** A text field, stored in chunks and changed only by edits, empty at first. */
export class TextField extends Field<
    FieldConfigurationOf<{ value: string; default: true; written: false }>
> {
    /** Declare the field, whose characters live in chunks. */
    constructor() {
        super({
            source: { builder: "text", arguments: [] },
            initial: { value: "" },
            build: () => {
                throw new TypeError("a text field stores its characters in chunks, not a column");
            },
            ...WRITTEN,
            isDefault: true,
            written: false,
        });
    }

    /** The field's meaning: a text stored in chunks. */
    override get type(): "text" {
        return "text";
    }
}

/** Declare the fields of objects. */
export const field = {
    /** Text a string schema validates. */
    string,

    /** A whole number within the exactly representable range. */
    integer() {
        return Field.required({ builder: "integer", arguments: [] }, integer);
    },

    /** A whole number of 64 bits, beyond the exact range of a double. */
    bigint() {
        return Field.required({ builder: "bigint", arguments: [] }, bigint);
    },

    /** A double-precision number. */
    number() {
        return Field.required({ builder: "number", arguments: [] }, real);
    },

    /** The count of objects belonging to each object. */
    count(): FieldOf<{ value: number; default: true; written: false }> {
        return Field.aggregated("count", integer, 0);
    },

    /** The sum of a field over each object's belonging objects. */
    sum(): FieldOf<{ value: number; default: true; written: false }> {
        return Field.aggregated("sum", real, 0);
    },

    /** The smallest value of a field over each object's belonging objects. */
    min(): FieldOf<{ value: number; required: false; written: false }> {
        return Field.extreme("min", real);
    },

    /** The largest value of a field over each object's belonging objects. */
    max(): FieldOf<{ value: number; required: false; written: false }> {
        return Field.extreme("max", real);
    },

    /** Bytes. */
    binary() {
        return Field.required({ builder: "binary", arguments: [] }, binary);
    },

    /** True or false. */
    boolean() {
        return Field.required({ builder: "boolean", arguments: [] }, boolean);
    },

    /** An instant in UTC epoch milliseconds. */
    time() {
        return Field.required({ builder: "time", arguments: [] }, integer);
    },

    /** One of a fixed set of strings. */
    enum<const Values extends readonly [string, ...string[]]>(values: Values) {
        return Field.required({ builder: "enum", arguments: [values] }, (name) =>
            textEnum(name, values),
        );
    },

    /** A state machine changed by its transition methods. */
    state<const Machine extends StateMachine>(machine: Machine): StateField<Machine> {
        return new StateField(machine);
    },

    /** A structured value validated by a schema. */
    json<Value extends JsonValue>(validator: schema.Schema<Value>) {
        return Field.required({ builder: "json", arguments: [validator] }, (name) =>
            json(name, validator),
        );
    },

    /** A message the declaring package's catalogs translate, with its identifier and source. */
    message() {
        return Field.required({ builder: "message", arguments: [] }, (name) => json(name, Message));
    },

    /** A file stored in a bucket the package declares, by its key. */
    file(bucket: FileBucket): FieldOf<{ value: ObjectFile }> {
        return new Field<FieldConfigurationOf<{ value: ObjectFile }>>({
            source: { builder: "file", arguments: [bucket] },
            build: (name) => json(name, ObjectFile),
            ...WRITTEN,
        });
    },

    /** A principal of given kinds, or kinds object types represent, stored as its reference. */
    subject(
        ...kinds: readonly (Policy | ObjectType)[]
    ): FieldOf<{ value: string; reference: "object" }> {
        // refuse an object type representing no principal kind
        kinds.forEach((kind) => Field.principal(kind));

        return new Field<FieldConfigurationOf<{ value: string; reference: "object" }>>({
            source: { builder: "subject", arguments: kinds },
            build: (name) => text(name).validate(schema.string().min(1)),
            ...WRITTEN,
            reference: "object",
        });
    },

    /** An object of the given types, or of any type, stored as its reference. */
    object(
        ...types: readonly (ObjectType | Policy)[]
    ): FieldOf<{ value: string; reference: "object" }> {
        return new Field<FieldConfigurationOf<{ value: string; reference: "object" }>>({
            source: { builder: "object", arguments: types },
            build: (name) => text(name).validate(schema.string().min(1)),
            ...WRITTEN,
            reference: "object",
        });
    },

    /** An object in the same space by identifier, or a principal of a global kind. */
    reference,

    /** A fractional index ordering objects among their siblings. */
    fractionalIndex() {
        return Field.required({ builder: "fractionalIndex", arguments: [] }, (name) =>
            text(name).validate(FractionalIndex),
        );
    },

    /** A text stored in chunks and changed by edits, empty at first. */
    text(): TextField {
        return new TextField();
    },
};
