import { IconName } from "@destack/icon";
import { NumberInput } from "./number.ts";
import { Condition, type JsonCondition } from "@destack/db";
import { Message } from "@destack/locale";
import {
    fromJsonSchema,
    type JsonObject,
    JsonValue,
    present,
    schema,
    toJsonSchema,
} from "@destack/schema";
import { ObjectReference, ObjectTypeReference } from "@destack/sync";
import type { ObjectType } from "../object/object.ts";
import { FieldChoice } from "../field/choice.ts";
import { Place } from "./place.ts";
import {
    type ActionInputDefinition,
    type ActionSource,
    ContextVariable,
    ObjectInput,
} from "./variable.ts";

/** The kinds of value a person gives, objects and choices aside. */
export const VALUE_KINDS = ["text", "boolean", "time", "person"] as const;

/** The type of the JSON value of each kind a person gives. */
const VALUE_TYPES = {
    text: "string",
    boolean: "boolean",
    time: "number",
} as const;

/** What a schema that wraps another says of a value's presence, by its type. */
const PRESENCES: Readonly<Partial<Record<string, InputPresence>>> = {
    optional: "optional",
    default: "default",
    prefault: "default",
};

/** The kinds of value people give for a schema, by its type. */
const SCHEMA_KINDS: Readonly<Partial<Record<string, "text" | "boolean">>> = {
    string: "text",
    boolean: "boolean",
};

/** A schema by its type, with the options of an enum and the schema it wraps. */
const SCHEMA = schema.looseObject({
    type: schema.string(),
    options: schema.array(schema.string()).exactOptional(),
    def: schema.looseObject({ innerType: schema.unknown().exactOptional() }),
});

/** A property of a method's input as JSON Schema: its type and its choices. */
const PROPERTY = schema.looseObject({
    type: schema.unknown().exactOptional(),
    enum: schema.array(schema.string()).exactOptional(),
});

/** Whether a person must give an input, may, or leaves it to its default. */
export type InputPresence = "required" | "optional" | "default";

/** What an input takes: a value, one of some choices, or objects from a source. */
export type ActionValue =
    | { readonly kind: (typeof VALUE_KINDS)[number] }
    | (NumberInput & { readonly kind: "number" })
    | {
          readonly kind: "json";
          /** The JSON Schema the value meets, whose name, such as a color's, picks its input. */
          readonly schema: JsonObject;
      }
    | { readonly kind: "choice"; readonly choices: readonly FieldChoice[] }
    | {
          readonly kind: "object";
          /** The type of the objects. */
          readonly type: ObjectType;
          /** Where they come from. */
          readonly source: ActionSource;
          /** The condition they meet for the action to apply, absent for any. */
          readonly where: JsonCondition | undefined;
      };

/** One input of an action, as people and programs give it: its name, its label, what it takes. */
export class ActionInput {
    /** The input's name. */
    readonly name: string;
    /** The input's name as people read it, absent to read it from its name. */
    readonly label: Message | undefined;
    /** Whether the person must give it, may, or leaves it to its default. */
    readonly presence: InputPresence;
    /** What it takes. */
    readonly value: ActionValue;
    /** The key of the variable places give it by, absent for any other input. */
    readonly variable: string | undefined;
    /** The icon beside its name, absent for none. */
    readonly icon: IconName | undefined;
    /** One sentence on what it takes and when to change it, absent for none. */
    readonly description: Message | undefined;

    /** Create an input. */
    constructor(
        definition: Pick<ActionInput, "name" | "label" | "presence" | "value"> &
            Partial<Pick<ActionInput, "icon" | "description">> &
            Partial<Pick<ActionInput, "variable">>,
    ) {
        // take the name, the label, the presence and the value
        this.name = definition.name;
        this.label = definition.label;
        this.presence = definition.presence;
        this.value = definition.value;
        this.variable = definition.variable;
        this.icon = definition.icon;
        this.description = definition.description;
    }

    /** Read an input as its definition declares it. */
    static of(name: string, definition: ActionInputDefinition): ActionInput {
        // take objects from where the person is, what a field takes, or what a schema does
        if (definition instanceof ObjectInput) {
            const { object: type, source, where, isOptional } = definition;

            return new ActionInput({
                name,
                label: undefined,
                presence: isOptional ? "default" : "required",
                value: {
                    kind: "object",
                    type,
                    source,
                    where: where === undefined ? undefined : type.condition(where),
                },
            });
        } else if (definition instanceof ContextVariable) {
            const { label, key: variable, isOptional } = definition;
            const { presence, value } = ActionInput.of(name, definition.value);

            return new ActionInput({
                name,
                label,
                presence: isOptional ? "default" : presence,
                value,
                variable,
            });
        } else if ("actionInput" in definition) {
            return definition.actionInput(name);
        }

        // read an optional or defaulted schema as what it wraps
        const outer = SCHEMA.parse(definition);
        const presence = PRESENCES[outer.type] ?? "required";
        const inner = presence === "required" ? outer : SCHEMA.parse(outer.def.innerType);
        const value: ActionValue =
            inner.type === "enum"
                ? {
                      kind: "choice",
                      choices: (inner.options ?? []).map((each) => ({ value: each })),
                  }
                : inner.type === "number"
                  ? { kind: "number", ...NumberInput.of(definition) }
                  : plainOr(
                        JsonValue.object(toJsonSchema(schema.unwrap(definition))),
                        SCHEMA_KINDS[inner.type],
                    );

        return new ActionInput({ name, label: undefined, presence, value });
    }

    /** The objects the input takes, absent for a value or a choice. */
    get objects(): Extract<ActionValue, { readonly kind: "object" }> | undefined {
        return this.value.kind === "object" ? this.value : undefined;
    }

    /** Read what places give for the input, the nearest first. */
    shown(places: readonly Place[]): JsonValue | undefined {
        // take a variable's nearest value, and leave what a person gives and picks to them
        const { objects, variable } = this;
        if (objects === undefined) {
            return variable === undefined ? undefined : Place.nearest(places, [variable]);
        }

        // take the nearest objects
        const { type, source } = objects;
        const found = new ObjectInput({
            object: type,
            source,
            where: undefined,
            isOptional: false,
        }).read(places);

        return source === "selected" && found.length > 0 ? found : found[0];
    }

    /** List the inputs of some a person still gives: those needed, and those they may give. */
    static asked(inputs: readonly ActionInput[], values: JsonObject): ActionInput[] {
        return inputs.filter(
            ({ name, presence, value }) =>
                values[name] === undefined &&
                (presence === "required" || (presence === "optional" && value.kind !== "json")),
        );
    }

    /** Report whether the input takes a value: objects of its type, a choice, or its kind. */
    takes(value: JsonValue): boolean {
        // take objects of the type, several only where several are selected
        const taken = this.value;
        if (taken.kind === "object") {
            const isSeveral = Array.isArray(value) && taken.source === "selected";
            const references = schema
                .array(ObjectReference)
                .safeParse(isSeveral ? value : [value]).data;

            return references?.every((each) => taken.type.policy.is(each)) === true;
        } else if (taken.kind === "choice") {
            return typeof value === "string" && taken.choices.some((each) => each.value === value);
        } else if (taken.kind === "person") {
            return ObjectReference.safeParse(value).success;
        }

        // take a number within its limits
        else if (taken.kind === "number") {
            return typeof value === "number" && NumberInput.takes(taken, value);
        }
        // take JSON its schema takes
        else if (taken.kind === "json") {
            return fromJsonSchema(taken.schema).safeParse(value).success;
        }

        return typeof value === VALUE_TYPES[taken.kind];
    }

    /** Describe the input by its name, its label, its presence and what it takes. */
    describe(): ActionInputDescription {
        // describe its choices, or its objects
        const { name, label, presence, value, variable, icon, description } = this;
        const shared = {
            name,
            ...(label === undefined ? {} : { label }),
            presence,
            kind: value.kind,
            ...(variable === undefined ? {} : { variable }),
            ...(icon === undefined ? {} : { icon }),
            ...(description === undefined ? {} : { description }),
        };
        if (value.kind === "choice") {
            return { ...shared, choices: [...value.choices] };
        } else if (value.kind === "number") {
            const { kind: _kind, ...numbers } = value;

            return Object.keys(numbers).length === 0 ? shared : { ...shared, numbers };
        } else if (value.kind === "json") {
            return { ...shared, schema: value.schema };
        } else if (value.kind === "object") {
            return {
                ...shared,
                type: value.type.typeReference,
                source: value.source,
                ...(value.where === undefined ? {} : { where: value.where }),
            };
        }

        return shared;
    }

    /** Read an input from its description, finding the type of its objects. */
    static from(
        description: ActionInputDescription,
        typeOf: (reference: ObjectTypeReference) => ObjectType,
    ): ActionInput {
        // read its choices, or its objects
        const { name, label, presence, kind, icon } = description;
        const value: ActionValue =
            kind === "choice"
                ? { kind, choices: description.choices ?? [] }
                : kind === "object"
                  ? {
                        kind,
                        type: typeOf(present(description.type, `the type of input '${name}'`)),
                        source: present(description.source, `the source of input '${name}'`),
                        where: description.where,
                    }
                  : kind === "number"
                    ? { kind, ...description.numbers }
                    : kind === "json"
                      ? {
                            kind,
                            schema: present(description.schema, `the schema of input '${name}'`),
                        }
                      : { kind };

        return new ActionInput({
            name,
            label,
            presence,
            value,
            variable: description.variable,
            icon,
            description: description.description,
        });
    }

    /** Read an input a JSON Schema takes. */
    static ofSchema(name: string, value: JsonValue, presence: InputPresence): ActionInput {
        // take one of its choices, or a value of its type
        const { type, enum: choices } = PROPERTY.parse(value);
        const taken: ActionValue =
            choices !== undefined
                ? { kind: "choice", choices: choices.map((each) => ({ value: each })) }
                : type === "number" || type === "integer"
                  ? { kind: "number", ...NumberInput.ofJson(value) }
                  : plainOr(
                        JsonValue.object(value),
                        typeof type === "string" ? SCHEMA_KINDS[type] : undefined,
                    );

        return new ActionInput({ name, label: undefined, presence, value: taken });
    }
}

/** An input of an action, as its declaration describes it. */
export const ActionInputDescription = schema.object({
    /** The input's name. */
    name: schema.string().min(1),
    /** The input's name as people read it, absent to read it from its name. */
    label: Message.exactOptional(),
    /** Whether the person must give it, may, or leaves it to its default. */
    presence: schema.enum(["required", "optional", "default"]),
    /** What it takes. */
    kind: schema.enum([...VALUE_KINDS, "number", "json", "choice", "object"]),
    /** The key of the variable places give it by. */
    variable: schema.string().min(1).exactOptional(),
    /** The numbers of a number, any when absent. */
    numbers: NumberInput.exactOptional(),
    /** The JSON Schema of a JSON value. */
    schema: schema.jsonObject().exactOptional(),
    /** The choices of a choice. */
    choices: schema.array(FieldChoice).exactOptional(),
    /** The type of an object. */
    type: ObjectTypeReference.exactOptional(),
    /** Where an object comes from. */
    source: schema.enum(["selected", "current", "picked"]).exactOptional(),
    /** The condition an object meets. */
    where: Condition.schema.exactOptional(),
    /** The icon beside its name. */
    icon: IconName.exactOptional(),
    /** One sentence on what it takes and when to change it. */
    description: Message.exactOptional(),
});
/** An input of an action, as its declaration describes it. */
export type ActionInputDescription = schema.Infer<typeof ActionInputDescription>;

/** Read what a schema takes: its plain kind, or JSON of it where it is named or of no kind. */
function plainOr(description: JsonObject, plain: "text" | "boolean" | undefined): ActionValue {
    // keep a named schema, such as a color's, whatever JSON it is, so its own input shows
    const isNamed = typeof description["$ref"] === "string";

    return plain === undefined || isNamed ? { kind: "json", schema: description } : { kind: plain };
}
