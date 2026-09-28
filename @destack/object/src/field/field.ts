import {
    boolean,
    type Column,
    ColumnBuilder,
    type Select,
    TABLE,
    identifier,
    integer,
    json,
    real,
    text,
} from "@destack/db";
import { schema, type Identifier } from "@destack/schema";
import * as validation from "@destack/schema";
import type { ObjectStorage, ObjectType } from "../object/object.ts";
import { Policy } from "@destack/access";

/** The digits of minted positions in ascending order, sorting alike in every collation. */
const POSITION_DIGITS = "0123456789abcdefghijklmnopqrstuvwxyz";

/** The shape of a position. */
const POSITION = /^[0-9a-z]+$/;

/** The meaning of a field beyond its stored value. */
export type FieldType =
    | "string"
    | "integer"
    | "number"
    | "boolean"
    | "time"
    | "enum"
    | "json"
    | "reference"
    | "subject"
    | "position"
    | "state"
    | "text";

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
export type StateField<Machine extends StateMachine = StateMachine> = Field<
    StateOf<Machine>,
    true,
    true,
    false,
    false,
    false
> & { readonly machine: Machine };

/** A text field, held in chunks and changed only by edits. */
export type TextField = Field<string, true, true, false, false, false> & {
    readonly type: "text";
};

/** The aggregate function keeping a field current. */
export type AggregateFunction = "count" | "sum" | "min" | "max";

/** A field of an object. */
export class Field<
    Value = unknown,
    Required extends boolean = boolean,
    Default extends boolean = boolean,
    Guarded extends boolean = boolean,
    Written extends boolean = boolean,
    Sensitive extends boolean = boolean,
> {
    /** The field's meaning. */
    readonly type: FieldType;
    /** The referenced object type, for references to objects. */
    readonly target?: () => ObjectType;
    /** The principal kinds a principal reference or subject field holds. */
    readonly principals?: readonly Policy[];
    /** Whether a reference holds the scope beside the identifier. */
    readonly qualified: boolean;
    /** Whether every object must hold a value. */
    readonly required: Required;
    /** The value creation supplies when the caller omits the field. */
    readonly initial?: { readonly value: Value };
    /** How the value is protected. */
    readonly classification?: "sensitive" | "personal";
    /** The permissions reading or writing the value requires. */
    readonly access?: FieldAccess;
    /** Whether creation fills the value with the calling principal. */
    readonly isCaller: boolean;
    /** The aggregate function keeping the value current. */
    readonly aggregate?: AggregateFunction;
    /** The state machine the value follows. */
    readonly machine?: StateMachine;
    /** Whether reading the value requires its own permission. */
    declare readonly guarded: Guarded;
    /** Whether callers write the value. */
    declare readonly written: Written;
    /** Whether the value is sensitive. */
    declare readonly isSensitive: Sensitive;
    /** Create the nullable column storing the field. */
    readonly #build: (name: string, owner: string, storage: ObjectStorage) => ColumnBuilder<Value>;

    /** Retain the field's meaning and storage. */
    constructor(definition: {
        readonly type: FieldType;
        readonly build: (
            name: string,
            owner: string,
            storage: ObjectStorage,
        ) => ColumnBuilder<Value>;
        readonly required: Required;
        readonly target?: () => ObjectType;
        readonly principals?: readonly Policy[];
        readonly qualified?: boolean;
        readonly initial?: { readonly value: Value };
        readonly classification?: "sensitive" | "personal";
        readonly access?: FieldAccess;
        readonly isCaller?: boolean;
        readonly aggregate?: AggregateFunction;
        readonly machine?: StateMachine;
    }) {
        // retain the declaration and its column factory
        this.type = definition.type;
        this.target = definition.target;
        this.principals = definition.principals;
        this.qualified = definition.qualified ?? false;
        this.required = definition.required;
        this.initial = definition.initial;
        this.classification = definition.classification;
        this.access = definition.access;
        this.isCaller = definition.isCaller ?? false;
        this.aggregate = definition.aggregate;
        this.machine = definition.machine;
        this.#build = definition.build;
    }

    /** Create the column storing the field. */
    column(
        name: string,
        owner: string,
        storage: ObjectStorage,
    ): ColumnBuilder<Value, Required, Default> {
        // build the nullable column
        let column: ColumnBuilder<Value, boolean, boolean> = this.#build(name, owner, storage);

        // mark sensitive values in their schemas
        if (this.classification === "sensitive") {
            const { definition } = column.sensitive();
            column = new ColumnBuilder({
                ...definition,
                schema: schema.sensitive(definition.schema.clone()),
                ...(definition.json === undefined
                    ? {}
                    : { json: schema.sensitive(definition.json.clone()) }),
            });
        }
        // classify personal values
        else if (this.classification === "personal") {
            column = column.personal();
        }

        // supply the default
        if (this.initial) {
            column = column.default(this.initial.value);
        }

        // require a value
        if (this.required) {
            column = column.notNull();
        }

        return column as ColumnBuilder<Value, Required, Default>;
    }

    /** Allow the field to be absent. */
    optional(): Field<Value, false, Default, Guarded, Written, Sensitive> {
        return this.#with({ required: false });
    }

    /** Supply a value when creation omits the field. */
    default(value: Value): Field<Value, Required, true, Guarded, Written, Sensitive> {
        return this.#with({ initial: { value } });
    }

    /** Keep the value out of logs, audit details, sync and request fingerprints. */
    sensitive(): Field<Value, Required, Default, Guarded, Written, true> {
        return this.#with<Required, Default, Guarded, Written, true>({
            classification: "sensitive",
        });
    }

    /** Mark the value as personal data, exported and erased with its subject. */
    personal(): Field<Value, Required, Default, Guarded, Written, Sensitive> {
        return this.#with({ classification: "personal" });
    }

    /** Fill the field with the principal creating the object. */
    caller(): Field<Value, Required, true, Guarded, false, Sensitive> {
        if (this.principals === undefined && this.type !== "subject") {
            throw new TypeError(
                "only principal references and subject fields hold the calling principal",
            );
        }

        return this.#with<Required, true, Guarded, false, Sensitive>({ isCaller: true });
    }

    /** Require extra permissions to read or write the value. */
    guard<const Access extends FieldAccess>(
        access: Access,
    ): Field<
        Value,
        Required,
        Default,
        Access extends { readonly read: string } ? true : Guarded,
        Written,
        Sensitive
    > {
        return this.#with({ access: { ...this.access, ...access } });
    }

    /** Derive a field with changed properties. */
    #with<
        NextRequired extends boolean = Required,
        NextDefault extends boolean = Default,
        NextGuarded extends boolean = Guarded,
        NextWritten extends boolean = Written,
        NextSensitive extends boolean = Sensitive,
    >(change: {
        readonly required?: NextRequired;
        readonly initial?: { readonly value: Value };
        readonly classification?: "sensitive" | "personal";
        readonly access?: FieldAccess;
        readonly isCaller?: boolean;
    }): Field<Value, NextRequired, NextDefault, NextGuarded, NextWritten, NextSensitive> {
        return new Field<Value, NextRequired, NextDefault, NextGuarded, NextWritten, NextSensitive>(
            {
                type: this.type,
                build: this.#build,
                target: this.target,
                ...(this.principals === undefined ? {} : { principals: this.principals }),
                qualified: this.qualified,
                required: (change.required ?? this.required) as NextRequired,
                initial: change.initial ?? this.initial,
                classification: change.classification ?? this.classification,
                access: change.access ?? this.access,
                ...(this.aggregate === undefined ? {} : { aggregate: this.aggregate }),
                ...(this.machine === undefined ? {} : { machine: this.machine }),
                isCaller: change.isCaller ?? this.isCaller,
            },
        );
    }
}

/** A qualified reference's scope and identifier. */
type QualifiedValue = { readonly scope: string; readonly id: string };

/** The value a reference holds. */
export type ReferenceValue = string | QualifiedValue;

/** Build a required field of one type from a nullable column. */
function required<Value>(
    type: FieldType,
    build: (name: string, owner: string, storage: ObjectStorage) => ColumnBuilder<Value>,
    target?: () => ObjectType,
): Field<Value, true, false, false, true, false> {
    return new Field({ type, build, required: true, target });
}

/** Declare a field an aggregate keeps, starting at a value or absent. */
function aggregated<Initial extends number | undefined>(
    aggregate: AggregateFunction,
    type: FieldType,
    build: (name: string) => ColumnBuilder<number>,
    initial?: Initial,
): Field<
    number,
    Initial extends number ? true : false,
    Initial extends number ? true : false,
    false,
    false,
    false
> {
    return new Field({
        type,
        build,
        required: (initial !== undefined) as Initial extends number ? true : false,
        ...(initial === undefined ? {} : { initial: { value: initial } }),
        aggregate,
    });
}

/** The options of a reference field. */
interface ReferenceOptions {
    /** Hold the scope beside the identifier. */
    readonly qualified?: boolean;
    /** What deleting the referenced object does to this one. */
    readonly delete?: "restrict" | "cascade" | "null";
}

/** The identifier of an object type's objects. */
export type IdentifierOf<Target extends ObjectType> =
    Select<Target["table"]> extends { readonly id: infer Id } ? Id : string;

/** Reference an object type's objects in the same scope by identifier. */
function reference<const Target extends ObjectType>(
    target: Target,
    options?: ReferenceOptions & { readonly qualified?: false },
): Field<IdentifierOf<Target>, true, false, false, true, false>;
/** Reference objects of a type declared later. */
function reference<const Identity extends string = string>(
    target: () => ObjectType,
    options?: ReferenceOptions & { readonly qualified?: false },
): Field<Identifier<Identity>, true, false, false, true, false>;
/** Reference objects in any scope by scope and identifier. */
function reference(
    target: ObjectType | (() => ObjectType) | "self",
    options: ReferenceOptions & { readonly qualified: true },
): Field<QualifiedValue, true, false, false, true, false>;
/** Reference a principal of a kind by its global identifier. */
function reference(target: Policy): Field<string, true, false, false, true, false>;
/** Reference another object, or a principal of a kind. */
function reference(
    target: ObjectType | Policy | (() => ObjectType) | "self",
    options: ReferenceOptions = {},
): Field<ReferenceValue, true, false, false, true, false> {
    // reference a global principal kind
    if (target instanceof Policy) {
        if (target.definition.isGlobal !== true) {
            throw new TypeError(
                `principal kind ${target.name} lives in scopes; hold it with field.subject(principal.${target.name})`,
            );
        } else if (options.qualified !== undefined || options.delete !== undefined) {
            throw new TypeError(
                `a reference to principal kind ${target.name} holds a global identifier, without a scope or deletion`,
            );
        }

        return new Field<ReferenceValue, true, false, false, true, false>({
            type: "reference",
            required: true,
            principals: [target],
            build: (name) =>
                text(name).validate(
                    schema.string().min(1),
                ) as unknown as ColumnBuilder<ReferenceValue>,
        });
    }

    // resolve the target lazily
    if (target === "self" && !options.qualified) {
        throw new TypeError(
            "a reference to its own type must be qualified; use a parent for trees",
        );
    }
    const resolve =
        target === "self" ? undefined : typeof target === "function" ? target : () => target;

    // build the column
    return new Field<ReferenceValue, true, false, false, true, false>({
        type: "reference",
        required: true,
        ...(resolve === undefined ? {} : { target: resolve }),
        qualified: options.qualified ?? false,
        build: (name, owner, storage) => {
            // store a qualified reference as JSON
            if (options.qualified) {
                return json(
                    name,
                    schema.object({
                        /** The scope containing the referenced object. */
                        scope: schema.string().min(1),
                        /** The referenced object's identifier. */
                        id: schema.lazy(() => validation.identifier(resolve?.().identity ?? owner)),
                    }),
                ) as unknown as ColumnBuilder<ReferenceValue>;
            }
            // store an ephemeral reference without a foreign key
            else if (storage === "ephemeral") {
                return identifier(
                    name,
                    () => resolve!().identity,
                ) as unknown as ColumnBuilder<ReferenceValue>;
            }
            // store a foreign key
            else {
                return identifier(name, () => resolve!().identity).references(
                    () => resolve!().table[TABLE].columns.id as Column<Identifier<string>>,
                    {
                        onDelete:
                            options.delete === "null" ? "set null" : (options.delete ?? "restrict"),
                    },
                ) as unknown as ColumnBuilder<ReferenceValue>;
            }
        },
    });
}

/** Declare the fields of objects. */
export const field = {
    /** Text a string schema validates. */
    string<Validator extends schema.Schema<string> = schema.Schema<string>>(
        validator: Validator = schema.string() as unknown as Validator,
    ) {
        return required<schema.Infer<Validator>>(
            "string",
            (name) =>
                text(name).validate(validator) as unknown as ColumnBuilder<schema.Infer<Validator>>,
        );
    },

    /** A whole number within the exactly representable range. */
    integer() {
        return required("integer", integer);
    },

    /** A double-precision number. */
    number() {
        return required("number", real);
    },

    /** The count of objects belonging to each object. */
    count(): Field<number, true, true, false, false, false> {
        return aggregated("count", "integer", integer, 0);
    },

    /** The sum of a field over each object's belonging objects. */
    sum(): Field<number, true, true, false, false, false> {
        return aggregated("sum", "number", real, 0);
    },

    /** The smallest value of a field over each object's belonging objects. */
    min(): Field<number, false, false, false, false, false> {
        return aggregated<undefined>("min", "number", real);
    },

    /** The largest value of a field over each object's belonging objects. */
    max(): Field<number, false, false, false, false, false> {
        return aggregated<undefined>("max", "number", real);
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
    state<const Machine extends StateMachine>(machine: Machine): StateField<Machine> {
        // list every state the machine names
        const names = new Set([machine.initial]);
        for (const transition of Object.values(machine.transitions)) {
            transition.from.forEach((state) => names.add(state));
            names.add(transition.to);
        }

        return new Field<StateOf<Machine>, true, true, false, false, false>({
            type: "state",
            required: true,
            initial: { value: machine.initial as StateOf<Machine> },
            machine,
            build: (name) =>
                text(name, {
                    enum: [...names] as [string, ...string[]],
                }) as unknown as ColumnBuilder<StateOf<Machine>>,
        }) as StateField<Machine>;
    },

    /** Structured data validated by a schema. */
    json<Validator extends schema.Schema>(validator: Validator) {
        return required("json", (name) => json(name, validator));
    },

    /** A principal of the given kinds, stored as its subject key. */
    subject(...kinds: readonly Policy[]) {
        // validate key prefixes
        const prefixes = kinds.map((kind) =>
            JSON.stringify([kind.definition.packageId, kind.name]).slice(0, -1),
        );
        const key =
            prefixes.length === 0
                ? schema.string().min(1)
                : schema.string().regex(new RegExp(`^(${prefixes.map(literally).join("|")}),`));

        return new Field({
            type: "subject",
            required: true,
            ...(kinds.length === 0 ? {} : { principals: kinds }),
            build: (name) => text(name).validate(key),
        });
    },

    /** Another object by identifier, or a principal by its global identifier. */
    reference,

    /** A fractional index ordering objects among their siblings. */
    position() {
        return required("position", (name) => text(name).validate(schema.string().regex(POSITION)));
    },

    /** A text held in chunks and changed by edits, empty at first. */
    text(): TextField {
        return new Field<string, true, true, false, false, false>({
            type: "text",
            required: true,
            initial: { value: "" },
            build: () => {
                throw new TypeError("a text field holds its characters in chunks, not a column");
            },
        }) as TextField;
    },
};

/** Fractional indexes ordering siblings. */
export const Position = {
    /** Mint a position between two others, either end open when absent. */
    between(before: string | undefined, after: string | undefined): string {
        // walk both positions digit by digit
        let minted = "";
        let upper = after;
        for (let index = 0; ; index++) {
            // read the digits at the index, the ends open past them
            const low = before !== undefined && index < before.length ? digit(before[index]!) : 0;
            const high =
                upper !== undefined && index < upper.length
                    ? digit(upper[index]!)
                    : POSITION_DIGITS.length;

            // take the middle digit once there is room
            if (high - low > 1) {
                return minted + POSITION_DIGITS[Math.floor((low + high) / 2)]!;
            }

            // keep the lower digit and drop an upper bound left behind
            minted += POSITION_DIGITS[low]!;
            if (high > low) {
                upper = undefined;
            }
        }
    },
};

/** Read the value of a position digit. */
function digit(character: string): number {
    return POSITION_DIGITS.indexOf(character);
}

/** Match a text literally within a regular expression. */
function literally(text: string): string {
    return text.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}
