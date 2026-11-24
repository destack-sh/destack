import { type Localization, t } from "@destack/locale";
import { ObjectError } from "@destack/object";
import type { Submission } from "@destack/object/client";
import { schema } from "@destack/schema";
import { TelemetryScope } from "@destack/telemetry";
import { useEnvironment } from "../page/context.ts";
import { useLocale } from "../page/locale.ts";
import { StoredItem } from "../primitives/storage.ts";
import { type Accessor, createSignal, onSettled, type Setter } from "../solid/reactive.ts";

/** What starts the keys of the drafts a browser keeps. */
const DRAFT_KEY = "destack-form:";

/** The message of a problem whose schema has no words of its own for it. */
const UNWORDED = "";

/** Where a form's last submission stands. */
export type FormStatus = "idle" | "pending" | "saved" | "failed";

/** When a form submits: as it is asked to, as a person settles on a value, or on each change. */
export type FormMode = "submit" | "settle" | "change";

/** What a form edits, how it checks it and where it goes. */
export interface FormOptions<Values extends object> {
    /** The values the form starts from, which may change under it. */
    readonly values: () => Values;
    /** What checks the values, nothing by default. */
    readonly schema?: schema.Schema | undefined;
    /** Take the values, with the edited ones alone: what settles, or nothing once it is done. */
    readonly submit: (
        values: Values,
        edits: Partial<Values>,
    ) => Submission<unknown> | Promise<unknown> | undefined;
    /** When the form submits, as it is asked to by default. */
    readonly mode?: FormMode | undefined;
    /** Where the edits outlive the page until they are saved, nowhere by default. */
    readonly draft?: FormDraft<Values> | undefined;
}

/** The edits of a form a browser keeps. */
export interface FormDraft<Values extends object> {
    /** What tells them from those of other forms, such as the object they are of. */
    readonly name: string;
    /** What reads them back, dropping those of another shape. */
    readonly schema: schema.Schema<Partial<Values>>;
}

/** A value someone else set while a person edits it. */
export interface FormConflict<Value> {
    /** Their value. */
    readonly value: Value;
}

/** How a field reads and writes its value in a form. */
interface FormLens<Value> {
    /** Read the value as edited. */
    readonly value: () => Value;
    /** Read the value the form starts from now, none for an item it starts without. */
    readonly start: () => readonly Value[];
    /** Read the values it started from as a person edited it, and as a person submitted it. */
    readonly known: () => readonly Value[];
    /** Write the value. */
    write(value: Value): void;
}

/** What a field asks of its form. */
interface FormState {
    /** Show why values at a path are refused, and submit them where the form does. */
    readonly settle: (path: readonly string[]) => void;
    /** Read why the value at a path or one inside it is refused. */
    readonly problem: (path: readonly string[]) => string | undefined;
    /** Take the value the form starts from now as known. */
    readonly keep: (path: readonly string[]) => void;
}

/** One value of a form: one of its fields, or a value inside one. */
export class FormField<Value> {
    /** The path to the value, the field's name first. */
    readonly path: readonly string[];
    /** Where the value is. */
    readonly #lens: FormLens<Value>;
    /** The form it belongs to. */
    readonly #form: FormState;

    /** Edit the value at a path of a form. */
    constructor(form: FormState, path: readonly string[], lens: FormLens<Value>) {
        this.path = path;
        this.#form = form;
        this.#lens = lens;
    }

    /** Read the value the form has now: the edited one, else the starting one. */
    value(): Value {
        return this.#lens.value();
    }

    /** Edit the value. */
    set(value: Value): void {
        this.#lens.write(value);
    }

    /** Settle on the value, as the focus leaves it, which shows why it is refused. */
    settle(): void {
        this.#form.settle(this.path);
    }

    /** Read why the value or one inside it is refused, in the person's language. */
    problem(): string | undefined {
        return this.#form.problem(this.path);
    }

    /** Report whether a person edited the value. */
    isEdited(): boolean {
        const value = this.#lens.value();

        return !this.#lens.start().some((start) => isSame(start, value));
    }

    /** Read what someone else set the value to as a person edits it, absent for nothing. */
    conflict(): FormConflict<Value> | undefined {
        // leave what the person set or already knew
        const known = [this.#lens.value(), ...this.#lens.known()];
        const [value] = this.#lens
            .start()
            .filter((start) => !known.some((each) => isSame(each, start)))
            .map((start) => ({ value: start }));

        return value;
    }

    /** Keep the person's value over someone else's. */
    keep(): void {
        this.#form.keep(this.path);
    }

    /** Take someone else's value over the person's. */
    take(): void {
        for (const start of this.#lens.start()) {
            this.#lens.write(start);
        }
    }

    /** Find a value inside an object. */
    field<Key extends keyof Value & string>(key: Key): FormField<Value[Key]> {
        const lens = this.#lens;

        return new FormField(this.#form, [...this.path, key], {
            value: () => lens.value()[key],
            start: () => lens.start().map((each) => each[key]),
            known: () => lens.known().map((each) => each[key]),
            write: (value) => lens.write({ ...lens.value(), [key]: value }),
        });
    }

    /** Find the item of a list at a place it has, the last one there once the list is shorter. */
    item<Item>(this: FormField<readonly Item[]>, index: number): FormField<Item> {
        // keep the last item of a place the list loses
        const lens = this.#lens;
        const at = (list: readonly Item[]): readonly Item[] => list.slice(index, index + 1);
        let last = at(lens.value());
        const read = (): Item => {
            // read what is there now, else what was
            const now = at(lens.value());
            last = now.length === 0 ? last : now;
            const [item] = last;
            if (item === undefined) {
                throw new RangeError(`list item '${String(index)}' is missing`);
            }

            return item;
        };

        return new FormField<Item>(this.#form, [...this.path, String(index)], {
            value: read,
            start: () => lens.start().flatMap(at),
            known: () => lens.known().flatMap(at),
            write: (value) => lens.write(lens.value().with(index, value)),
        });
    }

    /** Insert an item into a list, at its end by default. */
    insert<Item>(
        this: FormField<readonly Item[]>,
        item: Item,
        index: number = this.value().length,
    ): void {
        this.set(this.value().toSpliced(index, 0, item));
    }

    /** Remove an item from a list. */
    remove<Item>(this: FormField<readonly Item[]>, index: number): void {
        this.set(this.value().toSpliced(index, 1));
    }

    /** Move an item of a list to another place. */
    move<Item>(this: FormField<readonly Item[]>, from: number, to: number): void {
        const moved = this.value().slice(from, from + 1);
        this.set(
            this.value()
                .toSpliced(from, 1)
                .toSpliced(to, 0, ...moved),
        );
    }
}

/** The draft of some values, checked by a schema and submitted as a whole. */
export class FormControl<Values extends object> {
    /** Where the last submission stands. */
    readonly status: Accessor<FormStatus>;
    /** Why the last submission failed, absent while it has not. */
    readonly error: Accessor<unknown>;
    /** Why each shown value is refused, in the person's language, by its JSON Pointer. */
    readonly problems: Accessor<ReadonlyMap<string, string>>;
    /** What the form edits. */
    readonly #options: FormOptions<Values>;
    /** The language problems are written in. */
    readonly #locale: Localization;
    /** The edited values. */
    #edits: Partial<Values> = {};
    /** The values the edited ones started from. */
    #base: Partial<Values> = {};
    /** The values of the submissions since the last one saved, and of that one. */
    #sent: readonly Partial<Values>[] = [];
    /** The pointers whose refusals show. */
    readonly #settled = new Set<string>();
    /** The refusals of the server and of the owner, until the values they are of change. */
    readonly #refused = new Map<string, string>();
    /** Whether every refusal shows, as it does once submitted. */
    #isSubmitted = false;
    /** How often the form submitted, which tells a late answer. */
    #submissions = 0;
    /** The count of changes, which readers follow. */
    readonly #revision: Accessor<number>;
    /** Count a change. */
    readonly #setRevision: Setter<number>;
    /** Set where the last submission stands. */
    readonly #setStatus: Setter<FormStatus>;
    /** Set why the last submission failed. */
    readonly #setError: (error: unknown) => void;
    /** Set the shown refusals. */
    readonly #setProblems: (problems: ReadonlyMap<string, string>) => void;

    /** Where the browser keeps the edits, absent for nowhere. */
    readonly #stored: StoredItem | undefined;

    /** Edit some values. */
    constructor(options: FormOptions<Values>, locale: Localization, stored?: StoredItem) {
        // start without edits, refusals and submissions
        this.#options = options;
        this.#locale = locale;
        this.#stored = stored;
        const [revision, setRevision] = createSignal(0);
        this.#revision = revision;
        this.#setRevision = setRevision;
        const [status, setStatus] = createSignal<FormStatus>("idle");
        this.status = status;
        this.#setStatus = setStatus;
        const [failure, setFailure] = createSignal<{ readonly error: unknown }>();
        this.error = () => failure()?.error;
        this.#setError = (error) => setFailure(error === undefined ? undefined : { error });
        const [problems, setProblems] = createSignal<ReadonlyMap<string, string>>(new Map());
        this.problems = problems;
        this.#setProblems = (next) => setProblems(next);
    }

    /** Read the values as edited, cleared ones left out. */
    values(): Values {
        this.#revision();

        return this.#compose();
    }

    /** Find one of the form's fields. */
    field<Name extends keyof Values & string>(name: Name): FormField<Values[Name]> {
        const state: FormState = {
            settle: (path) => this.#settle(path),
            problem: (path) => firstUnder(this.problems(), jsonPointer(path)),
            keep: (path) => this.#keep(path),
        };

        return new FormField(state, [name], {
            value: () => this.values()[name],
            start: () => [this.#options.values()[name]],
            known: () => {
                this.#revision();
                const start = this.#options.values();

                return [{}, ...this.#sent].map(
                    (sent) => ({ ...start, ...this.#base, ...sent })[name],
                );
            },
            write: (value) => this.#write(name, value),
        });
    }

    /** Submit the values where nothing refuses them, and report whether they went. */
    submit(): boolean {
        // show every refusal from the first submit on
        this.#isSubmitted = true;

        return this.#send();
    }

    /** Refuse values by their fields' names, as a server does once they are submitted. */
    refuse(problems: Readonly<Partial<Record<keyof Values & string, string>>>): void {
        for (const [name, problem] of Object.entries<string | undefined>(problems)) {
            if (problem !== undefined) {
                this.#refused.set(jsonPointer([name]), problem);
            }
        }
        this.#check();
    }

    /** Report whether a person edited a value. */
    isEdited(): boolean {
        this.#revision();

        return Object.keys(this.#edits).length > 0;
    }

    /** Drop the edits and refusals, back to the starting values. */
    reset(): void {
        // start over, leaving a late answer alone
        this.#edits = {};
        this.#base = {};
        this.#sent = [];
        this.#settled.clear();
        this.#refused.clear();
        this.#isSubmitted = false;
        this.#submissions += 1;
        this.#store();
        this.#setRevision((count) => count + 1);
        this.#setProblems(new Map());
        this.#setError(undefined);
        this.#setStatus("idle");
    }

    /** Take the edits a browser kept, leaving those of another shape. */
    restore(): void {
        // read them as their schema does
        const text = this.#stored?.read();
        const kept = text === undefined ? undefined : readJson(text);
        const shape = this.#options.draft?.schema;
        const parsed =
            shape === undefined
                ? undefined
                : schema.object({ edits: shape, base: shape }).safeParse(kept);
        if (parsed?.success === true) {
            this.#edits = parsed.data.edits;
            this.#base = parsed.data.base;
            this.#setRevision((count) => count + 1);
        }
    }

    /** Keep the edits in the browser, or nothing without any. */
    #store(): void {
        if (Object.keys(this.#edits).length === 0) {
            this.#stored?.write("");
        } else {
            this.#stored?.write(JSON.stringify({ edits: this.#edits, base: this.#base }));
        }
    }

    /** Edit a field's value, and submit it in change mode. */
    #write<Name extends keyof Values & string>(name: Name, value: Values[Name]): void {
        // remember what an edit starts from, and forget one that comes back to it
        const start = this.#options.values()[name];
        if (!(name in this.#edits)) {
            this.#base = { ...this.#base, [name]: start };
        }
        this.#edits = { ...this.#edits, [name]: value };
        if (isSame(value, start)) {
            this.#drop([name]);
        }

        // forget what refused the value before, then check or submit it
        const pointer = jsonPointer([name]);
        for (const refused of this.#refused.keys()) {
            if (isUnder(refused, pointer)) {
                this.#refused.delete(refused);
            }
        }
        this.#setRevision((count) => count + 1);
        this.#store();
        if (this.#options.mode === "change") {
            this.#settled.add(pointer);
            this.#send();
        } else {
            this.#check();
        }
    }

    /** Show why a value is refused, and submit it in settle mode. */
    #settle(path: readonly string[]): void {
        this.#settled.add(jsonPointer(path));
        if (this.#options.mode === "settle" && Object.keys(this.#edits).length > 0) {
            this.#send();
        } else {
            this.#check();
        }
    }

    /** Take the starting value of a path's field as known. */
    #keep(path: readonly string[]): void {
        // leave the other fields as they are
        const start = this.#options.values();
        const others = Object.keys(start).filter((name) => name !== path[0]);
        this.#base = { ...this.#base, ...without<Partial<Values>>(start, others) };
        this.#setRevision((count) => count + 1);
    }

    /** Forget the edits of fields. */
    #drop(names: readonly string[]): void {
        this.#edits = without(this.#edits, names);
        this.#base = without(this.#base, names);
        this.#sent = this.#sent.map((each) => without(each, names));
    }

    /** Read the starting values with the edits over them, cleared ones left out. */
    #compose(): Values {
        const composed = { ...this.#options.values(), ...this.#edits };
        for (const [key, value] of Object.entries(composed)) {
            if (value === undefined) {
                Reflect.deleteProperty(composed, key);
            }
        }

        return composed;
    }

    /** Check the values, showing the refusals of settled ones until the first submit. */
    #check(): boolean {
        // refuse the issues of the form's fields, leaving the rest to the owner
        const held = this.#compose();
        const fields = this.#options.values();
        const parsed = this.#options.schema?.safeParse(held, { error: () => UNWORDED });
        const issues = (parsed === undefined || parsed.success ? [] : parsed.error.issues)
            .filter((issue) => String(issue.path[0] ?? "") in fields)
            .flatMap((issue) => closest(issue, held));

        // show the first refusal at each pointer of a settled value, or of every one once submitted
        const refused = new Map(this.#refused);
        for (const issue of issues) {
            const pointer = jsonPointer(issue.path);
            const isShown =
                this.#isSubmitted ||
                [...this.#settled].some((settled) => isUnder(pointer, settled));
            if (isShown && !refused.has(pointer)) {
                const value = valueAt(held, issue.path);
                refused.set(
                    pointer,
                    issue.message === UNWORDED
                        ? explain(issue, value, this.#locale)
                        : issue.message,
                );
            }
        }
        this.#setProblems(refused);

        return issues.length === 0;
    }

    /** Submit the values, following the answer while no later submission took over. */
    #send(): boolean {
        // leave refused values
        this.#refused.clear();
        if (!this.#check()) {
            return false;
        }
        this.#submissions += 1;
        const submission = this.#submissions;
        const sent = { ...this.#edits };
        this.#sent = [...this.#sent, sent];
        this.#setError(undefined);
        this.#setStatus("pending");

        // follow a submission, a promise, or nothing once it is done
        try {
            const answer = this.#options.submit(this.#compose(), sent);
            const settled =
                answer === undefined || answer instanceof Promise ? answer : answer.confirmed;
            if (settled === undefined) {
                this.#save(sent);
            } else {
                settled.then(
                    () => submission === this.#submissions && this.#save(sent),
                    (error: unknown) => submission === this.#submissions && this.#fail(error),
                );
            }
        } catch (error) {
            this.#fail(error);
        }

        return true;
    }

    /** Forget the edits a submission saved, keeping those made since. */
    #save(sent: Partial<Values>): void {
        // keep an edit that differs from what was sent
        this.#sent = [sent];
        const edits = new Map<string, unknown>(Object.entries(this.#edits));
        this.#drop(
            Object.entries<unknown>(sent).flatMap(([name, value]) =>
                isSame(edits.get(name), value) ? [name] : [],
            ),
        );
        this.#setRevision((count) => count + 1);
        this.#store();
        this.#setStatus("saved");
    }

    /** Show why a submission failed, at the values it refused where it tells. */
    #fail(error: unknown): void {
        // show the refusals at their values, and the rest as the form's error
        if (ObjectError.is(error, "INPUT_INVALID")) {
            for (const issue of error.data.issues) {
                this.#refused.set(
                    jsonPointer(issue.path),
                    this.#locale.render(t`Enter a valid value`),
                );
            }
        }
        this.#check();
        this.#setError(error);
        this.#setStatus("failed");
    }
}

/** Edit some values, as a form does. */
export function useForm<Values extends object>(options: FormOptions<Values>): FormControl<Values> {
    // keep the edits where the form has a draft
    const locale = useLocale();
    const { draft } = options;
    const environment = useEnvironment();
    const telemetry = TelemetryScope.of(import.meta.destack.package, environment.telemetry);
    const stored =
        draft === undefined
            ? undefined
            : new StoredItem(`${DRAFT_KEY}${draft.name}`, environment, telemetry);

    // take the edits once the page is the browser's
    const form = new FormControl(options, locale, stored);
    onSettled(() => form.restore());

    return form;
}

/** Read text as JSON, undefined for text that is none. */
function readJson(text: string): unknown {
    try {
        return JSON.parse(text);
    } catch {
        return undefined;
    }
}

/** Leave some of an object's entries out. */
function without<Values extends object>(values: Values, names: readonly string[]): Values {
    const kept = { ...values };
    for (const name of names) {
        Reflect.deleteProperty(kept, name);
    }

    return kept;
}

/** Report whether a pointer is at another one or inside it. */
function isUnder(pointer: string, outer: string): boolean {
    return pointer === outer || pointer.startsWith(`${outer}/`);
}

/** Write a value's path as a JSON Pointer (RFC 6901), escaping `~` and `/`. */
function jsonPointer(path: readonly PropertyKey[]): string {
    return path
        .map((key) => `/${String(key).replaceAll("~", "~0").replaceAll("/", "~1")}`)
        .join("");
}

/** Report whether a value has entries by key: an object or a list. */
function isContainer(value: unknown): value is Readonly<Record<string, unknown>> {
    return typeof value === "object" && value !== null;
}

/** Read the issues of the union member closest to a kept value. */
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

/** Report whether two values are the same, entry by entry. */
function isSame(left: unknown, right: unknown): boolean {
    // compare what has no entries of its own
    if (Object.is(left, right)) {
        return true;
    } else if (!isPlain(left) || !isPlain(right)) {
        return false;
    }

    // compare the entries of two lists or two objects
    const keys = new Set([...Object.keys(left), ...Object.keys(right)]);

    return (
        Array.isArray(left) === Array.isArray(right) &&
        [...keys].every((key) => isSame(left[key], right[key]))
    );
}

/** Report whether a value is a list or a plain object, which its entries tell apart. */
function isPlain(value: unknown): value is Readonly<Record<string, unknown>> {
    return (
        isContainer(value) &&
        (Array.isArray(value) || Object.getPrototypeOf(value) === Object.prototype)
    );
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
    // choices, such as a yes
    else if (problem.code === "invalid_value" && problem.values.every((each) => each === true)) {
        return locale.render(t`Check this to go on`);
    } else if (problem.code === "invalid_value" && (value === undefined || value === null)) {
        return locale.render(t`Choose an option`);
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
