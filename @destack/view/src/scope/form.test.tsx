import { none } from "@destack/access";
import { type CallInput, defineObject, field } from "@destack/object";
import type { Submission } from "@destack/object/client";
import { schema } from "@destack/schema";
import { Scope } from "@destack/sync";
import { expect, test } from "@destack/test";
import { flush } from "../solid/reactive.ts";
import { draw, wait } from "../test/index.ts";
import { type Form, type FormMode, useForm } from "./form.ts";

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
