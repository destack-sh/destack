import { none } from "@destack/access";
import { type CallInput, defineObject, field } from "@destack/object";
import type { Submission } from "@destack/object/client";
import { schema } from "@destack/schema";
import { Scope } from "@destack/sync";
import { expect, test } from "@destack/test";
import { flush } from "../solid/reactive.ts";
import { draw, wait } from "../test/index.ts";
import { type Form, type FormMode, useForm, useSchemaForm } from "./form.ts";

/** The first strong isolate MessageFormat 2 places around a placeholder's value. */
const OPEN = String.fromCodePoint(0x2068);

/** The pop directional isolate closing a placeholder's value. */
const CLOSE = String.fromCodePoint(0x2069);

/** A note whose title is between one and twenty characters. */
const note = defineObject({
    name: "note",
    plural: "notes",
    scope: Scope.universe.id,
    fields: { title: field.string(schema.string().min(1).max(20)) },
    permissions: { write: none() },
    methods: (method) => ({ update: method.update("write") }),
});

/** The input of a note's update the tests edit. */
type Rename = CallInput<typeof note, "update">;

/** Edit a note's rename in a drawn component, settling each submission as the test says. */
function renameForm(mode: FormMode): {
    /** The form. */
    readonly form: Form<Rename>;
    /** The inputs it submitted, in order. */
    readonly submitted: Rename[];
    /** Confirm or refuse the last submission. */
    readonly settle: (isConfirmed: boolean) => void;
} {
    const submitted: Rename[] = [];
    let settle: ((isConfirmed: boolean) => void) | undefined;
    let form: Form<Rename> | undefined;
    draw(() => {
        form = useForm(note, "update", {
            values: () => ({ id: note.generateId(), title: "Groceries" }),
            submit: (input): Submission<unknown> => {
                submitted.push(input);
                const confirmed = new Promise<void>((resolve, reject) => {
                    settle = (isConfirmed) =>
                        isConfirmed ? resolve() : reject(new Error("refused"));
                });

                return { predicted: Promise.resolve(input), confirmed };
            },
            mode,
        });

        return null;
    });
    if (form === undefined) {
        throw new TypeError("the component did not render");
    }

    return { form, submitted, settle: (isConfirmed) => settle?.(isConfirmed) };
}

test("hold edits until submitted, then submit the edited input and follow its save", async () => {
    const { form, submitted, settle } = renameForm("submit");
    form.field("title").set("Milk");
    flush();
    const held = [submitted.length, form.isEdited(), form.field("title").value()];
    form.submit();
    flush();
    const pending = form.status();
    settle(true);
    await wait(0);
    flush();
    expect([held, submitted.map((input) => input.title), pending, form.status()]).toEqual([
        [0, true, "Milk"],
        ["Milk"],
        "pending",
        "saved",
    ]);
});

test("submit each change at once in change mode", () => {
    const { form, submitted } = renameForm("change");
    form.field("title").set("Milk");
    form.field("title").set("Bread");
    expect(submitted.map((input) => input.title)).toEqual(["Milk", "Bread"]);
});

test("refuse a value the method's input schema refuses, explaining it in the person's language", () => {
    const { form, submitted } = renameForm("change");
    form.field("title").set("");
    flush();
    expect([submitted, form.submit(), form.field("title").problem()]).toEqual([
        [],
        undefined,
        "Enter a value",
    ]);
});

test("mark a submission the server refuses as failed", async () => {
    const { form, settle } = renameForm("change");
    form.field("title").set("Milk");
    settle(false);
    await wait(0);
    flush();
    expect(form.status()).toBe("failed");
});

test("drop the edits and refusals on reset", () => {
    const { form } = renameForm("submit");
    form.field("title").set("");
    flush();
    const refused = form.field("title").problem();
    form.reset();
    flush();
    expect([
        refused,
        form.field("title").problem(),
        form.isEdited(),
        form.field("title").value(),
    ]).toEqual(["Enter a value", undefined, false, "Groceries"]);
});

/** The input of a command filing a task at a place. */
const FILING = schema.object({
    title: schema.string().min(1),
    estimate: schema.int().min(1),
    place: schema.object({ city: schema.string().min(1), floor: schema.int() }),
    shape: schema.union([
        schema.object({ kind: schema.literal("circle"), radius: schema.number() }),
        schema.object({ kind: schema.literal("square"), side: schema.number() }),
    ]),
});

/** Edit a filing in a drawn component that submits nothing. */
function filingForm(): Form<Record<string, unknown>> {
    let form: Form<Record<string, unknown>> | undefined;
    draw(() => {
        form = useSchemaForm<Record<string, unknown>>(FILING, {
            values: () => ({ title: undefined, estimate: undefined, place: {}, shape: undefined }),
            submit: (input) => ({
                predicted: Promise.resolve(input),
                confirmed: Promise.resolve(),
            }),
        });

        return null;
    });
    if (form === undefined) {
        throw new TypeError("the component did not render");
    }

    return form;
}

test("refuse only the edited values until the first submit, then every value by its pointer", () => {
    const form = filingForm();
    form.field("estimate").set(0);
    form.field("place").set({ city: "" });
    flush();
    const edited = [...form.problems()];
    form.submit();
    flush();
    expect([edited, [...form.problems()], form.field("place").problem()]).toEqual([
        [
            ["/estimate", `Enter ${OPEN}1${CLOSE} or more`],
            ["/place/city", "Enter a value"],
        ],
        [
            ["/title", "Enter a value"],
            ["/estimate", `Enter ${OPEN}1${CLOSE} or more`],
            ["/place/city", "Enter a value"],
            ["/place/floor", "Enter a value"],
            ["/shape", "Enter a value"],
        ],
        "Enter a value",
    ]);
});

test("refuse a union's value by the issues of its closest member", () => {
    const form = filingForm();
    form.field("shape").set({ kind: "square", side: "two" });
    flush();
    expect([...form.problems()]).toEqual([["/shape/side", "Enter a number"]]);
});
