import { schema, toJsonSchema, type JsonObject } from "@destack/schema";
import { expect, test } from "@destack/test";
import { flush } from "@destack/view";
import { press, render } from "@destack/view/test";
import { Input } from "../input/index.ts";
import { SchemaForm, schemaFields } from "./index.ts";

/** The first strong isolate MessageFormat 2 places around a placeholder's value. */
const OPEN = String.fromCodePoint(0x2068);

/** The pop directional isolate closing a placeholder's value. */
const CLOSE = String.fromCodePoint(0x2069);

/** The isolates around placeholder values. */
const ISOLATES = new RegExp(`[${OPEN}${CLOSE}]`, "gu");

/** The input of a command filing a task. */
const TASK_INPUT = toJsonSchema(
    schema.object({
        title: schema.string().min(1).meta({ title: "Title" }),
        priority: schema.enum(["low", "high"]).meta({ title: "Priority" }).exactOptional(),
        done: schema.boolean().exactOptional(),
        estimate: schema.int().exactOptional(),
    }),
);

/** Render a schema form, recording what it submits. */
function drawForm(described: JsonObject): {
    /** The rendered form's container. */
    readonly container: HTMLElement;
    /** The objects it submitted, in order. */
    readonly submitted: JsonObject[];
} {
    const submitted: JsonObject[] = [];
    const { container } = render(() => (
        <SchemaForm schema={described} onSubmit={(value) => submitted.push(value)} />
    ));

    return { container, submitted };
}

/** Submit the form in a container. */
function submit(container: HTMLElement): void {
    container.querySelector("form")?.dispatchEvent(new Event("submit", { cancelable: true }));
    flush();
}

/** Type a value into the input labelled by a text. */
function fill(container: HTMLElement, label: string, value: string): void {
    const input = labelled(container, label);
    if (input instanceof HTMLInputElement || input instanceof HTMLSelectElement) {
        input.value = value;
        input.dispatchEvent(
            new Event(input instanceof HTMLSelectElement ? "change" : "input", { bubbles: true }),
        );
    }
    flush();
}

/** Click the checkbox or button named by a text. */
function click(container: HTMLElement, name: string): void {
    const button = [...container.querySelectorAll("button")].find(
        (found) =>
            (found.getAttribute("aria-label") ?? found.textContent).replaceAll(ISOLATES, "") ===
            name,
    );
    const target = button ?? labelled(container, name);
    if (target instanceof HTMLElement) {
        target.click();
    }
    flush();
}

/** Find the control a label with a text points at. */
function labelled(container: HTMLElement, label: string): Element | null {
    const found = [...container.querySelectorAll("label")].find(
        (element) => element.textContent === label,
    );

    return container.ownerDocument.getElementById(found?.htmlFor ?? "");
}

/** Find the group of controls a label with a text names. */
function grouped(container: HTMLElement, label: string): Element | null {
    const found = [...container.querySelectorAll("label")].find(
        (element) => element.textContent === label,
    );

    return container.querySelector(`[aria-labelledby="${found?.id ?? ""}"]`);
}

/** Type digits or letters into the segments of a group by their parts. */
function fillSegments(
    group: Element | null | undefined,
    keys: Readonly<Record<string, string>>,
): void {
    for (const [kind, typed] of Object.entries(keys)) {
        const segment = group?.querySelector<HTMLElement>(`[data-segment=${kind}]`);
        segment?.focus();
        for (const key of segment === null || segment === undefined ? "" : typed) {
            press(key);
            flush();
        }
    }
}

/** List the refusals a container shows, in order. */
function refusals(container: HTMLElement): (string | null)[] {
    return [...container.querySelectorAll("[data-slot=field-error]")].map(
        (element) => element.textContent,
    );
}

test("list a field per property with the control its type takes", () => {
    expect(
        schemaFields(TASK_INPUT).map((field) => [
            field.name,
            field.title,
            field.control,
            field.isRequired,
        ]),
    ).toEqual([
        ["title", "Title", "text", true],
        ["priority", "Priority", "choice", false],
        ["done", "done", "boolean", false],
        ["estimate", "estimate", "number", false],
    ]);
});

test("refuse a missing required field on submit, then submit the kept values", () => {
    const { container, submitted } = drawForm(TASK_INPUT);
    submit(container);
    const refused = refusals(container);
    fill(container, "Title", "Milk");
    fill(container, "estimate", "3");
    submit(container);
    expect([refused, submitted]).toEqual([["Enter a value"], [{ title: "Milk", estimate: 3 }]]);
});

test("start fields from their constants and null, and submit them untouched", () => {
    const { container, submitted } = drawForm(
        toJsonSchema(
            schema.object({
                kind: schema.literal("list"),
                note: schema.string().nullable().meta({ title: "Note" }),
                isShared: schema.boolean().meta({ title: "Shared" }),
            }),
        ),
    );
    submit(container);
    expect(submitted).toEqual([{ kind: "list", note: null, isShared: false }]);
});

test("wire each string format and number bound to its native input, the form validating none itself", () => {
    const { container } = render(() => (
        <SchemaForm
            schema={toJsonSchema(
                schema.object({
                    email: schema.email().meta({ title: "Email" }),
                    site: schema.url().meta({ title: "Site" }).exactOptional(),
                    count: schema.int().min(0).max(9).meta({ title: "Count" }).exactOptional(),
                    ratio: schema
                        .number()
                        .gt(0)
                        .multipleOf(0.5)
                        .meta({ title: "Ratio" })
                        .exactOptional(),
                }),
            )}
            submit="Save"
            onSubmit={() => undefined}
        />
    ));
    const inputs = [...container.querySelectorAll("input")].map((input) =>
        ["type", "min", "max", "step", "inputmode", "aria-required"].flatMap((name) => {
            const value = input.getAttribute(name);

            return value === null ? [] : [`${name}=${value}`];
        }),
    );
    expect([container.querySelector("form")?.hasAttribute("novalidate"), inputs]).toEqual([
        true,
        [
            ["type=email", "aria-required=true"],
            ["type=url"],
            ["type=number", "min=0", "max=9", "step=1", "inputmode=numeric"],
            ["type=number", "min=0", "step=0.5", "inputmode=decimal"],
        ],
    ]);
});

test("type a date and a date and time into their fields, submitting the date as written and the moment in UTC", () => {
    const { container, submitted } = drawForm(
        toJsonSchema(
            schema.object({
                email: schema.email().meta({ title: "Email" }),
                day: schema.iso.date().meta({ title: "Day" }),
                at: schema.iso.datetime().meta({ title: "At" }),
            }),
        ),
    );
    fill(container, "Email", "milk");
    const refused = refusals(container);
    fill(container, "Email", "milk@example.com");
    fillSegments(grouped(container, "Day"), { month: "10", day: "07", year: "2026" });
    const at = grouped(container, "At");
    fillSegments(at?.querySelector("[data-slot=date-picker]"), {
        month: "10",
        day: "07",
        year: "2026",
    });
    fillSegments(at?.querySelector("[data-slot=time-field]"), {
        hour: "09",
        minute: "30",
        dayPeriod: "a",
    });
    submit(container);

    expect([
        refused,
        [...(at?.querySelectorAll("[role=group]") ?? [])].map((group) =>
            group.getAttribute("aria-label"),
        ),
        submitted,
    ]).toEqual([
        ["Enter an email address"],
        ["Date", null, "Time"],
        [
            {
                email: "milk@example.com",
                day: "2026-10-07",
                at: new Date("2026-10-07T09:30").toISOString(),
            },
        ],
    ]);
});

test("add and remove items of a list, and choose several values of an enumeration", () => {
    const { container, submitted } = drawForm(
        toJsonSchema(
            schema.object({
                steps: schema.array(schema.string().min(1)).meta({ title: "Step" }),
                colors: schema
                    .array(schema.enum(["red", "green", "blue"]))
                    .meta({ title: "Colors" }),
            }),
        ),
    );
    click(container, "Add Step");
    click(container, "Add Step");
    click(container, "Add Step");
    fill(container, "Step 1", "Boil");
    fill(container, "Step 2", "");
    fill(container, "Step 3", "Serve");
    submit(container);
    const refused = refusals(container);
    click(container, "Remove Step 2");
    click(container, "blue");
    click(container, "red");
    submit(container);
    expect([refused, submitted]).toEqual([
        ["Enter a value"],
        [{ steps: ["Boil", "Serve"], colors: ["red", "blue"] }],
    ]);
});

test("fill a nested object in a fieldset, refusing its own field", () => {
    const { container, submitted } = drawForm(
        toJsonSchema(
            schema.object({
                address: schema
                    .object({
                        city: schema.string().min(1).meta({ title: "City" }),
                        geo: schema
                            .object({
                                latitude: schema.number().max(90).meta({ title: "Latitude" }),
                            })
                            .meta({ title: "Position" }),
                    })
                    .meta({ title: "Address" }),
            }),
        ),
    );
    fill(container, "Latitude", "91");
    const refused = refusals(container);
    const legends = [...container.querySelectorAll("legend")].map((legend) => legend.textContent);
    fill(container, "Latitude", "48.2");
    fill(container, "City", "Vienna");
    submit(container);
    expect([legends, refused, submitted]).toEqual([
        ["Address", "Position"],
        [`Enter ${OPEN}90${CLOSE} or less`],
        [{ address: { city: "Vienna", geo: { latitude: 48.2 } } }],
    ]);
});

test("choose the kind of a union of objects, then fill its fields and submit its constant", () => {
    const { container, submitted } = drawForm(
        toJsonSchema(
            schema.object({
                shape: schema
                    .discriminatedUnion("kind", [
                        schema.object({
                            kind: schema.literal("circle"),
                            radius: schema.number().meta({ title: "Radius" }),
                        }),
                        schema.object({
                            kind: schema.literal("square"),
                            side: schema.number().meta({ title: "Side" }),
                        }),
                    ])
                    .meta({ title: "Shape" }),
            }),
        ),
    );
    const kinds = [...(labelled(container, "Kind")?.querySelectorAll("option") ?? [])].map(
        (option) => option.textContent,
    );
    submit(container);
    const refused = refusals(container);
    fill(container, "Kind", "1");
    fill(container, "Side", "2");
    submit(container);
    expect([kinds, refused, submitted]).toEqual([
        ["circle", "square", "Choose"],
        ["Enter a value"],
        [{ shape: { kind: "square", side: 2 } }],
    ]);
});

test("render a field through renderField, keeping the form's controls for the others", () => {
    const submitted: JsonObject[] = [];
    const { container } = render(() => (
        <SchemaForm
            schema={toJsonSchema(
                schema.object({
                    title: schema.string().meta({ title: "Title" }),
                    ownerId: schema.string().meta({ title: "Owner" }),
                }),
            )}
            renderField={(field, held) =>
                field.name === "ownerId" ? (
                    <Input
                        aria-label="Pick owner"
                        onInput={(event) => held.set(event.currentTarget.value)}
                    />
                ) : undefined
            }
            onSubmit={(value) => submitted.push(value)}
        />
    ));
    const picker = container.querySelector("[aria-label='Pick owner']");
    if (picker instanceof HTMLInputElement) {
        picker.value = "user-1";
        picker.dispatchEvent(new Event("input", { bubbles: true }));
    }
    fill(container, "Title", "Milk");
    submit(container);
    expect([
        [...container.querySelectorAll("label")].map((label) => label.textContent),
        submitted,
    ]).toEqual([["Title"], [{ title: "Milk", ownerId: "user-1" }]]);
});

/** A node of a tree: a name and child nodes. */
type TreeNode = { readonly name: string; readonly children?: readonly TreeNode[] };

/** A node of a tree, referring back to itself through its children. */
const NODE: schema.Schema<TreeNode> = schema.object({
    name: schema.string().min(1).meta({ title: "Name" }),
    children: schema
        .array(schema.lazy(() => NODE))
        .meta({ title: "Child" })
        .exactOptional(),
});

/** The input of a command taking a tree. */
const TREE = toJsonSchema(schema.object({ tree: NODE.meta({ title: "Tree" }) }));

/** Type a value into an input, as the person does. */
function typeInto(input: HTMLInputElement | undefined, value: string): void {
    if (input !== undefined) {
        input.value = value;
        input.dispatchEvent(new Event("input", { bubbles: true }));
    }
    flush();
}

test("expand a recursive tree only where its nodes exist, adding a child node by node", () => {
    const { container, submitted } = drawForm(TREE);
    const before = [...container.querySelectorAll("legend")].map((legend) => legend.textContent);
    click(container, "Add Child");
    const inputs = [...container.querySelectorAll("input")];
    typeInto(inputs[0], "Root");
    typeInto(inputs[1], "Leaf");
    submit(container);
    expect([
        before,
        [...container.querySelectorAll("legend")].map((legend) => legend.textContent),
        submitted,
    ]).toEqual([
        ["Tree", "Child"],
        ["Tree", "Child", "Child 1", "Child"],
        [{ tree: { name: "Root", children: [{ name: "Leaf" }] } }],
    ]);
});

test("title a referenced field by the annotations beside its reference, following a reference to a reference", () => {
    // reach a title through two references, and override it beside the first
    const described: JsonObject = {
        type: "object",
        $defs: { name: { $ref: "#/$defs/text" }, text: { type: "string", title: "Text" } },
        properties: {
            first: { $ref: "#/$defs/name", title: "First name" },
            last: { $ref: "#/$defs/name" },
        },
    };
    expect(
        schemaFields(described).map((field) => [field.name, field.title, field.control]),
    ).toEqual([
        ["first", "First name", "text"],
        ["last", "Text", "text"],
    ]);
});

test("refuse a reference that resolves nowhere, and a schema that requires a value of itself", () => {
    const loop: JsonObject = {
        type: "object",
        $defs: {
            link: {
                type: "object",
                properties: { next: { $ref: "#/$defs/link" } },
                required: ["next"],
            },
        },
        properties: { first: { $ref: "#/$defs/link" } },
        required: ["first"],
    };
    expect(() => schemaFields({ properties: { owner: { $ref: "#/$defs/person" } } })).toThrow(
        new TypeError("schema reference #/$defs/person does not resolve"),
    );
    expect(() => schemaFields({ properties: { owner: { $ref: "other.json#/person" } } })).toThrow(
        new TypeError(
            "schema reference other.json#/person points outside the document's definitions",
        ),
    );
    expect(() => schemaFields(loop)[0]?.start).toThrow(
        new TypeError("schema #/$defs/link requires a value of itself"),
    );
});
