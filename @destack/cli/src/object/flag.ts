import { Casing, type ObjectType } from "@destack/object";
import { principal } from "@destack/access";
import {
    type Comparable,
    Filter,
    type FilterOptions,
    type JsonCondition,
    TABLE,
} from "@destack/db";
import { type JsonObject, type JsonValue, present, schema, nth } from "@destack/schema";
import { Scope, Subject } from "@destack/sync";
import { GLOBAL_OPTIONS, type Option, OptionValue, type Options } from "../command/command.ts";

import { CliError } from "../error/index.ts";
import { WallTime } from "./time.ts";

/** The options a list reads beside its field flags. */
export const LIST_OPTIONS = {
    where: {
        type: "string",
        description: "A filter the rows meet, such as 'status = open AND due < @friday'",
        argument: "filter",
    },
    sort: {
        type: "string",
        description: "The fields the rows sort by, a minus first for descending",
        argument: "fields",
    },
    limit: { type: "string", description: "The most rows to show", argument: "count" },
    fields: {
        type: "string",
        description: "The fields to show, comma separated",
        argument: "fields",
    },
    watch: { type: "boolean", description: "Follow the rows as they change" },
} as const;

/** The option giving a method's whole input as JSON, under its field flags. */
export const INPUT_OPTION = {
    input: {
        type: "string",
        description: "The method's input as a JSON object, - for stdin, @path for a file",
        argument: "json",
    },
} as const;

/** The options every noun's list reads itself, which no field's flag takes. */
const RESERVED: ReadonlySet<string> = new Set([
    ...Object.keys(LIST_OPTIONS),
    ...Object.keys(INPUT_OPTION),
    ...Object.keys(GLOBAL_OPTIONS),
]);

/** The days of `@name`, by their offset from today. */
const RELATIVE_DAYS: Readonly<Record<string, number>> = { yesterday: -1, today: 0, tomorrow: 1 };

/** The weekdays of `@name`, by their index in a JavaScript date. */
const WEEKDAYS = ["sunday", "monday", "tuesday", "wednesday", "thursday", "friday", "saturday"];

/** The days of a week. */
const WEEK_DAYS = 7;

/** A calendar date, read as local midnight. */
const DATE = /^(\d{4})-(\d{2})-(\d{2})$/u;

/** The pattern of an identifier's JSON Schema, with the identity before its UUID. */
const IDENTIFIER = /^\^([a-z][a-z0-9-]*?)-\[0-9a-f\]\{8\}-/u;

/** The JSON Schema of a value, as far as flags read it. */
const ValueSchema = schema.looseObject({
    type: schema.union([schema.string(), schema.array(schema.string())]).exactOptional(),
    enum: schema.array(schema.json()).exactOptional(),
    const: schema.json().exactOptional(),
    pattern: schema.string().exactOptional(),
    anyOf: schema.array(schema.json()).exactOptional(),
    oneOf: schema.array(schema.json()).exactOptional(),
    properties: schema.jsonObject().exactOptional(),
    items: schema.json().exactOptional(),
});

/** The JSON Schema of a value, as far as flags read it. */
type ValueSchema = schema.Infer<typeof ValueSchema>;

/** The value type a flag reads, from its property's JSON Schema. */
type FlagType = "string" | "integer" | "number" | "boolean";

/** A day of the calendar: its year, month from 1 and day of the month. */
type CalendarDate = Pick<WallTime, "year" | "month" | "day">;

/** A flag's value type, enumeration and identity, as its property's JSON Schema gives them. */
type FlagShape = Pick<Flag, "type" | "values" | "identity" | "isKeyed">;

/** One field a flag refers to. */
export class Flag {
    /** The flag's name in kebab case: its path joined by dashes, a reference's without Id. */
    readonly name: string;
    /** The path of the row or input property, nested for a structured field. */
    readonly path: readonly string[];
    /** The value type the flag reads. */
    readonly type: FlagType;
    /** The values an enumeration allows, absent for any. */
    readonly values?: readonly string[];
    /** The identity of the objects whose identifiers the flag takes. */
    readonly identity?: string;
    /** Whether the flag also takes the objects' keys. */
    readonly isKeyed: boolean;

    /** Refer to a field by its name and path, with its shape. */
    constructor(name: string, path: readonly string[], shape: FlagShape) {
        // set the name and path, and the shape's enumeration and identity when it has them
        this.name = name;
        this.path = path;
        this.type = shape.type;
        this.isKeyed = shape.isKeyed;
        if (shape.values !== undefined) {
            this.values = shape.values;
        }
        if (shape.identity !== undefined) {
            this.identity = shape.identity;
        }
    }

    /** Describe the flag's command-line option: its words, its choices and the value it takes. */
    option(): Option {
        // describe the flag's words, its choices, and the value it takes
        const words = this.name.replaceAll("-", " ");
        const choices = this.values === undefined ? "" : `: ${this.values.join(", ")}`;
        const argument =
            this.identity === undefined
                ? (this.values?.join("|") ?? this.type)
                : this.isKeyed
                  ? "key|id"
                  : "id";

        return this.type === "boolean"
            ? { type: "boolean", description: `Set ${words}` }
            : { type: "string", description: `The ${words}${choices}`, argument };
    }

    /** Read the flag's text as its property's value. */
    value(text: string, filter: FilterOptions): JsonValue {
        // read a named value
        let value: Comparable | null;
        if (text.startsWith("@")) {
            const named = filter.values?.[text.slice(1)];
            if (named === undefined) {
                throw CliError.of(
                    "USAGE_INVALID",
                    `--'${this.name}' value '${text}' is invalid: it is unknown`,
                );
            }
            value = named;
        }
        // read null
        else if (text === "null") {
            value = null;
        }
        // read a number, leaving other text such as a date or nothing for its field to convert
        else if (this.type === "integer" || this.type === "number") {
            const number = text.trim() === "" ? Number.NaN : Number(text);
            value = Number.isNaN(number) ? text : number;
        }
        // read text, refusing a value outside an enumeration
        else {
            if (this.values !== undefined && !this.values.includes(text)) {
                throw CliError.of(
                    "USAGE_INVALID",
                    `--'${this.name}' is invalid: it takes one of '${this.values.join(", ")}'`,
                );
            }
            value = text;
        }

        return value === null || filter.convert === undefined
            ? value
            : filter.convert(this.path, value);
    }

    /** Write a value of the flag in the filter syntax. */
    quote(value: string): string {
        if (value.startsWith("@") || value === "null" || this.type !== "string") {
            return value;
        }

        return `"${value.replaceAll("\\", "\\\\").replaceAll('"', '\\"')}"`;
    }

    /** Report whether a value of the flag is an object's key, not an identifier or an @ value. */
    isKey(value: string): boolean {
        const isIdentifier =
            this.identity !== undefined &&
            schema.identifier(this.identity).safeParse(value).success;

        return !isIdentifier && !value.startsWith("@") && value !== "null";
    }
}

/** The flags of a type's fields: a list's restrictions or a method's input. */
export class FlagSet {
    /** The object type whose fields the flags refer to. */
    readonly type: ObjectType;
    /** The flags. */
    readonly flags: readonly Flag[];

    /** Keep a type's flags. */
    constructor(type: ObjectType, flags: readonly Flag[]) {
        this.type = type;
        this.flags = flags;
    }

    /** Read a method's flags, one per scalar input path, leaving out the fields the call routes. */
    static input(type: ObjectType, method: string): FlagSet {
        // require the method and an object input
        const input = type.describeMethods()[method]?.input;
        if (input === undefined) {
            throw CliError.of(
                "USAGE_INVALID",
                `method '${method}' is invalid: '${Casing.kebab(type.plural)}' have no such method`,
            );
        }
        const properties = FlagSet.#properties([input]);
        if (properties === undefined) {
            throw CliError.of(
                "METHOD_NOT_FOUND",
                `object input of '${Casing.kebab(type.plural)}.${method}' not found`,
            );
        }

        // collect a flag per path, structured fields included
        const routed = [type.route.field, "id"].filter((field) => field !== undefined);

        return new FlagSet(type, FlagSet.#collect(type, properties, routed, [], true));
    }

    /** Read a list's flags, one per scalar field of the rows its list method answers. */
    static rows(type: ObjectType): FlagSet {
        // require a list method answering a page of object rows
        const plural = Casing.kebab(type.plural);
        const list = Object.values(type.describeMethods()).find((each) => each.kind === "list");
        const items =
            list === undefined ? undefined : FlagSet.#properties([list.output])?.["items"];
        const rows = items?.map((each) => ValueSchema.parse(each).items);
        const properties =
            rows === undefined || rows.includes(undefined)
                ? undefined
                : FlagSet.#properties(rows.filter((row) => row !== undefined));
        if (properties === undefined) {
            throw CliError.of(
                "METHOD_NOT_FOUND",
                `list of '${plural}' answering object rows not found`,
            );
        }

        return new FlagSet(type, FlagSet.#collect(type, properties, [], [], false));
    }

    /** Leave out the flags of some input fields, such as those a command sets. */
    without(fields: readonly string[]): FlagSet {
        const set = new Set(fields);

        return new FlagSet(
            this.type,
            this.flags.filter((flag) => !set.has(nth(flag.path, 0))),
        );
    }

    /** Describe the flags' command-line options, by flag name. */
    options(): Record<string, Option> {
        return Object.fromEntries(this.flags.map((flag) => [flag.name, flag.option()]));
    }

    /**
     * Read how filters and flags read the type's fields.
     *
     * `@me` is the person, days are their local midnight, and dates apply to time fields.
     */
    filter(userId: string | undefined, now: number, timeZone: string): FilterOptions {
        // read now and today in the person's time zone
        const today = WallTime.of(now, timeZone);
        const day = (offset: number) =>
            FlagSet.#midnight(
                { year: today.year, month: today.month, day: today.day + offset },
                timeZone,
            );
        const weekday = new Date(Date.UTC(today.year, today.month - 1, today.day)).getUTCDay();

        // name the days around today and the coming weekdays
        const values: Record<string, Comparable> = {
            ...(userId === undefined ? {} : { me: userId }),
            now,
        };
        for (const [name, offset] of Object.entries(RELATIVE_DAYS)) {
            values[name] = day(offset);
        }
        for (const [index, name] of WEEKDAYS.entries()) {
            values[name] = day((index - weekday + WEEK_DAYS) % WEEK_DAYS);
        }

        return {
            fields: new Set(Object.keys(this.type.table[TABLE].columns)),
            values,
            convert: (path, literal) => this.#convert(path, literal, timeZone),
        };
    }

    /** Read the keys given to flags of identifiers as the identifiers of their objects. */
    async identify(
        options: Options,
        find: (identity: string, name: string) => Promise<string>,
    ): Promise<Options> {
        const identified: Record<string, string | boolean | undefined> = { ...options };
        for (const flag of this.flags) {
            // find each key of a given flag of identifiers, passing identifiers and @ values through
            const { identity } = flag;
            const given = options[flag.name];
            if (identity !== undefined && typeof given === "string") {
                const values = given
                    .split(",")
                    .map(async (value) => (flag.isKey(value) ? find(identity, value) : value));
                identified[flag.name] = (await Promise.all(values)).join(",");
            }
        }

        return identified;
    }

    /** Build the condition the given flags and --where restrict rows by, absent for none. */
    condition(options: Options, filter: FilterOptions): JsonCondition | undefined {
        // read --where through the filter syntax
        const written = OptionValue.string(options["where"], "where");
        const conditions: JsonCondition[] =
            written === undefined ? [] : [Filter.parse(written, filter)];

        // read each given flag's values as any of them, through the filter syntax
        for (const flag of this.flags) {
            const given = options[flag.name];
            if (given !== undefined) {
                const field = flag.path.join(".");
                const restrictions = String(given)
                    .split(",")
                    .map((value) => `${field} = ${flag.quote(value)}`);
                conditions.push(Filter.parse(restrictions.join(" OR "), filter));
            }
        }

        // join the restrictions
        const [only] = conditions;

        return conditions.length <= 1 ? only : { AND: conditions };
    }

    /** Build the input the given flags set, over a JSON input. */
    input(options: Options, base: JsonObject, filter: FilterOptions): JsonObject {
        let input: JsonObject = base;
        for (const flag of this.flags) {
            // read each given flag as its path's value
            const given = options[flag.name];
            if (given !== undefined) {
                const value = typeof given === "boolean" ? given : flag.value(given, filter);
                input = FlagSet.#assign(input, flag.path, value);
            }
        }

        return input;
    }

    /** Convert a literal for its field: a date for time, a person's key for a subject. */
    #convert(path: readonly string[], literal: Comparable, timeZone: string): Comparable {
        const field = path.length === 1 ? this.type.fields[nth(path, 0)] : undefined;

        // read a date or an instant for a time field
        if (field?.type === "time" && typeof literal === "string") {
            const date = DATE.exec(literal);
            const instant =
                date === null
                    ? Date.parse(literal)
                    : FlagSet.#midnight(
                          { year: Number(date[1]), month: Number(date[2]), day: Number(date[3]) },
                          timeZone,
                      );
            if (Number.isNaN(instant)) {
                throw CliError.of(
                    "USAGE_INVALID",
                    `'${path.join(".")}' is invalid: it takes a date, such as 2026-10-09 or @friday`,
                );
            }

            return instant;
        }
        // key a person for a subject field of people
        else if (
            field?.type === "subject" &&
            field.principals?.some((kind) => kind === principal.user) === true &&
            typeof literal === "string" &&
            !literal.startsWith("[")
        ) {
            return Subject.key(principal.user.reference(Scope.universe.id, literal));
        }
        // pass any other literal through
        else {
            return literal;
        }
    }

    /** Build the flags of some properties' scalar paths, leaving out routed and reserved ones. */
    static #collect(
        type: ObjectType,
        properties: Readonly<Record<string, readonly JsonValue[]>>,
        routed: readonly string[],
        prefix: readonly string[],
        isNested: boolean,
    ): Flag[] {
        return Object.entries(properties).flatMap(([property, alternatives]): Flag[] => {
            // leave out a routed or reserved property
            const path = [...prefix, property];
            const name = [
                FlagSet.#propertyName(type, nth(path, 0)),
                ...path.slice(1).map((segment) => Casing.kebab(segment)),
            ].join("-");
            if (routed.includes(property) || RESERVED.has(name)) {
                return [];
            }

            // take a scalar, and the scalars of a structured value where nested
            const scalar = FlagSet.#scalar(alternatives);
            const nested = isNested ? FlagSet.#properties(alternatives) : undefined;
            // take a scalar
            if (scalar !== undefined) {
                return [new Flag(name, path, scalar)];
            }
            // take the scalars of a nested structured value
            else if (nested !== undefined) {
                return FlagSet.#collect(type, nested, [], path, true);
            }
            // take nothing of another value
            else {
                return [];
            }
        });
    }

    /** Choose a top-level property's flag name, in kebab case. */
    static #propertyName(type: ObjectType, property: string): string {
        const field = type.fields[property];
        const name =
            field?.type === "reference" && property.endsWith("Id")
                ? property.slice(0, -2)
                : property;

        return Casing.kebab(name);
    }

    /** List a value's alternatives: its schema, or each of a union's, without null. */
    static #alternatives(values: readonly JsonValue[]): ValueSchema[] {
        return values.flatMap((value): ValueSchema[] => {
            // read each alternative of a union
            const parsed = ValueSchema.safeParse(value);
            if (!parsed.success) {
                return [];
            }
            const union = parsed.data.anyOf ?? parsed.data.oneOf;
            if (union !== undefined) {
                return FlagSet.#alternatives(union);
            }

            return parsed.data.type === "null" ? [] : [parsed.data];
        });
    }

    /** Read a scalar value's type, enumeration and identity, absent for structured values. */
    static #scalar(values: readonly JsonValue[]): FlagShape | undefined {
        // take one scalar type
        const alternatives = FlagSet.#alternatives(values);
        const types = alternatives.flatMap((entry) =>
            Array.isArray(entry.type) ? entry.type : [entry.type],
        );
        const [type, ...others] = new Set(types.filter((entry) => entry !== "null"));
        if (others.length > 0 || !FlagSet.#isType(type)) {
            return undefined;
        }

        // take an enumeration every alternative makes, and the identity of identifiers
        const isEnumerated = alternatives.every(
            (entry) => entry.enum !== undefined || "const" in entry,
        );
        const enumerated = alternatives.flatMap(
            (entry) => entry.enum ?? ("const" in entry ? [entry.const] : []),
        );
        const read = alternatives.map((entry) => IDENTIFIER.exec(entry.pattern ?? "")?.[1]);
        const [identity, ...otherIdentities] = new Set(read.filter((each) => each !== undefined));
        const isIdentified = identity !== undefined && otherIdentities.length === 0;

        return {
            type,
            ...(isEnumerated && enumerated.length > 0
                ? { values: [...new Set(enumerated.map(String))] }
                : {}),
            ...(isIdentified ? { identity } : {}),
            isKeyed: isIdentified && read.includes(undefined),
        };
    }

    /** Merge the properties of a value's object alternatives, absent unless each is an object. */
    static #properties(values: readonly JsonValue[]): Record<string, JsonValue[]> | undefined {
        // require objects
        const alternatives = FlagSet.#alternatives(values);
        if (
            alternatives.length === 0 ||
            alternatives.some((entry) => entry.properties === undefined)
        ) {
            return undefined;
        }

        // collect each property's schemas across the alternatives
        const merged: Record<string, JsonValue[]> = {};
        for (const entry of alternatives) {
            for (const [property, value] of Object.entries(entry.properties ?? {})) {
                merged[property] = [...(merged[property] ?? []), present(value, property)];
            }
        }

        return merged;
    }

    /** Read the instant a date starts in a time zone, a day past the month's end rolling over. */
    static #midnight(date: CalendarDate, timeZone: string): number {
        // move the date's midnight in UTC by the zone's offset, then once more across an offset change
        const utc = Date.UTC(date.year, date.month - 1, date.day);
        const first = utc - WallTime.offset(utc, timeZone);

        return utc - WallTime.offset(first, timeZone);
    }

    /** Report whether a JSON Schema type is a flag's. */
    static #isType(type: string | undefined): type is FlagType {
        return type === "string" || type === "integer" || type === "number" || type === "boolean";
    }

    /** Set a value at a path of an input, leaving the fields beside it. */
    static #assign(input: JsonObject, path: readonly string[], value: JsonValue): JsonObject {
        // set a top-level field
        const head = nth(path, 0);
        const rest = path.slice(1);
        const given = input[head];
        if (rest.length === 0) {
            return { ...input, [head]: value };
        }
        // refuse a field --input gives as anything but the object the flag sets a field of
        else if (given !== undefined && !schema.jsonObject().safeParse(given).success) {
            throw CliError.of(
                "USAGE_INVALID",
                `--input field '${head}' is invalid: it is not an object`,
            );
        }
        // set a field of the object at the head, starting one where none is given
        else {
            const inner = given === undefined ? {} : schema.jsonObject().parse(given);

            return { ...input, [head]: FlagSet.#assign(inner, rest, value) };
        }
    }
}
