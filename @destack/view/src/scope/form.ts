import { type Localization, t } from "@destack/locale";
import { useLocale } from "../page/locale.ts";
import { type CallInput, inputSchemaOf, type MutationName, type ObjectType } from "@destack/object";
import type { Submission } from "@destack/object/client";
import type { schema } from "@destack/schema";
import { type Accessor, createSignal } from "../solid/reactive.ts";

/** Where a form's last submission stands: none yet, waiting for the server, confirmed, or refused. */
export type FormStatus = "idle" | "pending" | "saved" | "failed";

/** When a form submits: on each change of a field, as edits in place, or when it is submitted. */
export type FormMode = "change" | "submit";

/** How a form edits the input of one method call. */
export interface FormOptions<Input extends object> {
    /** The input the form starts from, such as the object's identifier and its current fields. */
    readonly values: () => Input;
    /** Call the method with the edited input, such as `objects.mutate.note.update`. */
    readonly submit: (input: Input) => Submission<unknown>;
    /** When the form submits, on submit by default. */
    readonly mode?: FormMode;
}

/** One input field of a form. */
export interface FormField<Value> {
    /** The value the form holds now: the edited one, else the starting one. */
    readonly value: Accessor<Value>;
    /** Edit the value, checked against the method's input schema, and submit in change mode. */
    set(value: Value): void;
    /** Why the field's value or a value inside it is refused, in the person's language, absent while it is kept. */
    readonly problem: Accessor<string | undefined>;
}

/** The draft input of one method call, checked by the method's own input schema. */
export interface Form<Input extends object> {
    /** One input field. */
    field<Name extends keyof Input & string>(name: Name): FormField<Input[Name]>;
    /** Submit the edited input when every field is kept, answering its submission, absent while a field is refused. */
    submit(): Submission<unknown> | undefined;
    /** Why each shown value is refused, in the person's language, by its JSON Pointer (RFC 6901) such as `/tags/0`. */
    readonly problems: Accessor<ReadonlyMap<string, string>>;
    /** Where the last submission stands. */
    readonly status: Accessor<FormStatus>;
    /** Whether a field holds an edited value. */
    readonly isEdited: Accessor<boolean>;
    /** Drop the edits and refusals, back to the starting input. */
    reset(): void;
}

/** Edit the input of one call of an object's method, every field checked by the method's input schema, after form libraries' field state. */
export function useForm<Object extends ObjectType, Name extends MutationName<Object>>(
    object: Object,
    method: Name,
    options: NoInfer<FormOptions<CallInput<Object, Name>>>,
): Form<CallInput<Object, Name>> {
    return useSchemaForm(inputSchemaOf(object, method), options);
}

/** Edit an input a schema checks, such as one read from JSON Schema, refusing edited values at once and every value once submitted. */
export function useSchemaForm<Input extends object>(
    input: schema.Schema,
    options: FormOptions<Input>,
): Form<Input> {
    // keep the edited fields and refusals and the last submission
    const locale = useLocale();
    const [revision, setRevision] = createSignal(0);
    let draft: Partial<Input> = {};
    const edit = (next: Partial<Input>): void => {
        draft = next;
        setRevision((count) => count + 1);
    };
    const edited = (): Partial<Input> => {
        revision();

        return draft;
    };
    const [problems, setProblems] = createSignal<ReadonlyMap<string, string>>(new Map());
    const [status, setStatus] = createSignal<FormStatus>("idle");
    let submissions = 0;
    let isSubmitted = false;
    const touched = new Set<string>();

    // read the whole input: the starting values with the edits over them
    const current = (): Input => ({ ...options.values(), ...edited() });

    // check the input against the schema, showing refusals of edited values until the first submit
    const check = (held: Input): boolean => {
        // refuse the issues of the fields the form holds, leaving the rest to the caller
        const parsed = input.safeParse(held);
        const issues = (parsed.success ? [] : parsed.error.issues)
            .filter((issue) => String(issue.path[0] ?? "") in held)
            .flatMap((issue) => closest(issue, held));

        // show the first refusal at each pointer of an edited value, or of every value once submitted
        const refused = new Map<string, string>();
        for (const issue of issues) {
            const pointer = pointerOf(issue.path);
            const isShown = isSubmitted || touched.has(pointer);
            if (isShown && !refused.has(pointer)) {
                refused.set(pointer, explain(issue, valueAt(held, issue.path), locale));
            }
        }
        setProblems(refused);

        return issues.length === 0;
    };

    // submit a kept input and follow the server's answer while no later submission took over
    const send = (held: Input): Submission<unknown> | undefined => {
        if (!check(held)) {
            return undefined;
        }
        submissions += 1;
        const submission = submissions;
        setStatus("pending");
        const submitted = options.submit(held);
        submitted.confirmed.then(
            () => {
                if (submission === submissions) {
                    setStatus("saved");
                    edit({});
                }
            },
            () => submission === submissions && setStatus("failed"),
        );

        return submitted;
    };

    return {
        field: (name) => ({
            value: () => current()[name],
            set: (value) => {
                // keep the edit, marking the values it changes, and check or submit the input it makes
                touch(touched, current()[name], value, [name]);
                edit({ ...draft, [name]: value });
                const held = { ...options.values(), ...draft };
                if (options.mode === "change") {
                    send(held);
                } else {
                    check(held);
                }
            },
            problem: () => firstUnder(problems(), pointerOf([name])),
        }),
        submit: () => {
            // show every refusal from the first submit on
            isSubmitted = true;

            return send({ ...options.values(), ...draft });
        },
        problems,
        status,
        isEdited: () => Object.keys(edited()).length > 0,
        reset: () => {
            // forget the edits and refusals and the last submission
            edit({});
            setProblems(new Map());
            setStatus("idle");
            isSubmitted = false;
            touched.clear();
            submissions += 1;
        },
    };
}

/** Write a value's path as a JSON Pointer (RFC 6901), escaping `~` and `/`. */
function pointerOf(path: readonly PropertyKey[]): string {
    return path
        .map((key) => `/${String(key).replaceAll("~", "~0").replaceAll("/", "~1")}`)
        .join("");
}

/** Mark the pointers of the values an edit changes, and of the values holding them. */
function touch(
    touched: Set<string>,
    before: unknown,
    after: unknown,
    path: readonly PropertyKey[],
): void {
    // leave an unchanged value unmarked
    if (Object.is(before, after)) {
        return;
    }

    // mark the value, then descend into the entries of an object or list it now holds
    touched.add(pointerOf(path));
    if (isContainer(after)) {
        const previous = isContainer(before) ? before : {};
        const keys = new Set([...Object.keys(previous), ...Object.keys(after)]);
        for (const key of keys) {
            touch(touched, previous[key], after[key], [...path, key]);
        }
    }
}

/** Report whether a value holds entries by key: an object or a list. */
function isContainer(value: unknown): value is Readonly<Record<string, unknown>> {
    return typeof value === "object" && value !== null;
}

/** Read the issues of the union member closest to a held value, or the issue itself for a missing value or another issue. */
function closest(issue: schema.Issue, held: unknown): schema.Issue[] {
    // keep an issue outside a union, or a union missing its value
    if (issue.code !== "invalid_union" || valueAt(held, issue.path) === undefined) {
        return [issue];
    }

    // take the member with the fewest issues, placed under the union's path
    const members = issue.errors.toSorted((left, right) => left.length - right.length);
    const nearest = members[0] ?? [];
    if (nearest.length === 0) {
        return [issue];
    }

    return nearest
        .map((inner) => ({ ...inner, path: [...issue.path, ...inner.path] }))
        .flatMap((inner) => closest(inner, held));
}

/** Read the value at a path inside a held value, absent where the path leads nowhere. */
function valueAt(held: unknown, path: readonly PropertyKey[]): unknown {
    let found = held;
    for (const key of path) {
        found = isContainer(found) ? found[String(key)] : undefined;
    }

    return found;
}

/** Read the first refusal at a pointer or inside the value it points at. */
function firstUnder(problems: ReadonlyMap<string, string>, pointer: string): string | undefined {
    for (const [at, problem] of problems) {
        if (at === pointer || at.startsWith(`${pointer}/`)) {
            return problem;
        }
    }

    return undefined;
}

/** Explain a problem a schema reports with a value in the person's language. */
function explain(problem: schema.Issue, value: unknown, locale: Localization): string {
    // lengths of text
    if (problem.code === "too_small" && problem.origin === "string" && problem.minimum === 1) {
        return locale.render(t`Enter a value`);
    } else if (problem.code === "too_small" && problem.origin === "string") {
        return locale.render(t`Enter at least ${String(problem.minimum)} characters`);
    } else if (problem.code === "too_big" && problem.origin === "string") {
        return locale.render(t`Enter at most ${String(problem.maximum)} characters`);
    }
    // lengths of lists
    else if (problem.code === "too_small" && problem.origin === "array") {
        return locale.render(t`Add at least ${String(problem.minimum)}`);
    } else if (problem.code === "too_big" && problem.origin === "array") {
        return locale.render(t`Add at most ${String(problem.maximum)}`);
    }
    // bounds of numbers
    else if (problem.code === "too_small" && problem.inclusive === false) {
        return locale.render(t`Enter more than ${String(problem.minimum)}`);
    } else if (problem.code === "too_small") {
        return locale.render(t`Enter ${String(problem.minimum)} or more`);
    } else if (problem.code === "too_big" && problem.inclusive === false) {
        return locale.render(t`Enter less than ${String(problem.maximum)}`);
    } else if (problem.code === "too_big") {
        return locale.render(t`Enter ${String(problem.maximum)} or less`);
    } else if (problem.code === "not_multiple_of") {
        return locale.render(t`Enter a multiple of ${String(problem.divisor)}`);
    }
    // formats of text
    else if (problem.code === "invalid_format" && problem.format === "email") {
        return locale.render(t`Enter an email address`);
    } else if (problem.code === "invalid_format" && problem.format === "url") {
        return locale.render(t`Enter a web address`);
    } else if (problem.code === "invalid_format" && problem.format === "date") {
        return locale.render(t`Enter a date`);
    } else if (problem.code === "invalid_format" && problem.format === "datetime") {
        return locale.render(t`Enter a date and time`);
    }
    // missing values, kinds of numbers and choices
    else if (value === undefined || value === null) {
        return locale.render(t`Enter a value`);
    } else if (problem.code === "invalid_type" && problem.expected === "int") {
        return locale.render(t`Enter a whole number`);
    } else if (problem.code === "invalid_type" && problem.expected === "number") {
        return locale.render(t`Enter a number`);
    } else if (problem.code === "invalid_value" || problem.code === "invalid_union") {
        return locale.render(t`Choose one of the options`);
    } else {
        return locale.render(t`Enter a valid value`);
    }
}
