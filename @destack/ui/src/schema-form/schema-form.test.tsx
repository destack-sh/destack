import type { JsonObject } from "@destack/schema";
import { expect, test } from "@destack/test";
import { flush } from "@destack/view";
import { draw, press } from "@destack/view/test";
import { Input } from "../input/index.ts";
import { SchemaForm, schemaFields } from "./index.ts";

/** The first strong isolate MessageFormat 2 places around a placeholder's value. */
const OPEN = String.fromCodePoint(0x2068);

/** The pop directional isolate closing a placeholder's value. */
const CLOSE = String.fromCodePoint(0x2069);

/** The isolates around placeholder values. */
const ISOLATES = new RegExp(`[${OPEN}${CLOSE}]`, "gu");

/** The input of a command filing a task. */
const TASK_INPUT: JsonObject = {
    type: "object",
    properties: {
        title: { type: "string", title: "Title", minLength: 1 },
        priority: { title: "Priority", enum: ["low", "high"] },
        done: { type: "boolean" },
        estimate: { type: "integer" },
    },
    required: ["title"],
};

/** Render a schema form, recording what it submits. */
function drawForm(described: JsonObject): {
    /** The rendered form's container. */
    readonly container: HTMLElement;
    /** The objects it submitted, in order. */
    readonly submitted: JsonObject[];
} {
    const submitted: JsonObject[] = [];
    const container = draw(() => (
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

/** Type digits or letters into the segments of a group by their parts, skipping parts the locale leaves out. */
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

test("start fields from their defaults, constants and null, and submit them untouched", () => {
    const { container, submitted } = drawForm({
        type: "object",
        properties: {
            name: { type: "string", title: "Name", default: "Inbox" },
            priority: { title: "Priority", enum: ["low", "high"], default: "high" },
            kind: { type: "string", const: "list" },
            note: { type: ["string", "null"], title: "Note" },
            isShared: { type: "boolean", title: "Shared" },
        },
        required: ["name", "priority", "kind", "note", "isShared"],
    });
    const shown = [labelled(container, "Name"), labelled(container, "Priority")].map((control) =>
        control instanceof HTMLInputElement || control instanceof HTMLSelectElement
            ? control.value
            : undefined,
    );
    submit(container);
    expect([shown, submitted]).toEqual([
        ["Inbox", "1"],
        [{ name: "Inbox", priority: "high", kind: "list", note: null, isShared: false }],
    ]);
});

test("wire each string format and number bound to its native input, the form validating none itself", () => {
    const container = draw(() => (
        <SchemaForm
            schema={{
                type: "object",
                properties: {
                    email: { type: "string", title: "Email", format: "email" },
                    site: { type: "string", title: "Site", format: "uri" },
                    count: { type: "integer", title: "Count", minimum: 0, maximum: 9 },
                    ratio: { type: "number", title: "Ratio", exclusiveMinimum: 0, multipleOf: 0.5 },
                },
                required: ["email"],
            }}
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

test("type a date, a time and a date and time into their fields, submitting the date as written, the time at the person's offset and the moment in UTC", () => {
    const { container, submitted } = drawForm({
        type: "object",
        properties: {
            email: { type: "string", title: "Email", format: "email" },
            day: { type: "string", title: "Day", format: "date" },
            starts: { type: "string", title: "Starts", format: "time" },
            at: { type: "string", title: "At", format: "date-time" },
        },
        required: ["email", "day", "starts", "at"],
    });
    fill(container, "Email", "milk");
    const refused = refusals(container);
    fill(container, "Email", "milk@example.com");
    fillSegments(grouped(container, "Day"), { month: "10", day: "07", year: "2026" });
    fillSegments(grouped(container, "Starts"), { hour: "09", minute: "30", dayPeriod: "a" });
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

    // the time of day as typed, at the local clock's offset from UTC
    const east = -new Date().getTimezoneOffset();
    const hours = String(Math.floor(Math.abs(east) / 60)).padStart(2, "0");
    const offset = `${east < 0 ? "-" : "+"}${hours}:${String(Math.abs(east) % 60).padStart(2, "0")}`;
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
                starts: `09:30:00${offset}`,
                at: new Date("2026-10-07T09:30").toISOString(),
            },
        ],
    ]);
});

test("add and remove items of a list, and choose several values of an enumeration", () => {
    const { container, submitted } = drawForm({
        type: "object",
        properties: {
            steps: { type: "array", title: "Step", items: { type: "string", minLength: 1 } },
            colors: {
                type: "array",
                title: "Colors",
                items: { type: "string", enum: ["red", "green", "blue"] },
            },
        },
        required: ["steps", "colors"],
    });
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
    const { container, submitted } = drawForm({
        type: "object",
        properties: {
            address: {
                type: "object",
                title: "Address",
                properties: {
                    city: { type: "string", title: "City", minLength: 1 },
                    geo: {
                        type: "object",
                        title: "Position",
                        properties: {
                            latitude: { type: "number", title: "Latitude", maximum: 90 },
                        },
                        required: ["latitude"],
                    },
                },
                required: ["city", "geo"],
            },
        },
        required: ["address"],
    });
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
    const { container, submitted } = drawForm({
        type: "object",
        properties: {
            shape: {
                title: "Shape",
                oneOf: [
                    {
                        type: "object",
                        properties: {
                            kind: { type: "string", const: "circle" },
                            radius: { type: "number", title: "Radius" },
                        },
                        required: ["kind", "radius"],
                    },
                    {
                        type: "object",
                        properties: {
                            kind: { type: "string", const: "square" },
                            side: { type: "number", title: "Side" },
                        },
                        required: ["kind", "side"],
                    },
                ],
            },
        },
        required: ["shape"],
    });
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
    const container = draw(() => (
        <SchemaForm
            schema={{
                type: "object",
                properties: {
                    title: { type: "string", title: "Title" },
                    ownerId: { type: "string", title: "Owner" },
                },
                required: ["title", "ownerId"],
            }}
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

/** A tree whose nodes hold a name and child nodes, referring back to the node. */
const TREE: JsonObject = {
    type: "object",
    $defs: {
        node: {
            type: "object",
            properties: {
                name: { type: "string", title: "Name", minLength: 1 },
                children: { type: "array", title: "Child", items: { $ref: "#/$defs/node" } },
            },
            required: ["name"],
        },
    },
    properties: { tree: { $ref: "#/$defs/node", title: "Tree" } },
    required: ["tree"],
};

/** Type a value into an input, as the person does. */
function typeInto(input: HTMLInputElement | undefined, value: string): void {
    if (input !== undefined) {
        input.value = value;
        input.dispatchEvent(new Event("input", { bubbles: true }));
    }
    flush();
}

test("resolve a $defs reference against the root schema, its title beside the reference winning, and submit its fields", () => {
    const described: JsonObject = {
        type: "object",
        $defs: {
            address: {
                type: "object",
                title: "Address",
                properties: { city: { type: "string", title: "City", minLength: 1 } },
                required: ["city"],
            },
        },
        properties: { home: { $ref: "#/$defs/address", title: "Home" } },
        required: ["home"],
    };
    const [home] = schemaFields(described);
    const { container, submitted } = drawForm(described);
    fill(container, "City", "Vienna");
    submit(container);
    expect([
        home?.title,
        home?.control === "object" ? home.fields.map((field) => field.title) : undefined,
        submitted,
    ]).toEqual(["Home", ["City"], [{ home: { city: "Vienna" } }]]);
});

test("resolve pointers whose tokens escape a slash and a tilde, under $defs and definitions alike", () => {
    const fields = schemaFields({
        type: "object",
        $defs: { "a/b~c": { type: "string", title: "Slashed" } },
        definitions: { "d e": { type: "integer", title: "Spaced" } },
        properties: {
            slashed: { $ref: "#/$defs/a~1b~0c" },
            spaced: { $ref: "#/definitions/d%20e" },
        },
    });
    expect(fields.map((field) => [field.name, field.title, field.control])).toEqual([
        ["slashed", "Slashed", "text"],
        ["spaced", "Spaced", "number"],
    ]);
});

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
    expect(() => schemaFields(loop)[0]?.start).toThrow(
        new TypeError("schema #/$defs/link requires a value of itself"),
    );
});
