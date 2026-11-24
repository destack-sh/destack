import { schema } from "@destack/schema";
import { present } from "@destack/schema";
import { expect, test } from "@destack/test";
import { createSignal, flush, Repeat, useForm } from "@destack/view";
import { render, wait } from "@destack/view/test";
import { Checkbox } from "../checkbox/index.ts";
import { Field, FieldConflict, FieldError, FieldLabel } from "../field/index.ts";
import { Input } from "../input/index.ts";
import { NumberField } from "../number-field/index.ts";
import { Switch } from "../switch/index.ts";
import { Form, FormError, FormReset, FormSubmit } from "./index.ts";

/** A trip. */
interface Trip {
    /** What it is called. */
    readonly title: string;
    /** How many come. */
    readonly guests: number | undefined;
    /** Whether it is insured. */
    readonly isInsured: boolean;
    /** Whether the person agrees. */
    readonly isAgreed: boolean;
}

/** Draw a trip's form, settling each submission as the test says. */
function draw() {
    const [start, setStart] = createSignal<Trip>({
        title: "Lake",
        guests: 2,
        isInsured: true,
        isAgreed: false,
    });
    const sent: Trip[] = [];
    let answer = Promise.withResolvers<void>();
    const { container } = render(() => {
        const form = useForm<Trip>({
            values: start,
            schema: schema.object({
                title: schema.string().min(1),
                guests: schema.int(),
                isInsured: schema.boolean(),
                isAgreed: schema.literal(true),
            }),
            submit: (values) => {
                sent.push(values);
                answer = Promise.withResolvers();

                return answer.promise;
            },
        });

        return (
            <Form form={form}>
                <Field field={form.field("title")}>
                    <FieldLabel>Title</FieldLabel>
                    <Input />
                    <FieldConflict />
                    <FieldError />
                </Field>
                <Field field={form.field("guests")}>
                    <NumberField aria-label="Guests" />
                </Field>
                <Field field={form.field("isInsured")}>
                    <Switch />
                </Field>
                <Field field={form.field("isAgreed")}>
                    <Checkbox />
                    <FieldError />
                </Field>
                <FormError />
                <FormSubmit>Book</FormSubmit>
                <FormReset>Start over</FormReset>
            </Form>
        );
    });
    const find = (selector: string) =>
        present(container.querySelector<HTMLElement>(selector), selector);
    const title = present(container.querySelector("input"), "the title");
    const type = (text: string) => {
        title.value = text;
        title.dispatchEvent(new InputEvent("input", { bubbles: true }));
        flush();
    };
    const submit = () => {
        find("form").dispatchEvent(new SubmitEvent("submit", { bubbles: true, cancelable: true }));
        flush();
    };
    const settle = async (run: (answer: PromiseWithResolvers<void>) => void) => {
        run(answer);
        await Promise.resolve();
        await Promise.resolve();
        flush();
    };
    const errors = () =>
        [...container.querySelectorAll("[role=alert]")].map((each) => each.textContent);

    return { container, find, title, type, submit, settle, errors, sent, setStart };
}

test("edit the values through the controls, and submit them", async () => {
    const { find, title, type, submit, settle, sent, container } = draw();
    const before = [title.value, find("[role=switch]").getAttribute("aria-checked")];
    type("Sea");
    find("[role=switch]").click();
    find("[role=checkbox]").click();
    flush();
    submit();
    const busy = find("[data-slot=form-submit]").getAttribute("aria-busy");
    await settle((answer) => answer.resolve());

    expect([
        before,
        sent,
        busy,
        find("form").getAttribute("data-status"),
        container.isConnected,
    ]).toEqual([
        ["Lake", "true"],
        [{ title: "Sea", guests: 2, isInsured: false, isAgreed: true }],
        "true",
        "saved",
        true,
    ]);
});

test("show what is refused, and focus the first control", () => {
    const { type, submit, errors, sent, title } = draw();
    type("");
    const typing = errors();
    submit();

    expect([
        typing,
        errors(),
        sent,
        document.activeElement === title,
        title.getAttribute("aria-invalid"),
    ]).toEqual([[], ["Enter a value", "Check this to go on"], [], true, "true"]);
});

test("tell that a submission failed, and drop the edits on reset", async () => {
    const { find, type, submit, settle, errors, title } = draw();
    type("Sea");
    find("[role=checkbox]").click();
    flush();
    submit();
    await settle((answer) => answer.reject(new Error("offline")));
    const failed = errors();
    find("form").dispatchEvent(new Event("reset", { bubbles: true }));
    flush();

    expect([failed, errors(), title.value]).toEqual([
        ["This could not be saved. Try again."],
        [],
        "Lake",
    ]);
});

test("offer what someone else set meanwhile", () => {
    const { find, type, setStart, title, container } = draw();
    type("Sea");
    setStart((current) => ({ ...current, title: "River" }));
    flush();
    const offered = find("[data-slot=field-conflict]").textContent;
    find("[data-slot=field-conflict] button").click();
    flush();

    expect([offered, title.value, container.querySelector("[data-slot=field-conflict]")]).toEqual([
        "Changed meanwhile: RiverUse thisKeep mine",
        "River",
        null,
    ]);
});

test("keep a control its caller sets", () => {
    const { container } = render(() => {
        const form = useForm({ values: () => ({ title: "Lake" }), submit: () => undefined });

        return (
            <Field field={form.field("title")}>
                <Input value="mine" />
            </Field>
        );
    });

    expect(container.querySelector("input")?.value).toBe("mine");
});

test("edit a list as it grows and shrinks", () => {
    const { container } = render(() => {
        const form = useForm<{ readonly names: readonly string[] }>({
            values: () => ({ names: ["Ada", "Grace"] }),
            submit: () => undefined,
        });
        const names = form.field("names");

        return (
            <>
                <Repeat count={names.value().length}>
                    {(index) => (
                        <Field field={names.item(index)}>
                            <Input />
                        </Field>
                    )}
                </Repeat>
                <button type="button" onClick={() => names.insert("Barbara")} />
                <button type="button" onClick={() => names.remove(0)} />
            </>
        );
    });
    const shown = () => [...container.querySelectorAll("input")].map((input) => input.value);
    const press = (index: number) => {
        container.querySelectorAll("button")[index]?.click();
        flush();

        return shown();
    };

    expect([shown(), press(0), press(1), press(1), press(1)]).toEqual([
        ["Ada", "Grace"],
        ["Ada", "Grace", "Barbara"],
        ["Grace", "Barbara"],
        ["Barbara"],
        [],
    ]);
});

test("ask for a number until there is one", async () => {
    const { container } = render(() => (
        <Field>
            <NumberField aria-label="Guests" />
            <FieldError />
        </Field>
    ));
    const input = present(container.querySelector("input"), "the input");
    const settle = async (text: string) => {
        input.value = text;
        input.dispatchEvent(new InputEvent("input", { bubbles: true }));
        flush();
        input.dispatchEvent(new FocusEvent("blur"));
        flush();
        await wait(0);
        flush();

        return container.querySelector("[role=alert]")?.textContent;
    };

    expect([await settle("two"), await settle("2")]).toEqual(["Enter a number", undefined]);
});
