import type { IconName } from "@destack/icon";
import {
    ActionInput,
    type InputPresence,
    type ActionValue,
    type VALUE_KINDS,
} from "../declare/input.ts";
import { NumberInput } from "../declare/number.ts";
import { PackageId } from "@destack/package";
import {
    bigint,
    binary,
    boolean,
    ColumnBuilder,
    TABLE,
    integer,
    blob,
    type BlobOptions,
    blobs,
    blobMap,
    json,
    real,
    text,
    type Column,
    type ColumnDefinition,
    type Condition,
    type ColumnValue,
    type SQL,
    type Encryption,
    type Select,
    type Table,
    textEnum,
} from "@destack/db";
import { Message, NumberFormat } from "@destack/locale";
import { canonicalize, schema, toJsonSchema, type Identifier, JsonValue } from "@destack/schema";
import type { BuilderCall } from "../object/description.ts";
import type { ObjectConstraint } from "../object/constraint.ts";
import type { IdentityOf, ObjectStorage, ObjectType } from "../object/object.ts";
import { Policy } from "@destack/access";
import { FractionalIndex } from "./fractional-index.ts";
import { type ObjectFileLimits, ObjectFile } from "../file/file.ts";
import type { MethodVisibility } from "../method/method.ts";
import { ChoiceDefinition, type ChoiceDefinitions, type FieldChoice } from "./choice.ts";

import {
    type TextMarkType,
    type TextNodeType,
    TextSchema,
    type TextSpan,
} from "../sequence/index.ts";
/** The fields of a structured value. */
const STRUCTURE = schema.looseObject({});

/** The path to the digests of a file's parts in it. */
const PARTS = "parts";

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
    "blob",
    "blobs",
    "blobMap",
    "state",
    "text",
] as const;

/** The meaning of a field beyond its stored value. */
export type FieldType = (typeof FIELD_TYPES)[number];

/** The field type of each builder: an aggregate's value type, else the builder's type. */
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
    blob: "blob",
    blobs: "blobs",
    blobMap: "blobMap",
    subject: "subject",
    object: "object",
    state: "state",
    reference: "reference",
};

/** The JSON Schema of any JSON value, which a field of no schema of its own takes. */
const ANY_JSON = JsonValue.object(toJsonSchema(schema.json()));

/** The kinds of value people give for a field, by its meaning. */
const VALUE_KIND: Readonly<Partial<Record<FieldType, (typeof VALUE_KINDS)[number]>>> = {
    string: "text",
    text: "text",
    boolean: "boolean",
    time: "time",
    subject: "person",
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
    /** How each state looks, by state, as stored without it. */
    readonly states?: ChoiceDefinitions;
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
                /** Who may call the transition, clients too when it declares none. */
                readonly visibility?: MethodVisibility;
            }
        >
    >;
}

/** Where a state lies in its machine: initial, active, or final where no transition leaves. */
export type StateStage = "initial" | "active" | "final";

/** A state machine a field follows. */
export const StateMachine = {
    /** List a machine's states, the initial one first, the others as transitions reach them. */
    states(machine: StateMachine): [string, ...string[]] {
        // list what transitions leave and enter beside the initial state
        const reached = Object.values(machine.transitions).flatMap((each) => [
            ...each.from,
            each.to,
        ]);
        const others = new Set(reached);
        others.delete(machine.initial);

        return [machine.initial, ...others];
    },

    /** Read where a state lies in a machine. */
    stage(machine: StateMachine, state: string): StateStage {
        // start at the initial state, end where no transition leaves, and pass the rest
        const isLeft = Object.values(machine.transitions).some((transition) =>
            transition.from.includes(state),
        );
        if (state === machine.initial) {
            return "initial";
        } else if (isLeft) {
            return "active";
        }

        return "final";
    },
};

/** The builder call of a field: the builder's name and its arguments as written. */
export type FieldSource =
    | { readonly builder: "string"; readonly arguments: readonly [schema.Schema<string>] }
    | {
          readonly builder: "integer" | "number";
          readonly arguments: readonly [] | readonly [schema.Schema<number>];
      }
    | {
          readonly builder:
              | "bigint"
              | "binary"
              | "boolean"
              | "time"
              | "fractionalIndex"
              | "message"
              | "count"
              | "sum"
              | "min"
              | "max";
          readonly arguments: readonly [];
      }
    | {
          readonly builder: "enum";
          readonly arguments: readonly [
              values: readonly [string, ...string[]],
              choices?: ChoiceDefinitions,
          ];
      }
    | {
          readonly builder: "json";
          readonly arguments: readonly [schema.Schema] | readonly [schema.Schema, JsonOptions];
      }
    | { readonly builder: "file"; readonly arguments: readonly [ObjectFileLimits] }
    | {
          readonly builder: "text";
          readonly arguments: readonly [] | readonly [readonly (TextMarkType | TextNodeType)[]];
      }
    | {
          readonly builder: "blob" | "blobs" | "blobMap";
          readonly arguments: readonly [BlobOptions];
      }
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
    "labeled",
    "described",
    "formatted",
    "open",
    "caller",
    "guard",
    "guardWrite",
    "relate",
] as const;

/** How each value of an enum looks, by value. */
const ChoiceMap = schema.record(schema.string(), ChoiceDefinition);

/** The values an enum field stores: a list's, or a map's keys. */
type EnumValueOf<Values> = Values extends readonly string[]
    ? Values[number]
    : keyof Values & string;

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
    /** Whether reading the value requires a separate permission. */
    readonly guarded: boolean;
    /** Whether callers write the value: always, only at creation, or never. */
    readonly written: boolean | "create";
    /** Whether the value is sensitive. */
    readonly sensitive: boolean;
    /** Whether creation fills the value with the caller, outside server-only creations. */
    readonly caller: boolean;
    /** How the value refers to another object: not, by identifier, or by object reference. */
    readonly reference: ReferenceKind;
    /** The relation a reference declares, absent for other fields. */
    readonly relation: string;
    /** Whether a subject field is open to contributed subjects and sets. */
    readonly open: boolean;
}

/** A field's flags as the values it stores, each typed by its signature's flag. */
interface FieldFlags<Configuration extends FieldConfiguration = FieldConfiguration> {
    /** Whether every object must have a value. */
    readonly required: Configuration["required"];
    /** Whether creation supplies a value the caller omits. */
    readonly isDefault: Configuration["default"];
    /** Whether reading the value requires a separate permission. */
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
    Options extends Partial<FieldConfiguration> & { readonly value: ColumnValue },
> = {
    readonly value: Options["value"];
    readonly required: Options extends { readonly required: infer Flag } ? Flag : true;
    readonly default: Options extends { readonly default: infer Flag } ? Flag : false;
    readonly guarded: Options extends { readonly guarded: infer Flag } ? Flag : false;
    readonly written: Options extends { readonly written: infer Flag } ? Flag : true;
    readonly sensitive: Options extends { readonly sensitive: infer Flag } ? Flag : false;
    readonly caller: Options extends { readonly caller: infer Flag } ? Flag : false;
    readonly reference: Options extends { readonly reference: infer Kind } ? Kind : "none";
    readonly relation: never;
    readonly open: Options extends { readonly open: infer Flag } ? Flag : false;
};

/** A field of a declared signature, each absent flag at its default. */
export type FieldOf<Options extends Partial<FieldConfiguration> & { readonly value: ColumnValue }> =
    Field<FieldConfigurationOf<Options>>;

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
    readonly open: Configuration["open"];
};

/** What a structured value keeps alive. */
export interface JsonOptions {
    /** The dotted path to the digests of blobs, such as `file.parts`. */
    readonly blobs: string;
    /** The media types and sizes of the file whose parts they are. */
    readonly limits: ObjectFileLimits;
}

/** How a field refers to another object: not, by identifier, or by object reference. */
export type ReferenceKind = "none" | "identifier" | "object";

/** The column suffixes of an object reference beside its identifier. */
const REFERENCE_COLUMNS = ["PackageId", "Type", "Scope"] as const;

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
    readonly [Entry in ReferenceColumnOf<Fields> as `${Entry["key"]}${Entry["suffix"]}`]: Field<
        Override<
            Entry["configuration"],
            {
                readonly value: Entry["suffix"] extends "PackageId" ? PackageId : string;
                readonly reference: "none";
            }
        >
    >;
} & {
    readonly [Key in OpenFieldOf<Fields> as `${Key}Relation`]: Fields[Key] extends Field<
        infer Configuration
    >
        ? Field<
              Override<
                  Configuration,
                  { readonly value: string; readonly reference: "none"; readonly required: false }
              >
          >
        : never;
};

/** The subject fields open to contributed subject sets, by key. */
type OpenFieldOf<Fields> = {
    [Key in keyof Fields & string]: Fields[Key] extends Field<infer Configuration>
        ? Configuration["open"] extends true
            ? Key
            : never
        : never;
}[keyof Fields & string];

/** Each object reference field's columns beside its identifier, as key, suffix and signature. */
type ReferenceColumnOf<Fields> = {
    [Key in keyof Fields & string]: Fields[Key] extends Field<infer Configuration>
        ? Configuration["reference"] extends "object"
            ? {
                  readonly key: Key;
                  readonly suffix: (typeof REFERENCE_COLUMNS)[number];
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
    /** The field's name as people read it. */
    readonly label?: Message;
    /** The icon beside the field's name. */
    readonly icon?: IconName;
    /** One sentence on what the field stores and when to change it. */
    readonly description?: Message;
    /** How people read a number field's values. */
    readonly format?: NumberFormat;
    /** Whether a subject field accepts the subjects and sets other types contribute. */
    readonly isOpen?: boolean;
    /** How a sensitive value is encrypted at rest, randomly unless a lookup finds it by value. */
    readonly encryption?: Encryption;
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
    /** Whether reading the value requires a separate permission. */
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
    /** The field's name as people read it, absent to read it from its property. */
    readonly label: Message | undefined;
    /** The icon beside the field's name, absent for none. */
    readonly icon: IconName | undefined;
    /** One sentence on what the field stores and when to change it, absent for none. */
    readonly description: Message | undefined;
    /** How people read a number field's values, absent for plain numbers and other fields. */
    readonly format: NumberFormat | undefined;
    /** Whether a subject field accepts contributed subjects and sets. */
    readonly isOpen: boolean;
    /** How a sensitive value is encrypted at rest, randomly unless a lookup finds it by value. */
    readonly encryption: Encryption;
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
        this.label = definition.label;
        this.icon = definition.icon;
        this.description = definition.description;
        this.format = definition.format;
        this.isOpen = definition.isOpen ?? false;
        this.encryption = definition.encryption ?? "randomized";
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

    /** Build the action input the field takes: an object, a choice, or a value of its kind. */
    actionInput(name: string, presence?: InputPresence): ActionInput {
        // take an object of a reference's type, a choice of an enum's, or a value
        const type = this.type === "reference" ? this.target?.() : undefined;
        const { source } = this;
        const value: ActionValue =
            type !== undefined
                ? { kind: "object", type, source: "picked", where: undefined }
                : source.builder === "enum"
                  ? { kind: "choice", choices: this.choices ?? [] }
                  : source.builder === "integer" || source.builder === "number"
                    ? { kind: "number", ...Field.numbers(source, this.format) }
                    : source.builder === "json"
                      ? {
                            kind: "json",
                            schema: JsonValue.object(toJsonSchema(source.arguments[0])),
                        }
                      : this.principals !== undefined
                        ? { kind: "person" }
                        : Field.plain(VALUE_KIND[this.type]);
        const declared = this.isDefault ? "default" : this.required ? "required" : "optional";

        return new ActionInput({
            name,
            label: this.label,
            presence: presence ?? declared,
            value,
            icon: this.icon,
            description: this.description,
        });
    }

    /** The aggregate function maintaining the value. */
    get aggregate(): AggregateFunction | undefined {
        const { builder } = this.source;

        return builder === "count" || builder === "sum" || builder === "min" || builder === "max"
            ? builder
            : undefined;
    }

    /** The values an enum or a state takes, with how each looks, absent for other fields. */
    get choices(): readonly FieldChoice[] | undefined {
        // read an enum's values, or a machine's states from the initial one as they are reached
        const { machine, source } = this;
        const values =
            source.builder === "enum"
                ? source.arguments[0]
                : machine === undefined
                  ? undefined
                  : StateMachine.states(machine);
        const choices = source.builder === "enum" ? source.arguments[1] : machine?.states;

        return values?.map((value) => ({ value, ...choices?.[value] }));
    }

    /** The state machine the value follows. */
    get machine(): StateMachine | undefined {
        return this.source.builder === "state" ? this.source.arguments[0] : undefined;
    }

    /** The media types and sizes a file field takes, absent for another field. */
    get limits(): ObjectFileLimits | undefined {
        return this.source.builder === "file" ? this.source.arguments[0] : undefined;
    }

    /** Find the file in a value of the field, with the limits it keeps to. */
    file(
        value: unknown,
    ): { readonly file: ObjectFile; readonly limits: ObjectFileLimits } | undefined {
        // read a file field's value, else the file whose parts a structured value's path leads to
        const { source } = this;
        const options =
            source.builder === "file"
                ? { blobs: PARTS, limits: source.arguments[0] }
                : source.builder === "json"
                  ? source.arguments[1]
                  : undefined;
        const steps = options?.blobs.split(".").slice(0, -1) ?? [];
        const found = steps.reduce<unknown>(
            (each, name) => STRUCTURE.safeParse(each).data?.[name],
            value,
        );

        return options === undefined || found === undefined || found === null
            ? undefined
            : { file: ObjectFile.parse(found), limits: options.limits };
    }

    /** Create the column storing the field, its definition following the field's flags. */
    column(name: string, owner: string, storage: ObjectStorage): ColumnBuilder {
        // build the nullable column
        let column: ColumnBuilder = this.#build(name, owner, storage);

        // mark sensitive and personal values
        if (this.isSensitive) {
            column = column.sensitive(this.encryption);
        } else if (this.isPersonal) {
            column = column.personal();
        }

        // supply the default
        if (this.initial) {
            column = column.default(this.initial.value);
        }

        // require a value, except where replicas store a guarded value concealed
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

    /** Encrypt the value at rest, out of logs, audit details, sync and fingerprints. */
    sensitive(
        encryption: Encryption = "randomized",
    ): Field<Override<Configuration, { readonly sensitive: true }>> {
        return this.#with({ ...this.#flags, isSensitive: true }, { encryption });
    }

    /** Mark the value as personal data, exported and erased with its subject. */
    personal(): Field<Configuration> {
        return new Field<Configuration>({ ...this.#options, ...this.#flags, isPersonal: true });
    }

    /** Label the field as people read it, such as Due at, with an icon beside the label. */
    labeled(label: Message, icon?: IconName): Field<Configuration> {
        return new Field<Configuration>({
            ...this.#options,
            ...this.#flags,
            label,
            ...(icon === undefined ? {} : { icon }),
        });
    }

    /** Describe in one sentence what the field stores and when to change it. */
    described(description: Message): Field<Configuration> {
        return new Field<Configuration>({ ...this.#options, ...this.#flags, description });
    }

    /** Write the field's numbers as people read them, such as a share or an amount. */
    formatted(format: NumberFormat): Field<Configuration> {
        // refuse a format of no number
        if (this.type !== "integer" && this.type !== "number") {
            throw new TypeError("only number fields have a number format");
        }

        // refuse a currency or a unit no language writes
        if (!NumberFormat.safeParse(format).success) {
            throw new TypeError(`no language writes the format ${JSON.stringify(format)}`);
        }

        return new Field<Configuration>({ ...this.#options, ...this.#flags, format });
    }

    /** Accept the subjects and subject sets other types contribute. */
    open(): Field<Override<Configuration, { readonly open: true }>> {
        if (this.type !== "subject") {
            throw new TypeError("only subject fields are open to contributed subjects");
        }

        return new Field<Override<Configuration, { readonly open: true }>>({
            ...this.#options,
            ...this.#flags,
            isOpen: true,
        });
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

    /** Build the field a builder call makes alone, before any modifier. */
    static #replay(source: FieldSource): Field {
        // refuse a builder call making no field
        const built: unknown = Reflect.apply(field[source.builder], field, source.arguments);
        if (!Field.is(built)) {
            throw new TypeError(`field builder '${source.builder}' does not make a field`);
        }

        return built;
    }

    /** Decide whether a value is a field. */
    static is(value: unknown): value is Field {
        return value instanceof Field;
    }

    /** Describe the field as builder calls: its builder's, then each effective modifier's. */
    describe(): readonly [FieldSource, ...BuilderCall[]] {
        // compare with the field the builder call makes alone
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
                          arguments: this.encryption === "randomized" ? [] : [this.encryption],
                      },
                  ]
                : []),
            ...(this.isPersonal && !plain.isPersonal
                ? [{ builder: "personal", arguments: [] }]
                : []),
            ...(this.label === undefined
                ? []
                : [
                      {
                          builder: "labeled",
                          arguments:
                              this.icon === undefined ? [this.label] : [this.label, this.icon],
                      },
                  ]),
            ...(this.description === undefined
                ? []
                : [{ builder: "described", arguments: [this.description] }]),
            ...(this.format === undefined
                ? []
                : [{ builder: "formatted", arguments: [this.format] }]),
            ...(this.isOpen && !plain.isOpen ? [{ builder: "open", arguments: [] }] : []),
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
                [columns.packageId]: this.#column(packages),
                [columns.type]: this.#column(types),
                [columns.scope]: this.#column(schema.string().min(1)),
                [columns.id]: this.relate(property),
                ...(this.isOpen
                    ? {
                          [Field.relationColumnOf(property)]: this.#column(
                              schema.string().min(1),
                              false,
                          ),
                      }
                    : {}),
            };
        }

        return { [property]: this };
    }

    /** Name the column holding the relation of a subject set a field names. */
    static relationColumnOf(property: string): string {
        return `${property}Relation`;
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
    #column(validator: schema.Schema<string>, isRequired: boolean = this.required): Field {
        return new Field({
            source: { builder: "string", arguments: [validator] },
            build: (name) => Field.validated(name, validator),
            isPersonal: this.isPersonal,
            ...(this.access === undefined ? {} : { access: this.access }),
            ...this.#flags,
            required: isRequired,
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
            ...(this.label === undefined ? {} : { label: this.label }),
            ...(this.icon === undefined ? {} : { icon: this.icon }),
            ...(this.description === undefined ? {} : { description: this.description }),
            ...(this.format === undefined ? {} : { format: this.format }),
            isOpen: this.isOpen,
            encryption: this.encryption,
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
        change: Pick<FieldOptions<Configuration>, "initial" | "access" | "encryption"> = {},
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

    /** Read what a field of a plain kind takes, any JSON for a field of none. */
    static plain(kind: (typeof VALUE_KINDS)[number] | undefined): ActionValue {
        return kind === undefined ? { kind: "json", schema: ANY_JSON } : { kind };
    }

    /** Read the numbers a number field takes, and how people read them. */
    static numbers(
        source: Extract<FieldSource, { readonly builder: "integer" | "number" }>,
        format: NumberFormat | undefined,
    ): NumberInput {
        const [validator] = source.arguments;
        const plain = source.builder === "integer" ? schema.number().int() : schema.number();

        return NumberInput.of(validator ?? plain, format);
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
        // leave references in memory, in the event log and to replicas that come and go without a foreign key
        if (storage === "ephemeral" || storage === "events" || options.delete === "keep") {
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
            throw new TypeError(`object '${kind.name}' has no principal kind to represent`);
        }

        return kind.policy;
    }

    /** Define the nullable text column a string schema validates, typed by the schema's output. */
    static validated<Validator extends schema.Schema<string>>(
        name: string,
        validator: Validator,
    ): ColumnBuilder<ColumnDefinition<schema.Output<Validator>, string>> {
        return text(name).validate(validator);
    }
}

/** The options of a reference field. */
interface ReferenceOptions {
    /** What deleting the referenced object does to this one, or nothing for a replicated target. */
    readonly delete?: "restrict" | "cascade" | "null" | "keep";
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
            `principal kind '${target.name}' belongs to scopes: use field.subject(principal.'${target.name}')`,
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

            // accept the target's identifier, or its key, which its store resolves to the identifier
            return Field.keyed(
                Field.validated(
                    name,
                    schema.lazy(() => resolve().addressSchema),
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
                    `a reference to principal kind '${resolved.name}' must take the kind, not its policy`,
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
        // list the states, refusing to describe one no transition reaches
        const names = StateMachine.states(machine);
        const unknown = Object.keys(machine.states ?? {}).find((name) => !names.includes(name));
        if (unknown !== undefined) {
            throw new TypeError(`state '${unknown}' is invalid: no transition reaches it`);
        }

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

/** A text field stored in chunks, written at creation and by edits after. */
export class TextField extends Field<
    FieldConfigurationOf<{ value: readonly TextSpan[]; default: true; written: "create" }>
> {
    /** The marks and nodes the field allows beside characters. */
    readonly schema: TextSchema;

    /** Declare a text field with the mark and node types it allows. */
    constructor(types: readonly (TextMarkType | TextNodeType)[]) {
        super({
            source: { builder: "text", arguments: types.length === 0 ? [] : [types] },
            initial: { value: [] },
            build: () => {
                throw new TypeError(
                    "a text field stores its characters in chunks and has no column",
                );
            },
            ...WRITTEN,
            isDefault: true,
            written: "create",
        });
        this.schema = TextSchema.of(...types);
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

    /** A whole number within the exactly representable range, or that a schema validates. */
    integer(validator?: schema.Schema<number>): FieldOf<{ value: number }> {
        // refuse a schema of other numbers
        if (validator !== undefined && toJsonSchema(validator).type !== "integer") {
            throw new TypeError("an integer field takes a schema of whole numbers");
        }

        return Field.required(
            { builder: "integer", arguments: validator === undefined ? [] : [validator] },
            (name) => (validator === undefined ? integer(name) : integer(name).validate(validator)),
        );
    },

    /** A whole number of 64 bits, beyond the exact range of a double. */
    bigint() {
        return Field.required({ builder: "bigint", arguments: [] }, bigint);
    },

    /** A double-precision number, or one a schema validates. */
    number(validator?: schema.Schema<number>): FieldOf<{ value: number }> {
        return Field.required(
            { builder: "number", arguments: validator === undefined ? [] : [validator] },
            (name) => (validator === undefined ? real(name) : real(name).validate(validator)),
        );
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

    /** The digest of a blob in the partition's blob store, kept alive by the row. */
    blob(options: BlobOptions = {}) {
        return Field.required({ builder: "blob", arguments: [options] }, (name) =>
            blob(name, options),
        );
    },

    /** The digests of blobs in the partition's blob store, kept alive by the row. */
    blobs(options: Extract<BlobOptions, { onDelete?: "restrict" }> = {}) {
        return Field.required({ builder: "blobs", arguments: [options] }, (name) =>
            blobs(name, options),
        );
    },

    /** The digests of blobs in the partition's blob store by key, kept alive by the row. */
    blobMap(options: Extract<BlobOptions, { onDelete?: "restrict" }> = {}) {
        return Field.required({ builder: "blobMap", arguments: [options] }, (name) =>
            blobMap(name, options),
        );
    },

    /** True or false. */
    boolean() {
        return Field.required({ builder: "boolean", arguments: [] }, boolean);
    },

    /** An instant in UTC epoch milliseconds. */
    time() {
        return Field.required({ builder: "time", arguments: [] }, integer);
    },

    /** One of a fixed set of strings: a list, or each value with how it looks. */
    enum<const Values extends readonly [string, ...string[]] | ChoiceDefinitions>(
        values: Values,
        choices?: ChoiceDefinitions,
    ): FieldOf<{ value: EnumValueOf<Values> }> {
        // read the values in order, of a map by its keys
        const isList = Array.isArray(values);
        const [first, ...rest] = isList ? values.map(String) : Object.keys(values);
        if (first === undefined) {
            throw new TypeError("an enum field needs a value");
        }
        const names: [string, ...string[]] = [first, ...rest];
        const looks = isList ? choices : ChoiceMap.parse(values);

        return new Field<FieldConfigurationOf<{ value: EnumValueOf<Values> }>>({
            source: { builder: "enum", arguments: looks === undefined ? [names] : [names, looks] },
            build: (name) => textEnum(name, names),
            ...WRITTEN,
        });
    },

    /** A state machine changed by its transition methods. */
    state<const Machine extends StateMachine>(machine: Machine): StateField<Machine> {
        return new StateField(machine);
    },

    /** A structured value validated by a schema, keeping alive the blobs at a path in it. */
    json<Value extends JsonValue>(validator: schema.Schema<Value>, options?: JsonOptions) {
        return options === undefined
            ? Field.required({ builder: "json", arguments: [validator] }, (name) =>
                  json(name, validator),
              )
            : Field.required({ builder: "json", arguments: [validator, options] }, (name) =>
                  json(name, validator, { blobs: options.blobs }),
              );
    },

    /** A message the declaring package's catalogs translate, with its identifier and source. */
    message() {
        return Field.required({ builder: "message", arguments: [] }, (name) => json(name, Message));
    },

    /** A file of given media types and sizes, its bytes kept alive by the row. */
    file(limits: ObjectFileLimits = {}): FieldOf<{ value: ObjectFile }> {
        return new Field<FieldConfigurationOf<{ value: ObjectFile }>>({
            source: { builder: "file", arguments: [limits] },
            build: (name) => json(name, ObjectFile, { blobs: PARTS }),
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

    /** A text stored in chunks and changed by edits, allowing some marks and nodes. */
    text(types: readonly (TextMarkType | TextNodeType)[] = []): TextField {
        return new TextField(types);
    },
};
