import type { Now, Select } from "@destack/db";
import type { Message } from "@destack/locale";
import { ModuleMetadata, type PackageVersion } from "@destack/package";
import type { Identifier, JsonObject, schema } from "@destack/schema";
import type { ObjectReference, ObjectTypeReference } from "@destack/sync";
import type { Field } from "../field/field.ts";
import type { CallInput, MethodName } from "../method/procedure.ts";
import type { ObjectType } from "../object/object.ts";
import type { ConditionOf } from "../query/index.ts";

/** The name of the variable that reads the person an action runs for. */
export const ME = "me";

/** The name of the variable that reads the space an action runs in. */
export const SPACE = "space";

/** The value a variable reads, as a type only. */
declare const VALUE: unique symbol;

/** A value of an action's call, read by name when the action runs. */
export class Variable<Value = unknown> {
    /** The value read, as a type only. */
    declare readonly [VALUE]?: Value;
    /** The name: an input's, a field of an input's object as `issue.title`, or the person's. */
    readonly #name: string;

    /** Create the variable. */
    constructor(name: string) {
        this.#name = name;
    }

    /** Read a variable's name, apart from the fields of its object. */
    static nameOf(variable: Variable): string {
        return variable.#name;
    }
}

/** The person an action runs for, such as `assignee: me` to assign an object to oneself. */
export const me: Variable<ObjectReference> & { readonly id: Variable<Identifier<"user">> } =
    Object.assign(new Variable<ObjectReference>(ME), {
        id: new Variable<Identifier<"user">>(`${ME}.id`),
    });

/** The space an action runs in, such as `owner: space` to give it to the space. */
export const space = new Variable<string>(SPACE);

/** The names of the object references an input spreads into columns. */
export type SubjectField<Input> = string extends keyof Input
    ? never
    : {
          [Name in keyof Input]: Name extends `${infer Property}PackageId` ? Property : never;
      }[keyof Input];

/** An input whose values variables may stand for, a person by their field's name. */
export type VariableInput<Input> = {
    readonly [Name in keyof Input]:
        | Input[Name]
        | Variable<Input[Name]>
        | Variable<NonNullable<Input[Name]>>
        | (number extends Input[Name] ? Now : never);
} & {
    readonly [Name in SubjectField<Input>]?: ObjectReference | Variable<ObjectReference>;
};

/** The call an action makes, its values variables where it reads them. */
export interface ActionTemplate {
    /** The object type it calls. */
    readonly object: ObjectType;
    /** The method it calls. */
    readonly method: string;
    /** The input, its values variables where it reads them. */
    readonly input: Readonly<Record<string, unknown>>;
    /** The name of the input with the objects it calls, absent for a call on a scope. */
    readonly target?: string;
}

/** The fields of an input's object that a call reads, a person's by their field's name. */
export type ObjectFields<Object extends ObjectType> = {
    readonly [Column in keyof Select<Object["table"]> & string]: Variable<
        Select<Object["table"]>[Column]
    >;
} & {
    readonly [
        Column in keyof Select<Object["table"]> &
            string as Column extends `${infer Property}PackageId` ? Property : never
    ]: Variable<ObjectReference>;
};

/** An input's object, whose fields a call reads and whose methods it calls. */
export type ObjectVariable<Object extends ObjectType> = Variable<ObjectReference> &
    ObjectFields<Object> & {
        /** Prepare a call of a method on the object, or on each of several. */
        readonly prepare: <const Name extends MethodName<Object>>(
            name: Name,
            input: VariableInput<Omit<CallInput<Object, Name>, "id">>,
        ) => ActionTemplate;
    };

/** Where an input's objects come from: those selected, the nearest one, or one a person picks. */
export type ActionSource = "selected" | "current" | "picked";

/** A value places give, by its key: a declared one, or objects of a type from a source. */
export interface PlaceVariable<Value = unknown> {
    /** The key, unique among the variables places give. */
    readonly key: string;
    /** The value places give, as a type only. */
    readonly [VALUE]?: Value;
}

/** An input that takes objects of a type, asking for one where its source has none. */
export class ObjectInput<
    Object extends ObjectType = ObjectType,
    Source extends ActionSource = ActionSource,
    Optional extends boolean = boolean,
> implements PlaceVariable<
    Source extends "selected" ? readonly ObjectReference[] : ObjectReference
> {
    /** The value places give, as a type only. */
    declare readonly [VALUE]?: Source extends "selected"
        ? readonly ObjectReference[]
        : ObjectReference;
    /** The type of the objects. */
    readonly object: Object;
    /** Where they come from. */
    readonly source: Source;
    /** The condition the objects meet for the action to apply, absent for any. */
    readonly where: ConditionOf<Object, Object> | undefined;
    /** Whether the action runs without one. */
    readonly isOptional: Optional;

    /** Create the input. */
    constructor(
        definition: Pick<
            ObjectInput<Object, Source, Optional>,
            "object" | "source" | "where" | "isOptional"
        >,
    ) {
        // take the type, the source and what they allow
        this.object = definition.object;
        this.source = definition.source;
        this.where = definition.where;
        this.isOptional = definition.isOptional;
    }

    /** The key of the objects a place gives for the type and the source. */
    get key(): string {
        return ObjectInput.keyOf(this.source, this.object.typeReference);
    }

    /** Key the objects of a type from a source. */
    static keyOf(source: ActionSource, { packageId, type }: ObjectTypeReference): string {
        return `${source}:${packageId}/${type}`;
    }

    /** Let the action run without one. */
    optional(): ObjectInput<Object, Source, true> {
        const { object, source, where } = this;

        return new ObjectInput({ object, source, where, isOptional: true });
    }
}

/** Take the selected objects of a type, else the highlighted one, where each meets a condition. */
export function selected<Object extends ObjectType>(
    object: Object,
    where?: ConditionOf<Object, Object>,
): ObjectInput<Object, "selected", false> {
    return new ObjectInput({ object, source: "selected", where, isOptional: false });
}

/** Take the nearest object of a type around the person, where it meets a condition. */
export function current<Object extends ObjectType>(
    object: Object,
    where?: ConditionOf<Object, Object>,
): ObjectInput<Object, "current", false> {
    return new ObjectInput({ object, source: "current", where, isOptional: false });
}

/** Ask for an object of a type, among those meeting a condition. */
export function picked<Object extends ObjectType>(
    object: Object,
    where?: ConditionOf<Object, Object>,
): ObjectInput<Object, "picked", false> {
    return new ObjectInput({ object, source: "picked", where, isOptional: false });
}

/** What an object of a type created at a place starts with, such as a column's status. */
export function creating<Object extends ObjectType>(
    object: Object,
): PlaceVariable<Readonly<Partial<Select<Object["table"]>>>> {
    return { key: `creating:${object.package.id}/${object.name}` };
}

/** A value some places give, such as the dates a calendar shows, which actions take as an input. */
export class ContextVariable<Type extends schema.Schema = schema.Schema> implements PlaceVariable<
    schema.Output<Type>
> {
    /** The value places give, as a type only. */
    declare readonly [VALUE]?: schema.Output<Type>;
    /** The declaring package. */
    readonly package: PackageVersion;
    /** The name, unique among the package's variables. */
    readonly name: string;
    /** The value's name as people read it, absent for one nobody is asked for. */
    readonly label: Message | undefined;
    /** The value's schema. */
    readonly value: Type;

    /** Create a variable of a package. */
    constructor(owner: PackageVersion, definition: VariableDefinition<Type>) {
        // take the package and the definition
        this.package = owner;
        this.name = definition.name;
        this.label = definition.label;
        this.value = definition.value;
    }

    /** Build the variable a call reads the value by, with the fields of an object's. */
    variable(name: string): Variable {
        // read each field of an object's schema
        const shape: unknown = "shape" in this.value ? this.value.shape : undefined;
        const fields =
            typeof shape === "object" && shape !== null
                ? Object.keys(shape).map((field): [string, Variable] => [
                      field,
                      new Variable(`${name}.${field}`),
                  ])
                : [];

        return Object.assign(new Variable(name), Object.fromEntries<Variable>(fields));
    }

    /** The variable's key: its package and name. */
    get key(): string {
        return `${this.package.id}/${this.name}`;
    }
}

/** A variable as its package defines it. */
export interface VariableDefinition<Type extends schema.Schema = schema.Schema> {
    /** The name, unique among the package's variables. */
    readonly name: string;
    /** The value's name as people read it, absent for one nobody is asked for. */
    readonly label?: Message;
    /** The value's schema. */
    readonly value: Type;
}

/** Declare a value some places give, which actions take as an input. */
export function defineVariable<const Type extends schema.Schema>(
    definition: VariableDefinition<Type>,
    module?: ModuleMetadata,
): ContextVariable<Type> {
    const owner = ModuleMetadata.require(module, "defineVariable").package;

    return new ContextVariable(owner, definition);
}

/** An input of an action: objects, a variable places give, or a field or schema a person gives. */
export type ActionInputDefinition = ObjectInput | ContextVariable | Field | schema.Schema;

/** The inputs of an action, by name. */
export type ActionInputs = Readonly<Record<string, ActionInputDefinition>>;

/** The fields of a value that a call reads, none for one without fields. */
type ValueFields<Value> =
    Value extends Readonly<Record<string, unknown>>
        ? { readonly [Name in keyof Value & string]: Variable<Value[Name]> }
        : unknown;

/** The variable of an input, by what the input takes. */
type VariableOf<Definition> =
    Definition extends ObjectInput<infer Object>
        ? ObjectVariable<Object>
        : Definition extends ContextVariable<infer Type>
          ? Variable<schema.Output<Type>> & ValueFields<schema.Output<Type>>
          : Definition extends Field<infer Configuration>
            ? Variable<Configuration["value"]>
            : Definition extends schema.Schema
              ? Variable<schema.Output<Definition>>
              : never;

/** The variables of an action's inputs, which its call reads. */
export type VariablesOf<Inputs extends ActionInputs> = {
    readonly [Name in keyof Inputs]: VariableOf<Inputs[Name]>;
};

/** The values of an action's inputs, by name, an object's by its reference. */
export type ActionValues = JsonObject;

/** The value of an input, by what the input takes. */
type ValueOf<Definition> =
    Definition extends ObjectInput<ObjectType, infer Source>
        ? "selected" extends Source
            ? ObjectReference | readonly ObjectReference[]
            : ObjectReference
        : Definition extends ContextVariable<infer Type>
          ? schema.Output<Type>
          : Definition extends Field<infer Configuration>
            ? Configuration["reference"] extends "none"
                ? Configuration["value"]
                : ObjectReference
            : Definition extends schema.Schema
              ? schema.Output<Definition>
              : never;

/** Whether an input may be left out: an optional or defaulted one. */
type IsOptional<Definition> =
    Definition extends ObjectInput<ObjectType, ActionSource, infer Optional>
        ? Optional
        : Definition extends ContextVariable
          ? false
          : Definition extends Field<infer Configuration>
            ? Configuration extends { readonly required: true; readonly default: false }
                ? false
                : true
            : Definition extends schema.Schema
              ? undefined extends schema.Input<Definition>
                  ? true
                  : false
              : true;

/** The values of an action's inputs, by name, the optional ones left out as they may be. */
export type ValuesOf<Inputs extends ActionInputs> = string extends keyof Inputs
    ? ActionValues
    : {
          readonly [
              Name in keyof Inputs as IsOptional<Inputs[Name]> extends false ? Name : never
          ]: ValueOf<Inputs[Name]>;
      } & {
          readonly [
              Name in keyof Inputs as IsOptional<Inputs[Name]> extends false ? never : Name
          ]?: ValueOf<Inputs[Name]>;
      };
