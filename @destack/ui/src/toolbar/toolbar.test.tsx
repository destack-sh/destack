import { Locale, Localization } from "@destack/locale";
import { expect, test } from "@destack/test";
import { flush, LocaleContext } from "@destack/view";
import { render } from "@destack/view/test";
import { Input } from "../input/index.ts";
import { ToggleGroup, ToggleGroupItem } from "../toggle-group/index.ts";
import { Toolbar, ToolbarButton } from "./index.ts";

/** Read what the focus is on, by its text or its label. */
function focused(): string | null | undefined {
    const text = document.activeElement?.textContent ?? "";

    return text === "" ? document.activeElement?.getAttribute("aria-label") : text;
}

/** Press a key where the focus is, and read where it goes. */
function press(key: string): string | null | undefined {
    document.activeElement?.dispatchEvent(
        new KeyboardEvent("keydown", { key, bubbles: true, cancelable: true }),
    );
    flush();

    return focused();
}

/** Draw a toolbar. */
function draw(tag = "en", loop = true) {
    const { container } = render(() => (
        <LocaleContext value={Localization.of(Locale.parse(tag), [])}>
            <button type="button">before</button>
            <Toolbar aria-label="Text" loop={loop}>
                <ToolbarButton>undo</ToolbarButton>
                <ToolbarButton disabled>redo</ToolbarButton>
                <ToggleGroup multiple>
                    <ToggleGroupItem value="bold">bold</ToggleGroupItem>
                    <ToggleGroupItem value="italic">italic</ToggleGroupItem>
                </ToggleGroup>
                <Input aria-label="find" />
            </Toolbar>
        </LocaleContext>
    ));
    const focus = (text: string) =>
        [...container.querySelectorAll("button")]
            .find((each) => each.textContent === text)
            ?.focus();

    return { container, focus };
}

test("move between controls by the arrow keys, through groups inside", () => {
    const { focus } = draw();
    focus("undo");

    expect(
        ["ArrowRight", "ArrowRight", "ArrowRight", "ArrowRight", "ArrowRight"].map(press),
    ).toEqual(["redo", "bold", "italic", "find", "undo"]);
});

test("loop at the ends, or stop there", () => {
    const looping = draw();
    looping.focus("undo");
    const looped = [press("ArrowLeft")];
    const stopping = draw("en", false);
    stopping.focus("undo");

    expect([looped, press("ArrowLeft"), press("End"), focused()]).toEqual([
        ["find"],
        "undo",
        "find",
        "find",
    ]);
});

test("mirror the arrows in right-to-left text", () => {
    const { focus } = draw("ar");
    focus("undo");

    expect([press("ArrowLeft"), press("ArrowRight")]).toEqual(["redo", "undo"]);
});

test("leave by Tab from its last control", () => {
    const { focus } = draw();
    focus("undo");

    expect(press("Tab")).toBe("find");
});

test("come back by Tab to the control it was left from, and leave other focus alone", () => {
    const { container, focus } = draw();
    const input = container.querySelector<HTMLElement>("[aria-label=find]");
    const enter = (key: string) => {
        focus("italic");
        press("Tab");
        focus("before");
        press(key);
        input?.focus();

        return focused();
    };

    expect([enter("Tab"), enter("Enter")]).toEqual(["italic", "find"]);
});

test("take no input while disabled", () => {
    const { container } = render(() => (
        <Toolbar aria-label="Text" disabled>
            <ToolbarButton>undo</ToolbarButton>
            <Input aria-label="find" />
        </Toolbar>
    ));
    const toolbar = container.querySelector("[role=toolbar]");

    expect([
        toolbar?.getAttribute("aria-disabled"),
        toolbar?.querySelector("fieldset")?.disabled,
        toolbar?.querySelectorAll("fieldset button, fieldset input").length,
    ]).toEqual(["true", true, 2]);
});

test("leave a text at its ends alone", () => {
    const { container, focus } = draw();
    const input = container.querySelector<HTMLInputElement>("[aria-label=find]");
    if (input !== null) {
        input.value = "lake";
    }
    focus("italic");
    press("ArrowRight");
    const walk = (caret: number) => {
        input?.focus();
        input?.setSelectionRange(caret, caret);

        return press("ArrowLeft");
    };

    expect([walk(4), walk(2), walk(0)]).toEqual(["find", "find", "italic"]);
});
