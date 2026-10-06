import { type Localization, t } from "@destack/locale";
import { useLocale } from "@destack/locale/solid";
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
    /** Why the field's value is refused, in the person's language, absent while it is kept. */
    readonly problem: Accessor<string | undefined>;
}

/** The draft input of one method call, checked by the method's own input schema. */
export interface Form<Input extends object> {
    /** One input field. */
    field<Name extends keyof Input & string>(name: Name): FormField<Input[Name]>;
    /** Submit the edited input when every field is kept, answering its submission, absent while a field is refused. */
    submit(): Submission<unknown> | undefined;
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
    return editInput(inputSchemaOf(object, method), options);
}

/** Edit an input checked by a schema, which keeps the fields the form holds. */
function editInput<Input extends object>(
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

    // read the whole input: the starting values with the edits over them
    const current = (): Input => ({ ...options.values(), ...edited() });

    // check the fields the form holds against the method's input schema
    const check = (held: Input): boolean => {
        // keep the first refusal of each held field
        const parsed = input.safeParse(held);
        const refused = new Map<string, string>();
        for (const issue of parsed.success ? [] : parsed.error.issues) {
            const name = String(issue.path[0] ?? "");
            if (name in held && !refused.has(name)) {
                refused.set(name, explain(issue, locale));
            }
        }
        setProblems(refused);

        return refused.size === 0;
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
                // keep the edit and check or submit the input it makes
                edit({ ...draft, [name]: value });
                const held = { ...options.values(), ...draft };
                if (options.mode === "change") {
                    send(held);
                } else {
                    check(held);
                }
            },
            problem: () => problems().get(name),
        }),
        submit: () => send({ ...options.values(), ...draft }),
        status,
        isEdited: () => Object.keys(edited()).length > 0,
        reset: () => {
            // forget the edits and refusals and the last submission
            edit({});
            setProblems(new Map());
            setStatus("idle");
            submissions += 1;
        },
    };
}

/** Explain a problem a schema reports with a value in the person's language. */
function explain(problem: schema.Issue, locale: Localization): string {
    if (problem.code === "too_small" && problem.origin === "string" && problem.minimum === 1) {
        return locale.render(t`Enter a value`);
    } else if (problem.code === "too_small" && problem.origin === "string") {
        return locale.render(t`Enter at least ${String(problem.minimum)} characters`);
    } else if (problem.code === "too_big" && problem.origin === "string") {
        return locale.render(t`Enter at most ${String(problem.maximum)} characters`);
    } else if (problem.code === "too_small") {
        return locale.render(t`Enter ${String(problem.minimum)} or more`);
    } else if (problem.code === "too_big") {
        return locale.render(t`Enter ${String(problem.maximum)} or less`);
    } else if (problem.code === "invalid_type") {
        return locale.render(t`Enter a value`);
    } else if (problem.code === "invalid_value") {
        return locale.render(t`Choose one of the options`);
    } else {
        return locale.render(t`Enter a valid value`);
    }
}
