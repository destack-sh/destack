import { Now } from "@destack/db";
import { type JsonObject, JsonValue, schema } from "@destack/schema";
import { ObjectReference } from "@destack/sync";
import { Field } from "../field/field.ts";
import type { Moment } from "@destack/db";
import type { Place } from "./place.ts";
import { type ActionValues, ME, SPACE, Variable } from "./variable.ts";

/** The separator between an input's name and a field of its object in a variable's name. */
const FIELD_SEPARATOR = ".";

/** The fields of an object that its reference has, which need no row. */
const REFERENCE_FIELDS: ReadonlySet<string> = new Set(["packageId", "type", "scope", "id"]);

/** A variable in a template's JSON form. */
const PLACEHOLDER = schema.object({ placeholder: schema.string() });

/** What an action reads where it runs: who it is for, where and when, and what is around. */
export interface ActionContext {
    /** Who it is for, absent while nobody is signed in. */
    readonly subject: ObjectReference | undefined;
    /** The space it runs in. */
    readonly scope: string;
    /** The moment times read from. */
    readonly moment: Moment;
    /** The places around, the nearest first, none on a server. */
    readonly places: readonly Place[];
}

/** What a template's variables read: the inputs' values, their objects' rows, and the context. */
export interface TemplateContext {
    /** The inputs' values, by name. */
    readonly values: ActionValues;
    /** The rows of the inputs' objects, by the key of their reference. */
    readonly rows: ReadonlyMap<string, JsonObject>;
    /** What is always there. */
    readonly context: Pick<ActionContext, "subject" | "scope" | "moment">;
}

/** A method's input with variables in place of some values, as plain JSON. */
export class Template {
    /** The input, each variable as `{ placeholder }`. */
    readonly value: JsonObject;

    /** Create a template from its JSON form. */
    constructor(value: JsonObject) {
        this.value = value;
    }

    /** Write a template from an input with variables and times. */
    static of(input: Readonly<Record<string, unknown>>): Template {
        return new Template(JsonValue.object(Template.#encoded(input)));
    }

    /** Report whether a name is one the template reads for itself: the person's, or a time's. */
    static isReserved(name: string): boolean {
        return (
            name === ME ||
            name === SPACE ||
            name.includes(FIELD_SEPARATOR) ||
            Now.parse(name) !== undefined
        );
    }

    /** List the names of the inputs the template reads. */
    get inputs(): string[] {
        const names = Template.#variables(this.value)
            .filter((variable) => Now.parse(variable) === undefined)
            .map((variable) => Template.#root(variable));

        return [...new Set(names)].filter((name) => name !== ME && name !== SPACE);
    }

    /** List the names of the inputs whose objects' rows the template reads. */
    get reads(): string[] {
        const names = Template.#variables(this.value)
            .filter((variable) => {
                const field = Template.#field(variable);

                return field !== undefined && !REFERENCE_FIELDS.has(field);
            })
            .map((variable) => Template.#root(variable));

        return [...new Set(names)].filter((name) => name !== ME);
    }

    /** Read the name of the input a field takes as it is, absent for another value. */
    inputAt(field: string): string | undefined {
        const variable = PLACEHOLDER.safeParse(this.value[field]).data?.placeholder;

        return variable === undefined || Template.isReserved(variable) ? undefined : variable;
    }

    /** Read the field that takes an input as it is, or its object's identifier, absent for none. */
    fieldOf(input: string): string | undefined {
        return Object.entries(this.value).find(([, value]) => {
            const variable = PLACEHOLDER.safeParse(value).data?.placeholder;

            return variable === input || variable === `${input}${FIELD_SEPARATOR}id`;
        })?.[0];
    }

    /** Bind each field, leaving out those no value gives. */
    bind(context: TemplateContext): JsonObject {
        return Object.fromEntries(
            Object.entries(this.value).flatMap(([name, value]) => {
                const bound = value === undefined ? undefined : Template.#bound(value, context);

                return bound === undefined ? [] : [[name, bound]];
            }),
        );
    }

    /** Write a value in JSON form, each variable by its name. */
    static #encoded(value: unknown): unknown {
        // write a variable or a time by its name, and each entry of a list or an object
        if (value instanceof Variable) {
            return { placeholder: Variable.nameOf(value) };
        } else if (value instanceof Now) {
            return { placeholder: value.placeholder };
        } else if (Array.isArray(value)) {
            return value.map((each: unknown) => Template.#encoded(each));
        } else if (typeof value === "object" && value !== null) {
            return Object.fromEntries(
                Object.entries(value).map(([key, each]) => [key, Template.#encoded(each)]),
            );
        }

        return value;
    }

    /** List the names of the variables a value reads. */
    static #variables(value: JsonValue): string[] {
        // read a variable, else those of each entry
        const name = PLACEHOLDER.safeParse(value).data?.placeholder;
        if (name !== undefined) {
            return [name];
        } else if (typeof value !== "object" || value === null) {
            return [];
        }

        return Object.values(value).flatMap((each: JsonValue | undefined) =>
            each === undefined ? [] : Template.#variables(each),
        );
    }

    /** Bind a value, absent where a variable's input has none. */
    static #bound(value: JsonValue, context: TemplateContext): JsonValue | undefined {
        // read a variable, and bind each entry of a list or an object
        const name = PLACEHOLDER.safeParse(value).data?.placeholder;
        if (name !== undefined) {
            return Template.#read(name, context);
        } else if (Array.isArray(value)) {
            return value.flatMap((each: JsonValue) => {
                const bound = Template.#bound(each, context);

                return bound === undefined ? [] : [bound];
            });
        } else if (typeof value === "object" && value !== null) {
            return new Template(JsonValue.object(value)).bind(context);
        }

        return value;
    }

    /** Read a variable: a time, the person, an input's value, or a field of its object. */
    static #read(variable: string, context: TemplateContext): JsonValue | undefined {
        // read a time, and what the variable starts from
        const { values, rows } = context;
        const { subject, scope, moment } = context.context;
        const time = Now.parse(variable);
        const root = Template.#root(variable);
        const value = root === ME ? subject : root === SPACE ? scope : values[root];
        const field = Template.#field(variable);
        if (time !== undefined) {
            return time.at(moment);
        } else if (root === ME && value === undefined) {
            throw new TypeError("variable 'me' has no person: nobody is signed in");
        } else if (value === undefined || field === undefined) {
            return value;
        }

        // read a field of a plain value
        const reference = ObjectReference.safeParse(value).data;
        if (reference === undefined) {
            return JsonValue.object(value)[field];
        }

        // read a field of the object, from its reference or its row
        if (REFERENCE_FIELDS.has(field)) {
            return JsonValue.object(reference)[field];
        }
        const row = rows.get(ObjectReference.key(reference));
        if (row === undefined) {
            throw new TypeError(
                `variable '${variable}' has no row of ${reference.type} '${reference.id}'`,
            );
        }

        // read a person or an object by its columns, else the column
        const { packageId } = Field.columnsOf(field);
        if (field in row || !(packageId in row)) {
            return row[field];
        }

        return row[packageId] === null ? null : ObjectReference.at(field, row);
    }

    /** Read the name of the input a variable reads. */
    static #root(variable: string): string {
        return variable.split(FIELD_SEPARATOR, 1)[0] ?? variable;
    }

    /** Read the name of the field a variable reads of its input's object, absent for the input. */
    static #field(variable: string): string | undefined {
        const separator = variable.indexOf(FIELD_SEPARATOR);

        return separator === -1 ? undefined : variable.slice(separator + 1);
    }
}
