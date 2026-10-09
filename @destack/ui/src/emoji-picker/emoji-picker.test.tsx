import { Locale, Localization } from "@destack/locale";
import { expect, test } from "@destack/test";
import { render } from "@destack/view/test";
import { flush, LocaleContext } from "@destack/view";
import {
    EmojiPicker,
    EmojiPickerContent,
    EmojiPickerFooter,
    EmojiPickerSearch,
    EmojiPickerSkinTone,
} from "./index.ts";

/** Wait until a container shows an element a selector finds, flushing between looks. */
async function shown(container: HTMLElement, selector: string): Promise<HTMLElement> {
    for (let attempt = 0; attempt < 100; attempt++) {
        flush();
        const found = container.querySelector<HTMLElement>(selector);
        if (found !== null) {
            return found;
        }
        await new Promise((resolve) => {
            setTimeout(resolve, 10);
        });
    }
    throw new TypeError(`nothing shows ${selector}`);
}

/** Press a key in a container's search field. */
function press(container: HTMLElement, key: string): void {
    container
        .querySelector("input[type=search]")
        ?.dispatchEvent(new KeyboardEvent("keydown", { key, bubbles: true }));
    flush();
}

/** Type a search into a container's search field as a person does. */
function type(container: HTMLElement, search: string): void {
    const input = container.querySelector<HTMLInputElement>("input[type=search]");
    if (input !== null) {
        input.value = search;
        input.dispatchEvent(new Event("input", { bubbles: true }));
    }
    flush();
}

test("pick the emoji a search finds in the chosen skin tone, and show the empty part when nothing matches", async () => {
    // search for waving and pick it in the darkest tone
    const picked: string[] = [];
    const { container } = render(() => (
        <EmojiPicker onEmojiSelect={(entry) => picked.push(entry.emoji)}>
            <EmojiPickerSearch />
            <EmojiPickerSkinTone />
            <EmojiPickerContent />
        </EmojiPicker>
    ));
    type(container, "waving hand");
    (await shown(container, "[role=radio]:last-of-type")).click();
    (await shown(container, '[role=gridcell][aria-label="waving hand"]')).click();

    // search for nothing an emoji is called
    type(container, "qqqqq");
    const empty = await shown(container, "[data-slot=emoji-picker-empty]");

    expect({ picked, empty: empty.textContent }).toEqual({
        picked: ["👋🏿"],
        empty: "No emoji found",
    });
});

test("name and find emoji in the reader's language, a regional locale falling back to its language", async () => {
    const { container } = render(() => (
        <LocaleContext value={Localization.of(Locale.parse("de-AT"), [])}>
            <EmojiPicker defaultSearch="tschüss" onEmojiSelect={() => undefined}>
                <EmojiPickerContent />
            </EmojiPicker>
        </LocaleContext>
    ));

    expect((await shown(container, "[role=gridcell]")).getAttribute("aria-label")).toBe(
        "winkende Hand",
    );
});

test("move the active emoji through the rows and columns from the search, show it in the footer and pick it on Enter", async () => {
    // search the faces, laid out three to a row
    const picked: string[] = [];
    const { container } = render(() => (
        <EmojiPicker columns={3} onEmojiSelect={(entry) => picked.push(entry.label)}>
            <EmojiPickerSearch />
            <EmojiPickerContent />
            <EmojiPickerFooter />
        </EmojiPicker>
    ));
    type(container, "face");
    await shown(container, "[role=gridcell]");
    const cells = [...container.querySelectorAll("[role=gridcell]")];
    const target = cells[4];

    // step down a row and right a column, then pick
    press(container, "ArrowDown");
    press(container, "ArrowRight");
    const active = container.querySelector("[role=gridcell][aria-selected=true]");
    const footer = container.querySelector("[data-slot=emoji-picker-footer]")?.textContent;
    press(container, "Enter");

    // the footer shows the fifth emoji and its name, and Enter picks it
    expect({ active: active?.id, footer, picked }).toEqual({
        active: target?.id,
        footer: `${String(target?.textContent)}${String(target?.getAttribute("aria-label"))}`,
        picked: [target?.getAttribute("aria-label")],
    });
});
