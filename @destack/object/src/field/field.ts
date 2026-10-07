import {
    boolean,
    ColumnBuilder,
    TABLE,
    identifier,
    integer,
    json,
    real,
    text,
    type Column,
    type ColumnDefinition,
    type ColumnValue,
    type SQL,
    type Select,
    type Table,
} from "@destack/db";
import { schema, type Identifier, type JsonValue } from "@destack/schema";
import type { IdentityOf, ObjectStorage, ObjectType } from "../object/object.ts";
import { Policy } from "@destack/access";
import { FractionalIndex } from "./fractional-index.ts";
import { type FileBucket, ObjectFile } from "./file.ts";

/** The meanings of fields beyond their stored values. */
export const FIELD_TYPES = [
    "string",
    "integer",
    "number",
    "boolean",
    "time",
    "enum",
    "json",
    "reference",
    "subject",
    "fractionalIndex",
    "file",
    "state",
    "text",
] as const;

/** The meaning of a field beyond its stored value. */
export type FieldType = (typeof FIELD_TYPES)[number];

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
            }
        >
    >;
}

/** Every state a state machine names. */
export type StateOf<Machine extends StateMachine> =
    | Machine["initial"]
    | Machine["transitions"][keyof Machine["transitions"]]["from"][number]
    | Machine["transitions"][keyof Machine["transitions"]]["to"];

/** A field following a state machine. */
export type StateField<Machine extends StateMachine = StateMachine> = FieldOf<{
    value: StateOf<Machine>;
    default: true;
    written: false;
}> & { readonly machine: Machine };

/** A text field, kept in chunks and changed only by edits. */
export type TextField = FieldOf<{ value: string; default: true; written: false }> & {
    readonly type: "text";
};

/** The aggregate function keeping a field current. */
export type AggregateFunction = "count" | "sum" | "min" | "max";

/** The column definition a field declares: not null when required, with a default when it has one, sensitive when classified so. */
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
    /** The value the field keeps. */
    readonly value: ColumnValue;
    /** Whether every object must have a value. */
    readonly required: boolean;
    /** Whether creation supplies a value the caller omits. */
    readonly default: boolean;
    /** Whether reading the value requires its own permission. */
    readonly guarded: boolean;
    /** Whether callers write the value. */
    readonly written: boolean;
    /** Whether the value is sensitive. */
    readonly sensitive: boolean;
    /** Whether creation fills the value with the calling principal, outside system creations. */
    readonly caller: boolean;
    /** Whether the value is another object's or a principal's identifier, which the object keeps under `<relation>Id`. */
    readonly reference: boolean;
    /** The relation a reference declares, never for other fields. */
    readonly relation: string;
}

/** A field's flags as the values it keeps, each typed by its signature's flag. */
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
    /** Whether the value is another object's or a principal's identifier. */
    readonly isReference: Configuration["reference"];
}

/** The flags of a required, written, unguarded field. */
const WRITTEN = {
    required: true,
    isDefault: false,
    guarded: false,
    written: true,
    isSensitive: false,
    isCallerFilled: false,
    isReference: false,
} as const;

/** The signature of a required, written, unguarded field a declaration names, each absent flag at its default. */
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
    readonly reference: Declared extends { readonly reference: infer Flag } ? Flag : false;
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

/** A field's signature keeping its value and relation, its flags the types of the values kept. */
type FlagConfiguration<Configuration extends FieldConfiguration, Flags extends FieldFlags> = {
    readonly value: Configuration["value"];
    readonly relation: Configuration["relation"];
    readonly required: Flags["required"];
    readonly default: Flags["isDefault"];
    readonly guarded: Flags["guarded"];
    readonly written: Flags["written"];
    readonly sensitive: Flags["isSensitive"];
    readonly caller: Flags["isCallerFilled"];
    readonly reference: Flags["isReference"];
};

/** The fields as an object keeps them, each identifier reference under `<relation>Id` declaring its relation. */
export type ObjectFields<Fields> = {
    readonly [
        Key in keyof Fields & string as Fields[Key] extends Field<infer Configuration>
            ? Configuration["reference"] extends true
                ? `${Key}Id`
                : Key
            : Key
    ]: Fields[Key] extends Field<infer Configuration>
        ? Configuration["reference"] extends true
            ? Field<Override<Configuration, { readonly relation: Key }>>
            : Fields[Key]
        : Fields[Key];
};

/** Build the column of a field of a value. */
type BuildColumn<Value extends ColumnValue> = (
    name: string,
    owner: string,
    storage: ObjectStorage,
) => ColumnBuilder<ColumnDefinition<Value>>;

/** What a field keeps beside its flags. */
interface FieldOptions<Configuration extends FieldConfiguration> {
    /** The field's meaning. */
    readonly type: FieldType;
    /** Create the nullable column storing the field. */
    readonly build: BuildColumn<Configuration["value"]>;
    /** The referenced object type, for references to objects. */
    readonly target?: (() => ObjectType) | undefined;
    /** The principal kinds a principal reference or subject field accepts. */
    readonly principals?: readonly Policy[] | undefined;
    /** Whether a reference keeps the scope beside the identifier. */
    readonly qualified?: boolean | undefined;
    /** The relation a reference declares. */
    readonly relation?: Configuration["relation"] | undefined;
    /** The value creation supplies when the caller omits the field. */
    readonly initial?: { readonly value: Configuration["value"] } | undefined;
    /** Whether the value is personal data. */
    readonly isPersonal?: boolean | undefined;
    /** The permissions reading or writing the value requires. */
    readonly access?: FieldAccess | undefined;
    /** The aggregate function keeping the value current. */
    readonly aggregate?: AggregateFunction | undefined;
    /** The state machine the value follows. */
    readonly machine?: StateMachine | undefined;
    /** The bucket a file field keeps its files in. */
    readonly bucket?: FileBucket | undefined;
}

/** A field of an object. */
export class Field<Configuration extends FieldConfiguration = FieldConfiguration> {
    /** The field's meaning. */
    readonly type: FieldType;
    /** The referenced object type, for references to objects. */
    readonly target: (() => ObjectType) | undefined;
    /** The principal kinds a principal reference or subject field accepts. */
    readonly principals: readonly Policy[] | undefined;
    /** Whether a reference keeps the scope beside the identifier. */
    readonly qualified: boolean;
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
    /** Whether the value is another object's or a principal's identifier. */
    readonly isReference: Configuration["reference"];
    /** The value creation supplies when the caller omits the field. */
    readonly initial: { readonly value: Configuration["value"] } | undefined;
    /** Whether the value is personal data, exported and erased with its subject. */
    readonly isPersonal: boolean;
    /** The permissions reading or writing the value requires. */
    readonly access: FieldAccess | undefined;
    /** The aggregate function keeping the value current. */
    readonly aggregate: AggregateFunction | undefined;
    /** The state machine the value follows. */
    readonly machine: StateMachine | undefined;
    /** The bucket a file field keeps its files in. */
    readonly bucket: FileBucket | undefined;
    /** Create the nullable column storing the field. */
    readonly #build: BuildColumn<Configuration["value"]>;

    /** Retain the field's meaning, storage and flags. */
    constructor(definition: FieldOptions<Configuration> & FieldFlags<Configuration>) {
        // retain the declaration and its column builder
        this.type = definition.type;
        this.target = definition.target;
        this.principals = definition.principals;
        this.qualified = definition.qualified ?? false;
        this.relation = definition.relation;
        this.initial = definition.initial;
        this.isPersonal = definition.isPersonal ?? false;
        this.access = definition.access;
        this.aggregate = definition.aggregate;
        this.machine = definition.machine;
        this.bucket = definition.bucket;
        this.#build = definition.build;

        // retain the flags
        this.required = definition.required;
        this.isDefault = definition.isDefault;
        this.guarded = definition.guarded;
        this.written = definition.written;
        this.isSensitive = definition.isSensitive;
        this.isCallerFilled = definition.isCallerFilled;
        this.isReference = definition.isReference;
    }

    /** Create the column storing the field, its definition following the field's flags. */
    column(
        name: string,
        owner: string,
        storage: ObjectStorage,
    ): ColumnBuilder<
        FieldColumn<
            Configuration["value"],
            Configuration["required"],
            Configuration["default"],
            Configuration["sensitive"]
        >
    >;
    /**
     * Create the column storing the field, built from the field's flags.
     *
     * @construct the column has the nullability, default and sensitivity the field's flags set, which is how FieldColumn maps the signature.
     */
    column(name: string, owner: string, storage: ObjectStorage): ColumnBuilder {
        // build the nullable column
        let column: ColumnBuilder = this.#build(name, owner, storage);

        // mark sensitive and personal values
        if (this.isSensitive) {
            column = column.sensitive();
        } else if (this.isPersonal) {
            column = column.personal();
        }

        // supply the default
        if (this.initial) {
            column = column.default(this.initial.value);
        }

        // require a value, except where copies keep a guarded value concealed
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

    /** Keep the value out of logs, audit details, sync and request fingerprints. */
    sensitive(): Field<Override<Configuration, { readonly sensitive: true }>> {
        return this.#with({ ...this.#flags, isSensitive: true });
    }

    /** Mark the value as personal data, exported and erased with its subject. */
    personal(): Field<Configuration> {
        return new Field<Configuration>({ ...this.#options, ...this.#flags, isPersonal: true });
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
                "only principal references and subject fields take the calling principal",
            );
        }

        return this.#with({
            ...this.#flags,
            isDefault: true,
            written: false,
            isCallerFilled: true,
        });
    }

    /**
     * Require a permission to read the value, and optionally another to write it.
     *
     * A field guarded to read stays required to write, while its column is nullable: readers without the permission and copies in other databases keep it concealed.
     */
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

    /** Read the relation the field keeps under a property: a reference's declared relation, any other field's property. */
    relationAt(property: string): string {
        return this.relation ?? property;
    }

    /** Declare the relation a reference keeps, as its object keys it under `<relation>Id`. */
    relate<const Relation extends string>(
        relation: Relation,
    ): Field<Override<Configuration, { readonly relation: Relation }>> {
        return new Field<Override<Configuration, { readonly relation: Relation }>>({
            ...this.#options,
            ...this.#flags,
            relation,
        });
    }

    /** Read what the field keeps beside its flags. */
    get #options(): FieldOptions<Configuration> {
        return {
            type: this.type,
            build: this.#build,
            target: this.target,
            principals: this.principals,
            qualified: this.qualified,
            relation: this.relation,
            initial: this.initial,
            isPersonal: this.isPersonal,
            access: this.access,
            aggregate: this.aggregate,
            machine: this.machine,
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
            isReference: this.isReference,
        };
    }

    /** Derive a field keeping the given flags, its signature the types of those flags. */
    #with<const Flags extends FieldFlags>(
        flags: Flags,
        change: Pick<FieldOptions<Configuration>, "initial" | "access"> = {},
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
            isReference: flags.isReference,
        });
    }
}

/** A qualified reference's scope and identifier. */
type QualifiedValue = { readonly scope: string; readonly id: string };

/** The value of a reference. */
export type ReferenceValue = string | QualifiedValue;

/** Build a required, written field of one type from a nullable column. */
function required<Value extends ColumnValue>(
    type: FieldType,
    build: BuildColumn<Value>,
): FieldOf<{ value: Value }> {
    return new Field<FieldConfigurationOf<{ value: Value }>>({ type, build, ...WRITTEN });
}

/** Define the nullable text column a string schema validates, typed by the schema's output. */
export function validated<Validator extends schema.Schema<string>>(
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

/** Declare a field an aggregate keeps, starting at a value. */
function aggregated(
    aggregate: "count" | "sum",
    type: FieldType,
    build: (name: string) => ColumnBuilder<ColumnDefinition<number>>,
    initial: number,
): FieldOf<{ value: number; default: true; written: false }> {
    return new Field<FieldConfigurationOf<{ value: number; default: true; written: false }>>({
        type,
        build,
        initial: { value: initial },
        aggregate,
        ...WRITTEN,
        isDefault: true,
        written: false,
    });
}

/** Declare a field an extreme keeps, absent until a belonging object has a value. */
function extreme(
    aggregate: "min" | "max",
    type: FieldType,
    build: (name: string) => ColumnBuilder<ColumnDefinition<number>>,
): FieldOf<{ value: number; required: false; written: false }> {
    return new Field<FieldConfigurationOf<{ value: number; required: false; written: false }>>({
        type,
        build,
        aggregate,
        ...WRITTEN,
        required: false,
        written: false,
    });
}

/** The options of a reference field. */
interface ReferenceOptions {
    /** Keep the scope beside the identifier. */
    readonly qualified?: boolean;
    /** What deleting the referenced object does to this one. */
    readonly delete?: "restrict" | "cascade" | "null";
}

/** The options of a reference keeping only the identifier. */
type UnqualifiedOptions = ReferenceOptions & { readonly qualified?: false };

/** The options of a reference keeping the scope beside the identifier. */
type QualifiedOptions = ReferenceOptions & { readonly qualified: true };

/** The identifier of an object type's objects. */
export type IdentifierOf<Target extends ObjectType> = Identifier<IdentityOf<Target>>;

/** The value naming one of an object type's objects: its natural key, or its generated identifier. */
type ReferenceOf<Target extends ObjectType> =
    Select<Target["table"]> extends { readonly id: infer Id extends string }
        ? Id
        : IdentifierOf<Target>;

/** Reference an object type's objects in the same scope by identifier. */
function reference<const Target extends ObjectType>(
    target: Target,
    options?: UnqualifiedOptions,
): FieldOf<{ value: ReferenceOf<Target>; reference: true }>;
/** Reference objects of a type declared later, named by its identity. */
function reference<const Identity extends string>(
    identity: Identity,
    target: () => ObjectType,
    options?: UnqualifiedOptions,
): FieldOf<{ value: Identifier<Identity>; reference: true }>;
/** Reference objects in any scope by scope and identifier. */
function reference(
    target: ObjectType | (() => ObjectType) | "self",
    options: QualifiedOptions,
): FieldOf<{ value: QualifiedValue }>;
/** Reference a principal of a kind by its global identifier. */
function reference(target: Policy): FieldOf<{ value: string; reference: true }>;
/**
 * Reference another object, or a principal of a kind.
 *
 * @construct each form returns the field of the builder its signature names, typed by the values it keeps.
 */
function reference(
    target: ObjectType | Policy | (() => ObjectType) | string,
    second?: ReferenceOptions | (() => ObjectType),
    third?: UnqualifiedOptions,
): FieldOf<{ value: string; reference: true }> | FieldOf<{ value: QualifiedValue }> {
    // reference a global principal kind
    if (target instanceof Policy) {
        return referencePrincipal(target, typeof second === "function" ? {} : (second ?? {}));
    }
    // reference a type declared later by its identity
    else if (typeof second === "function") {
        if (typeof target !== "string") {
            throw new TypeError("a reference to a type declared later names its identity first");
        }

        return referenceLater(target, second, third ?? {});
    }

    // keep the scope beside the identifier
    const options = second ?? {};
    if (options.qualified === true) {
        if (typeof target === "string" && target !== "self") {
            throw new TypeError(
                `a qualified reference takes an object type or "self", not ${target}`,
            );
        }

        return referenceQualified(target);
    }
    // refuse an unqualified reference without an object type
    else if (typeof target === "string") {
        throw new TypeError(
            "a reference to its own type must be qualified; use a parent for trees",
        );
    } else if (typeof target === "function") {
        throw new TypeError("a reference to a type declared later names its identity first");
    }

    return referenceObject(target, options);
}

/** Reference a global principal kind by its identifier. */
function referencePrincipal(
    target: Policy,
    options: ReferenceOptions,
): FieldOf<{ value: string; reference: true }> {
    if (target.definition.isGlobal !== true) {
        throw new TypeError(
            `principal kind ${target.name} lives in scopes; keep it with field.subject(principal.${target.name})`,
        );
    } else if (options.qualified !== undefined || options.delete !== undefined) {
        throw new TypeError(
            `a reference to principal kind ${target.name} keeps a global identifier, without a scope or deletion`,
        );
    }

    return new Field<FieldConfigurationOf<{ value: string; reference: true }>>({
        type: "reference",
        principals: [target],
        build: (name) => text(name).validate(schema.string().min(1)),
        ...WRITTEN,
        isReference: true,
    });
}

/** Reference an object type's objects by identifier, or by their natural key. */
function referenceObject(
    target: ObjectType,
    options: ReferenceOptions,
): FieldOf<{ value: string; reference: true }> {
    return new Field<FieldConfigurationOf<{ value: string; reference: true }>>({
        type: "reference",
        target: () => target,
        build: (name, _owner, storage) =>
            keyed(
                target.keyed === undefined
                    ? identifier(name, target.identity)
                    : validated(name, target.keyed),
                () => target,
                storage,
                options,
            ),
        ...WRITTEN,
        isReference: true,
    });
}

/** Reference objects of a type declared later, refusing a type of another identity. */
function referenceLater(
    identity: string,
    target: () => ObjectType,
    options: ReferenceOptions,
): FieldOf<{ value: string; reference: true }> {
    // resolve the type, checking the identity the reference names
    const resolve = (): ObjectType => {
        const resolved = target();
        if (resolved.identity !== identity) {
            throw new TypeError(
                `a reference to ${identity} resolves to object type ${resolved.name} of identity ${resolved.identity}`,
            );
        }

        return resolved;
    };

    return new Field<FieldConfigurationOf<{ value: string; reference: true }>>({
        type: "reference",
        target: resolve,
        build: (name, _owner, storage) =>
            keyed(
                validated(
                    name,
                    schema.lazy(() => resolve().idSchema),
                ),
                resolve,
                storage,
                options,
            ),
        ...WRITTEN,
        isReference: true,
    });
}

/** Reference objects in any scope by scope and identifier, of the field's own type for `"self"`. */
function referenceQualified(
    target: ObjectType | (() => ObjectType) | "self",
): FieldOf<{ value: QualifiedValue }> {
    const resolve =
        target === "self" ? undefined : typeof target === "function" ? target : () => target;

    return new Field<FieldConfigurationOf<{ value: QualifiedValue }>>({
        type: "reference",
        target: resolve,
        qualified: true,
        build: (name, owner) =>
            json(
                name,
                schema.object({
                    /** The scope containing the referenced object. */
                    scope: schema.string().min(1),
                    /** The referenced object's identifier. */
                    id: schema.lazy(() => resolve?.().idSchema ?? schema.identifier(owner)),
                }),
            ),
        ...WRITTEN,
    });
}

/** Read the identifier column of an object type's table, which holds the values its references hold. */
export function keyColumn<Value extends ColumnValue>(table: Table): Column<ColumnDefinition<Value>>;
/**
 * Read the identifier column of an object type's table.
 *
 * @construct every object type keys its rows by its identifiers in an `id` column, and a reference resolves only to a type of the identity its values name.
 */
export function keyColumn(table: Table): Column {
    return table[TABLE].column("id");
}

/** Key an identifier column to its type's table, without a foreign key in ephemeral storage. */
function keyed<Definition extends ColumnDefinition>(
    column: ColumnBuilder<Definition>,
    target: () => ObjectType,
    storage: ObjectStorage,
    options: ReferenceOptions,
): ColumnBuilder<Definition> {
    // keep ephemeral references without a foreign key
    if (storage === "ephemeral") {
        return column;
    }

    return column.references(() => keyColumn(target().table), {
        onDelete: options.delete === "null" ? "set null" : (options.delete ?? "restrict"),
    });
}

/** Declare text a string schema validates, typed by the schema's output, any string without one. */
function string<Validator extends schema.Schema<string> = schema.Schema<string>>(
    ...validator: [Validator] | (schema.Schema<string> extends Validator ? [] : never)
): FieldOf<{ value: schema.Output<Validator> }>;
/**
 * Declare text a string schema validates.
 *
 * @construct a string field omits its schema only when its validator type admits any string, which the signature's rest tuple requires.
 */
function string(validator: schema.Schema<string> = schema.string()): FieldOf<{ value: string }> {
    return required("string", (name) => validated(name, validator));
}

/** Declare a field following a state machine, keeping the machine it follows. */
function state<const Machine extends StateMachine>(machine: Machine): StateField<Machine>;
/**
 * Declare a state field following a machine.
 *
 * @construct the field keeps the machine it follows, which StateField keeps in its type.
 */
function state(machine: StateMachine): FieldOf<{ value: string; default: true; written: false }> {
    // list every state the machine names, the initial one first
    const others = new Set<string>();
    for (const transition of Object.values(machine.transitions)) {
        transition.from.forEach((name) => others.add(name));
        others.add(transition.to);
    }
    others.delete(machine.initial);
    const names: [string, ...string[]] = [machine.initial, ...others];

    return new Field<FieldConfigurationOf<{ value: string; default: true; written: false }>>({
        type: "state",
        initial: { value: machine.initial },
        machine,
        build: (name) => text(name, { enum: names }),
        ...WRITTEN,
        isDefault: true,
        written: false,
    });
}

/** Declare a text kept in chunks and changed by edits, empty at first. */
function textField(): TextField;
/**
 * Declare a text field kept in chunks.
 *
 * @construct the field is a text field kept in chunks, which TextField marks in its type.
 */
function textField(): FieldOf<{ value: string; default: true; written: false }> {
    return new Field<FieldConfigurationOf<{ value: string; default: true; written: false }>>({
        type: "text",
        initial: { value: "" },
        build: () => {
            throw new TypeError("a text field keeps its characters in chunks, not a column");
        },
        ...WRITTEN,
        isDefault: true,
        written: false,
    });
}

/** Declare the fields of objects. */
export const field = {
    /** Text a string schema validates. */
    string,

    /** A whole number within the exactly representable range. */
    integer() {
        return required("integer", integer);
    },

    /** A double-precision number. */
    number() {
        return required("number", real);
    },

    /** The count of objects belonging to each object. */
    count(): FieldOf<{ value: number; default: true; written: false }> {
        return aggregated("count", "integer", integer, 0);
    },

    /** The sum of a field over each object's belonging objects. */
    sum(): FieldOf<{ value: number; default: true; written: false }> {
        return aggregated("sum", "number", real, 0);
    },

    /** The smallest value of a field over each object's belonging objects. */
    min(): FieldOf<{ value: number; required: false; written: false }> {
        return extreme("min", "number", real);
    },

    /** The largest value of a field over each object's belonging objects. */
    max(): FieldOf<{ value: number; required: false; written: false }> {
        return extreme("max", "number", real);
    },

    /** True or false. */
    boolean() {
        return required("boolean", boolean);
    },

    /** An instant in UTC epoch milliseconds. */
    time() {
        return required("time", integer);
    },

    /** One of a fixed set of strings. */
    enum<const Values extends readonly [string, ...string[]]>(values: Values) {
        return required("enum", (name) => text(name, { enum: values }));
    },

    /** A state machine changed by its transition methods. */
    state,

    /** A structured value validated by a schema. */
    json<Value extends JsonValue>(validator: schema.Schema<Value>) {
        return required("json", (name) => json(name, validator));
    },

    /** A file kept in a bucket the package declares, by its key. */
    file(bucket: FileBucket) {
        return new Field<FieldConfigurationOf<{ value: ObjectFile }>>({
            type: "file",
            build: (name) => json(name, ObjectFile),
            bucket,
            ...WRITTEN,
        });
    },

    /** A principal of the given kinds, stored as its subject key. */
    subject(...kinds: readonly Policy[]): FieldOf<{ value: string }> {
        // validate key prefixes
        const prefixes = kinds.map((kind) =>
            JSON.stringify([kind.definition.packageId, kind.name]).slice(0, -1),
        );
        const key =
            prefixes.length === 0
                ? schema.string().min(1)
                : schema
                      .string()
                      .regex(new RegExp(`^(${prefixes.map(literally).join("|")}),`, "u"));

        return new Field<FieldConfigurationOf<{ value: string }>>({
            type: "subject",
            ...(kinds.length === 0 ? {} : { principals: kinds }),
            build: (name) => text(name).validate(key),
            ...WRITTEN,
        });
    },

    /** Another object by identifier, or a principal by its global identifier. */
    reference,

    /** A fractional index ordering objects among their siblings. */
    fractionalIndex() {
        return required("fractionalIndex", (name) => text(name).validate(FractionalIndex));
    },

    /** A text kept in chunks and changed by edits, empty at first. */
    text: textField,
};

/** Match a text literally within a regular expression. */
function literally(literal: string): string {
    return literal.replace(/[.*+?^${}()|[\]\\]/gu, "\\$&");
}
