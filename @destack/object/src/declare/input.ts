import { Condition, type JsonCondition } from "@destack/db";
import { Message } from "@destack/locale";
import { type JsonObject, type JsonValue, present, schema } from "@destack/schema";
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
export const VALUE_KINDS = ["text", "number", "boolean", "time", "person", "json"] as const;

/** The type of the JSON value of each kind a person gives. */
const VALUE_TYPES = {
    text: "string",
    number: "number",
    boolean: "boolean",
    time: "number",
} as const;

/** What a schema that wraps another says of the need for a value, by its type. */
const NEEDS: Readonly<Partial<Record<string, ActionNeed>>> = {
    optional: "optional",
    default: "default",
    prefault: "default",
};

/** The kinds of value people give for a schema, by its type. */
const SCHEMA_KINDS: Readonly<Partial<Record<string, "text" | "number" | "boolean">>> = {
    string: "text",
    number: "number",
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
export type ActionNeed = "required" | "optional" | "default";

/** What an input takes: a value, one of some choices, or objects from a source. */
export type ActionValue =
    | { readonly kind: (typeof VALUE_KINDS)[number] }
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
    readonly need: ActionNeed;
    /** What it takes. */
    readonly value: ActionValue;
    /** The key of the variable places give it by, absent for any other input. */
    readonly variable: string | undefined;

    /** Create an input. */
    constructor(
        definition: Pick<ActionInput, "name" | "label" | "need" | "value"> &
            Partial<Pick<ActionInput, "variable">>,
    ) {
        // take the name, the label, the need and the value
        this.name = definition.name;
        this.label = definition.label;
        this.need = definition.need;
        this.value = definition.value;
        this.variable = definition.variable;
    }

    /** Read an input as its definition declares it. */
    static of(name: string, definition: ActionInputDefinition): ActionInput {
        // take objects from where the person is, what a field takes, or what a schema does
        if (definition instanceof ObjectInput) {
            const { object: type, source, where, isOptional } = definition;

            return new ActionInput({
                name,
                label: undefined,
                need: isOptional ? "default" : "required",
                value: {
                    kind: "object",
                    type,
                    source,
                    where: where === undefined ? undefined : type.condition(where),
                },
            });
        } else if (definition instanceof ContextVariable) {
            const { label, key: variable } = definition;
            const { need, value } = ActionInput.of(name, definition.value);

            return new ActionInput({ name, label, need, value, variable });
        } else if ("actionInput" in definition) {
            return definition.actionInput(name);
        }

        // read an optional or defaulted schema as what it wraps
        const outer = SCHEMA.parse(definition);
        const need = NEEDS[outer.type] ?? "required";
        const inner = need === "required" ? outer : SCHEMA.parse(outer.def.innerType);
        const value: ActionValue =
            inner.type === "enum"
                ? {
                      kind: "choice",
                      choices: (inner.options ?? []).map((each) => ({ value: each })),
                  }
                : { kind: SCHEMA_KINDS[inner.type] ?? "json" };

        return new ActionInput({ name, label: undefined, need, value });
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
        } else if (objects.source === "picked") {
            return undefined;
        }

        // take the nearest objects: those selected, or the current one alone
        const { type, source } = objects;
        const nearest = Place.nearest(places, [
            ObjectInput.keyOf("selected", type.typeReference),
            ObjectInput.keyOf("current", type.typeReference),
        ]);
        const found: readonly JsonValue[] =
            nearest === undefined ? [] : Array.isArray(nearest) ? nearest : [nearest];
        if (source === "selected") {
            return found.length === 0 ? undefined : found;
        }

        return found.length === 1 ? found[0] : undefined;
    }

    /** List the inputs of some a person still gives: those needed, and those they may give. */
    static asked(inputs: readonly ActionInput[], values: JsonObject): ActionInput[] {
        return inputs.filter(
            ({ name, need, value }) =>
                values[name] === undefined &&
                (need === "required" || (need === "optional" && value.kind !== "json")),
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

        return taken.kind === "json" || typeof value === VALUE_TYPES[taken.kind];
    }

    /** Describe the input by its name, its label, its need and what it takes. */
    describe(): ActionInputDescription {
        // describe its choices, or its objects
        const { name, label, need, value, variable } = this;
        const shared = {
            name,
            ...(label === undefined ? {} : { label }),
            need,
            kind: value.kind,
            ...(variable === undefined ? {} : { variable }),
        };
        if (value.kind === "choice") {
            return { ...shared, choices: [...value.choices] };
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
        const { name, label, need, kind } = description;
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
                  : { kind };

        return new ActionInput({ name, label, need, value, variable: description.variable });
    }

    /** Read an input a JSON Schema takes. */
    static ofSchema(name: string, value: JsonValue, need: ActionNeed): ActionInput {
        // take one of its choices, or a value of its type
        const { type, enum: choices } = PROPERTY.parse(value);
        const taken: ActionValue =
            choices !== undefined
                ? { kind: "choice", choices: choices.map((each) => ({ value: each })) }
                : type === "number" || type === "integer"
                  ? { kind: "number" }
                  : type === "boolean"
                    ? { kind: "boolean" }
                    : { kind: type === "string" ? "text" : "json" };

        return new ActionInput({ name, label: undefined, need, value: taken });
    }
}

/** An input of an action, as its declaration describes it. */
export const ActionInputDescription = schema.object({
    /** The input's name. */
    name: schema.string().min(1),
    /** The input's name as people read it, absent to read it from its name. */
    label: Message.exactOptional(),
    /** Whether the person must give it, may, or leaves it to its default. */
    need: schema.enum(["required", "optional", "default"]),
    /** What it takes. */
    kind: schema.enum([...VALUE_KINDS, "choice", "object"]),
    /** The key of the variable places give it by. */
    variable: schema.string().min(1).exactOptional(),
    /** The choices of a choice. */
    choices: schema.array(FieldChoice).exactOptional(),
    /** The type of an object. */
    type: ObjectTypeReference.exactOptional(),
    /** Where an object comes from. */
    source: schema.enum(["selected", "current", "picked"]).exactOptional(),
    /** The condition an object meets. */
    where: Condition.schema.exactOptional(),
});
/** An input of an action, as its declaration describes it. */
export type ActionInputDescription = schema.Infer<typeof ActionInputDescription>;
