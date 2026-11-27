import type { Choice } from "@destack/object";
import { Mention } from "@destack/object/markdown";
import { Subject } from "@destack/sync";
import { present } from "@destack/schema";
import { expect, test } from "@destack/test";
import { createSignal, flush } from "@destack/view";
import { render, stubPopovers } from "@destack/view/test";
import { Composer, ComposerInput, ComposerMention } from "./index.ts";

/** The people a mention list finds, valued by a handle in place of a subject. */
const PEOPLE: readonly Choice[] = [
    { value: "ada", title: "Ada Lovelace", description: "ada@example.com" },
    { value: "grace", title: "Grace Hopper" },
];

/** Read the handle a test person is valued by. */
function handleOf(choice: Choice): string {
    return typeof choice.value === "string" ? choice.value : "";
}

/** Draw a composer with a mention list, recording its searches, its text and what it sends. */
function draw() {
    stubPopovers();
    const searches: string[] = [];
    const sent: string[] = [];
    const [text, setText] = createSignal("");
    const { container } = render(() => (
        <Composer onSubmit={() => sent.push(text())}>
            <ComposerInput value={text()} onInput={(event) => setText(event.currentTarget.value)} />
            <ComposerMention
                choices={PEOPLE}
                onSearch={(search) => searches.push(search)}
                onMention={(choice) => `[@${choice.title}](person:${handleOf(choice)})`}
            />
        </Composer>
    ));
    const input = present(container.querySelector("textarea"), "the text");

    return { container, input, searches, sent, text };
}

/** Type text into a text area at its end, as a person does. */
function type(input: HTMLTextAreaElement, typed: string): void {
    input.focus();
    input.value += typed;
    input.setSelectionRange(input.value.length, input.value.length);
    input.dispatchEvent(new InputEvent("input", { bubbles: true, inputType: "insertText" }));
    flush();
}

/** Press a key in the text, reporting whether something took it. */
function press(input: HTMLTextAreaElement, key: string): boolean {
    const event = new KeyboardEvent("keydown", { key, bubbles: true, cancelable: true });
    input.dispatchEvent(event);
    flush();

    return event.defaultPrevented;
}

/** Read the names of the options a mention list shows, none while it is closed. */
function shown(container: Element): string[] {
    const list = present(container.querySelector("[data-slot=composer-mention]"), "the list");

    return list.hasAttribute("data-popover-open")
        ? [...list.querySelectorAll("[role=option]")].map((option) => option.textContent)
        : [];
}

test("search people after an @, and put the one chosen by Enter in its place", () => {
    // type an @ search, move to the second person and choose them
    const { container, input, searches, sent, text } = draw();
    type(input, "Ask @");
    type(input, "gr");
    const open = shown(container);
    press(input, "ArrowDown");
    press(input, "Enter");

    // expect the searches, the list, the mention in the text and nothing sent
    expect({ searches, open, text: text(), closed: shown(container), sent }).toEqual({
        searches: ["", "gr"],
        open: ["Ada Lovelaceada@example.com", "Grace Hopper"],
        text: "Ask [@Grace Hopper](person:grace) ",
        closed: [],
        sent: [],
    });
});

test("leave an @ inside a word, close on Escape, and send by Enter once closed", () => {
    // type an address, then an @ search the person closes, then send
    const { container, input, searches, sent } = draw();
    type(input, "mail ada@example");
    const inWord = shown(container);
    type(input, " @ad");
    const isEscapeTaken = press(input, "Escape");
    type(input, "a");
    const afterEscape = shown(container);
    press(input, "Enter");

    // expect no search inside the address, the list kept closed on that @, and the text sent
    expect({ inWord, searches, isEscapeTaken, afterEscape, sent }).toEqual({
        inWord: [],
        searches: ["ad"],
        isEscapeTaken: true,
        afterEscape: [],
        sent: ["mail ada@example @ada"],
    });
});

test("choose the first person by Tab, keeping the text after the caret", () => {
    // type a search before a sentence's end, then put the caret back after the search
    const { input, text } = draw();
    type(input, "Hi @a, welcome");
    const caret = "Hi @a".length;
    input.setSelectionRange(caret, caret);
    input.dispatchEvent(new KeyboardEvent("keyup", { key: "ArrowLeft", bubbles: true }));
    flush();
    press(input, "Tab");

    expect(text()).toBe("Hi [@Ada Lovelace](person:ada) , welcome");
});

test("write a chosen subject's Markdown mention by default, which reads back as the subject", () => {
    // choose the only person, mentioned by their subject
    stubPopovers();
    const subject = Subject.parse({
        packageId: "package-01996ab0-0000-7000-8000-000000000009",
        type: "person",
        scope: "team",
        id: "ada",
    });
    const [text, setText] = createSignal("");
    const { container } = render(() => (
        <Composer>
            <ComposerInput value={text()} onInput={(event) => setText(event.currentTarget.value)} />
            <ComposerMention choices={[{ value: subject, title: "Ada" }]} onSearch={() => {}} />
        </Composer>
    ));
    const input = present(container.querySelector("textarea"), "the text");
    type(input, "@a");
    press(input, "Enter");

    // read the link's address back as the subject
    const href = /\]\((?<href>[^)]+)\) $/u.exec(text())?.groups?.["href"] ?? "";
    expect({ text: text(), subject: Mention.subject(href) }).toEqual({
        text: `${Mention.markdown(subject, "Ada")} `,
        subject,
    });
});
