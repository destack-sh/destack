import { readFile } from "node:fs/promises";
import { join } from "node:path";
import { Catalog, Localization, t } from "@destack/locale";
import { expect, onTestFinished, test } from "@destack/test";
import { Toaster, toast } from "@destack/ui/toast";
import { commandsOf, note, queryOf, settled, type Settled } from "./fixture.ts";
import { createSignal, flush } from "../solid/reactive.ts";
import { draw, stubPopovers, wait } from "../test/dom.ts";
import { messageOf } from "./binding.ts";
import { CommandButton } from "./command-button.tsx";
import { Field } from "./field.tsx";
import { ReferenceCombobox } from "./reference-combobox.tsx";
import { StateTransition } from "./state-transition.tsx";

/** The German catalog this package ships of its built-in messages. */
const GERMAN = Catalog.of(
    "locale/de.json",
    JSON.parse(await readFile(join(import.meta.dirname, "../../locale/de.json"), "utf8")),
    t`Saved`.package,
);

/** Commit a value to a field's control as a person does, typing and leaving. */
function commit(control: Element | null, value: string): void {
    if (control instanceof HTMLInputElement || control instanceof HTMLSelectElement) {
        control.value = value;
        control.dispatchEvent(new Event("change", { bubbles: true }));
    }
    flush();
}

test("render a bound text field as a labelled input, writing a valid value and following its save", async () => {
    const [title, setTitle] = createSignal("Groceries");
    const writes: string[] = [];
    let pending: Settled<unknown> | undefined;
    const container = draw(() => (
        <Field
            for={{
                object: note,
                field: "title",
                value: title(),
                write: (value) => {
                    writes.push(value);
                    setTitle(value);
                    pending = settled<unknown>(value);

                    return pending.submission;
                },
            }}
            label="Title"
        />
    ));
    const input = container.querySelector("input");
    const shown = input?.value;
    commit(input, "Milk");
    const saving = container.querySelector("[data-slot=field-status]")?.textContent;
    pending?.confirm();
    await wait(0);
    flush();
    expect([
        shown,
        writes,
        saving,
        container.querySelector("[data-slot=field-status]")?.textContent,
        input?.value,
    ]).toEqual(["Groceries", ["Milk"], "Saving", "Saved", "Milk"]);
});

test("refuse a value the field's schema refuses, explaining it in the person's language", () => {
    const writes: string[] = [];
    const container = draw(() => (
        <Field
            for={{
                object: note,
                field: "title",
                value: "Groceries",
                write: (value) => (writes.push(value), settled<unknown>(value).submission),
            }}
            label="Title"
        />
    ));
    const input = container.querySelector("input");
    commit(input, "");
    const empty = [
        input?.getAttribute("aria-invalid"),
        container.querySelector("[role=alert]")?.textContent,
    ];
    commit(input, "A title far longer than twenty characters");
    expect([empty, container.querySelector("[role=alert]")?.textContent, writes]).toEqual([
        ["true", "Enter a value"],
        "Enter at most \u206820\u2069 characters",
        [],
    ]);
    expect(input?.getAttribute("aria-describedby")).toBe(
        container.querySelector("[role=alert]")?.id,
    );
});

test("mark a field whose write the server refuses", async () => {
    let pending: Settled<unknown> | undefined;
    const container = draw(() => (
        <Field
            for={{
                object: note,
                field: "words",
                value: 3,
                write: (value) => (pending = settled<unknown>(value)).submission,
            }}
            label="Words"
        />
    ));
    const input = container.querySelector("input");
    commit(input, "12");
    pending?.refuse();
    await wait(0);
    flush();
    expect([
        input?.type,
        container.querySelector("[role=alert]")?.textContent,
        input?.getAttribute("aria-invalid"),
    ]).toEqual(["number", "Could not save", "true"]);
});

/** Write any value, confirming nothing. */
function write(value: unknown) {
    return settled<unknown>(value).submission;
}

test("choose each field type's control from its declaration", async () => {
    const container = draw(() => (
        <>
            <Field for={{ object: note, field: "pinned", value: true, write }} label="Pinned" />
            <Field for={{ object: note, field: "kind", value: "task", write }} label="Kind" />
            <Field
                for={{
                    object: note,
                    field: "status",
                    value: "published",
                    write,
                    id: "note-1",
                    access: commandsOf(["manage"], []),
                }}
                label="Status"
            />
        </>
    ));
    await wait(0);
    flush();
    const pinned = container.querySelector<HTMLInputElement>("input[role=switch]");
    const kind = container.querySelector("select");
    expect([
        pinned?.checked,
        [...(kind?.options ?? [])].map((option) => option.value),
        kind?.value,
    ]).toEqual([true, ["idea", "task"], "task"]);
    expect([...container.querySelectorAll("label")].map((label) => label.textContent)).toEqual([
        "Pinned",
        "Kind",
    ]);

    // a state field titles its state and the transitions the person may make
    expect([
        container.querySelector("[data-slot=field-label]:not(label)")?.textContent,
        container.querySelector("[data-slot=state-transition]")?.textContent,
    ]).toEqual(["Status", "publishedarchive"]);
});

test("run a command, busy until the server confirms it, and toast a refusal", async () => {
    stubPopovers();
    onTestFinished(() => toast.dismiss());
    let pending: Settled<unknown> | undefined;
    const container = draw(() => (
        <>
            <CommandButton
                run={() => (pending = settled<unknown>(undefined)).submission}
                errorMessage="Could not archive"
            >
                Archive
            </CommandButton>
            <Toaster />
        </>
    ));
    const button = container.querySelector<HTMLButtonElement>("[data-slot=command-button]");
    button?.click();
    flush();
    const busy = [button?.getAttribute("aria-busy"), button?.getAttribute("data-state")];
    button?.click();
    pending?.refuse();
    await wait(0);
    flush();
    expect([
        busy,
        button?.getAttribute("data-state"),
        container.querySelector("[data-slot=toast-title]")?.textContent,
    ]).toEqual([["true", "pending"], "failed", "Could not archive"]);
});

test("show the state and offer exactly the transitions it allows and the person may make", async () => {
    const calls: string[] = [];
    const container = draw(() => (
        <>
            <StateTransition
                access={commandsOf(["write", "manage"], calls)}
                object={note}
                id="note-1"
                field="status"
                value="draft"
            />
            <StateTransition
                access={commandsOf(["write"], [])}
                object={note}
                id="note-1"
                field="status"
                value="draft"
            />
            <StateTransition
                access={commandsOf(["write"], [])}
                object={note}
                id="note-1"
                field="status"
                value="archived"
                labels={{ archived: "Archived", restore: "Restore" }}
            />
        </>
    ));
    await wait(0);
    flush();
    const offered = [...container.querySelectorAll("[data-slot=state-transition]")].map((group) =>
        [...group.querySelectorAll("[data-slot=state-transition-state], button")].map(
            (element) => element.textContent,
        ),
    );
    container.querySelector<HTMLElement>("[data-transition=archive]")?.click();
    expect([offered, calls]).toEqual([
        [
            ["draft", "publish", "archive"],
            ["draft", "publish"],
            ["Archived", "Restore"],
        ],
        ["archive"],
    ]);
});

test("search objects through the query as the person types and pick one", async () => {
    stubPopovers();
    const notebooks = [
        { id: "notebook-1", name: "Trips" },
        { id: "notebook-2", name: "Work" },
        { id: "notebook-3", name: "Trivia" },
    ];
    const searches: string[] = [];
    const chosen: string[] = [];
    const container = draw(() => (
        <ReferenceCombobox
            aria-label="Notebook"
            value={notebooks[1]}
            search={(text) => {
                searches.push(text);

                return queryOf(() =>
                    notebooks.filter((notebook) =>
                        notebook.name.toLowerCase().startsWith(text.toLowerCase()),
                    ),
                );
            }}
            label={(notebook) => notebook.name}
            onValueChange={(notebook) => chosen.push(notebook.id)}
        />
    ));
    await wait(0);
    flush();
    const input = container.querySelector("input");
    const shown = input?.value;
    if (input !== null) {
        input.value = "tri";
        input.dispatchEvent(new InputEvent("input", { bubbles: true }));
    }
    await wait(0);
    flush();
    const options = [...container.querySelectorAll("[role=option]")].map(
        (option) => option.textContent,
    );
    container.querySelectorAll<HTMLElement>("[role=option]")[1]?.click();
    await wait(0);
    flush();
    expect([shown, searches.at(-1), options, chosen, input?.value]).toEqual([
        "Work",
        "Trivia",
        ["Trips", "Trivia"],
        ["notebook-3"],
        "Trivia",
    ]);
});

test("explain each refused value in the German drafts the package ships, for an Austrian reader", () => {
    const german = Localization.of("de-AT", [GERMAN]);
    const problems = [
        { code: "too_small", origin: "string", minimum: 1 },
        { code: "too_small", origin: "string", minimum: 3 },
        { code: "too_big", origin: "string", maximum: 20 },
        { code: "too_small", origin: "number", minimum: 0 },
        { code: "too_big", origin: "number", maximum: 9 },
        { code: "invalid_type" },
        { code: "invalid_value" },
        { code: "custom" },
    ];
    expect(problems.map((problem) => messageOf([problem], german))).toEqual([
        "Wert eingeben",
        "Mindestens \u20683\u2069 Zeichen eingeben",
        "Höchstens \u206820\u2069 Zeichen eingeben",
        "\u20680\u2069 oder mehr eingeben",
        "\u20689\u2069 oder weniger eingeben",
        "Wert eingeben",
        "Eine der Optionen wählen",
        "Gültigen Wert eingeben",
    ]);
});
