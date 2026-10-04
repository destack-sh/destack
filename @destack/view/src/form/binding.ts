import { type Localization, t } from "@destack/locale";
import type { Field, ObjectType } from "@destack/object";
import type { Submission } from "@destack/object/client";
import type { ScopeCommands } from "../scope/scope.ts";

/** The milliseconds in a minute, which time zone offsets count in. */
const MINUTE = 60_000;

/** The name of a field an object type declares. */
export type FieldName<Object extends ObjectType> = keyof Object["fields"] & string;

/** The value an object type keeps in a field: its declared value, or none for a field not every object has. */
export type FieldValue<Object extends ObjectType, Name extends FieldName<Object>> =
    Object["fields"][Name] extends Field<infer Configuration>
        ? Configuration["value"] | (Configuration["required"] extends true ? never : null)
        : never;

/** An object field a form field shows and writes: its type and name, its value now, and how a new value is written. */
export interface FieldBinding<
    Object extends ObjectType = ObjectType,
    Name extends FieldName<Object> = FieldName<Object>,
> {
    /** The object type declaring the field. */
    readonly object: Object;
    /** The field's name in the object type. */
    readonly field: Name;
    /** The value the object holds now, predicted while a write is pending. */
    readonly value: FieldValue<Object, Name>;
    /** Write a new value, such as through the object's update, returning its submission. */
    write(value: FieldValue<Object, Name>): Submission<unknown>;
    /** The object's identifier, which a state field's transitions name. */
    readonly id?: string;
    /** The scope the object lives in, which checks and calls a state field's transitions. */
    readonly access?: ScopeCommands;
}

/** The control a field's kind takes when the field renders its own. */
export type FieldKind = "text" | "number" | "boolean" | "time" | "choice" | "state";

/** Where a field's write stands: none yet, waiting for the server, confirmed, or refused. */
export type FieldStatus = "idle" | "pending" | "saved" | "failed";

/** A problem a schema reports with a value, as far as a field explains it. */
interface Problem {
    /** The kind of problem. */
    readonly code: string;
    /** What the bound applies to: a string's length or a number's value. */
    readonly origin?: unknown;
    /** The smallest value or length allowed. */
    readonly minimum?: unknown;
    /** The largest value or length allowed. */
    readonly maximum?: unknown;
}

/** How an object field's values validate. */
export interface FieldColumn {
    /** Validate a value, with the problems of one the schema refuses. */
    readonly safeParse: (
        value: unknown,
    ) =>
        | { readonly success: true }
        | { readonly success: false; readonly error: { readonly issues: readonly Problem[] } };
    /** The values an enum column allows. */
    readonly enumValues: readonly string[] | undefined;
    /** Whether the column may be empty. */
    readonly isNullable: boolean;
}

/** Read how an object field's values validate, refusing a field the object type lacks. */
export function columnOf(binding: FieldBinding): FieldColumn {
    // read the field's schema from the object type's row schema
    const declared = binding.object.fields[binding.field];
    const validator: unknown = Object.entries(binding.object.rowSchema().shape).find(
        ([name]) => name === binding.field,
    )?.[1];
    if (declared === undefined || !isValidator(validator)) {
        throw new TypeError(`${binding.object.name} has no field ${binding.field}`);
    }

    return {
        safeParse: (value) => validator.safeParse(value),
        enumValues:
            "options" in validator && isStrings(validator.options) ? validator.options : undefined,
        isNullable: !declared.required,
    };
}

/** Report whether a value validates values as a field's schema does. */
function isValidator(value: unknown): value is Pick<FieldColumn, "safeParse"> & object {
    return (
        typeof value === "object" &&
        value !== null &&
        "safeParse" in value &&
        typeof value.safeParse === "function"
    );
}

/** Report whether a value lists strings. */
function isStrings(value: unknown): value is readonly string[] {
    return Array.isArray(value) && value.every((entry) => typeof entry === "string");
}

/** List the problems the column schema of a bound field reports with a value, none for a value the field keeps. */
export function problemsOf(binding: FieldBinding, value: unknown): readonly Problem[] {
    const column = columnOf(binding);
    const parsed = value === null && column.isNullable ? undefined : column.safeParse(value);

    return parsed === undefined || parsed.success ? [] : parsed.error.issues;
}

/** Report whether a value is one a bound field keeps, as the schema of the field's column decides. */
export function isFieldValue<Object extends ObjectType, Name extends FieldName<Object>>(
    binding: FieldBinding<Object, Name>,
    value: unknown,
): value is FieldValue<Object, Name> {
    return problemsOf(binding, value).length === 0;
}

/** Read the control a field's declared type takes, refusing types a form field cannot write. */
export function kindOf(binding: FieldBinding): FieldKind {
    const type = binding.object.fields[binding.field]?.type;
    if (type === "string") {
        return "text";
    } else if (type === "integer" || type === "number") {
        return "number";
    } else if (type === "boolean") {
        return "boolean";
    } else if (type === "time") {
        return "time";
    } else if (type === "enum") {
        return "choice";
    } else if (type === "state") {
        return "state";
    } else {
        throw new TypeError(`a ${String(type)} field needs its own control inside the field`);
    }
}

/** Read the value a control's raw input stands for in a field's kind, empty input standing for none. */
export function valueOf(kind: FieldKind, raw: string | boolean): unknown {
    if (typeof raw === "boolean") {
        return raw;
    } else if (raw === "") {
        return null;
    } else if (kind === "number") {
        return Number(raw);
    } else if (kind === "time") {
        return new Date(raw).getTime();
    } else {
        return raw;
    }
}

/** Write a field's value as its control shows it, a time as a local date and time. */
export function displayOf(kind: FieldKind, value: unknown): string {
    if (value === null || value === undefined) {
        return "";
    } else if (kind === "time" && typeof value === "number") {
        const date = new Date(value - new Date(value).getTimezoneOffset() * MINUTE);

        return date.toISOString().slice(0, 16);
    } else {
        return typeof value === "string" || typeof value === "number" || typeof value === "boolean"
            ? String(value)
            : JSON.stringify(value);
    }
}

/** Explain the first problem a schema reports with a value in the person's language. */
export function messageOf(problems: readonly Problem[], locale: Localization): string {
    const problem = problems[0];
    const isText = problem?.origin === "string";
    if (problem?.code === "too_small" && isText && problem.minimum === 1) {
        return locale.render(t`Enter a value`);
    } else if (problem?.code === "too_small" && isText) {
        return locale.render(t`Enter at least ${String(problem.minimum)} characters`);
    } else if (problem?.code === "too_big" && isText) {
        return locale.render(t`Enter at most ${String(problem.maximum)} characters`);
    } else if (problem?.code === "too_small") {
        return locale.render(t`Enter ${String(problem.minimum)} or more`);
    } else if (problem?.code === "too_big") {
        return locale.render(t`Enter ${String(problem.maximum)} or less`);
    } else if (problem?.code === "invalid_type") {
        return locale.render(t`Enter a value`);
    } else if (problem?.code === "invalid_value") {
        return locale.render(t`Choose one of the options`);
    } else {
        return locale.render(t`Enter a valid value`);
    }
}
