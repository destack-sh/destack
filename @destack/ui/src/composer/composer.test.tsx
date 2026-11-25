import { expect, test } from "@destack/test";
import { createSignal, flush } from "@destack/view";
import { markup, render } from "@destack/view/test";
import {
    ComposerAddon,
    Composer,
    ComposerFooter,
    ComposerFrame,
    ComposerHeader,
    ComposerInput,
    ComposerSubmit,
} from "./index.ts";

/** Press Enter in the text with some keys held, and report whether the press was taken. */
function enter(container: Element, held: KeyboardEventInit = {}): boolean {
    const event = new KeyboardEvent("keydown", {
        key: "Enter",
        bubbles: true,
        cancelable: true,
        ...held,
    });
    container.querySelector("textarea")?.dispatchEvent(event);
    flush();

    return event.defaultPrevented;
}

test("render a composer's header, addon, text and footer as a form", () => {
    const { container } = render(() => (
        <Composer>
            <ComposerFrame>
                <ComposerHeader>Files</ComposerHeader>
                <ComposerAddon>Beside</ComposerAddon>
                <ComposerInput placeholder="Write" />
                <ComposerFooter>
                    <ComposerSubmit />
                </ComposerFooter>
            </ComposerFrame>
        </Composer>
    ));

    expect(markup(container)).toBe(
        [
            '<form data-slot="composer"><div data-slot="composer-frame" data-variant="raised">',
            '<div data-slot="composer-header">Files</div>',
            '<div data-slot="composer-addon" data-align="inline-end">Beside</div>',
            '<textarea data-slot="composer-input" rows="1" aria-label="Message" placeholder="Write"></textarea>',
            '<div data-slot="composer-footer">',
            '<button data-slot="composer-submit" data-variant="default" data-size="icon" type="submit" aria-label="Send">',
            '<svg viewBox="0 0 256 256" fill="currentColor" width="1em" height="1em" aria-hidden="true" data-slot="icon"></svg>',
            "</button></div></div></form>",
        ].join(""),
    );
});

test.for([
    ["enter", [true, false, true, false, 2]],
    ["ctrlEnter", [false, false, true, false, 1]],
    ["none", [false, false, true, false, 0]],
] as const)(
    "send by %s and the command key, unless by none, breaking the line and composing else",
    ([submitMode, expected]) => {
        let sent = 0;
        const { container } = render(() => (
            <Composer submitMode={submitMode} onSubmit={() => (sent += 1)}>
                <ComposerFrame>
                    <ComposerInput />
                </ComposerFrame>
            </Composer>
        ));

        expect([
            enter(container),
            enter(container, { shiftKey: true }),
            enter(container, { ctrlKey: true }),
            enter(container, { isComposing: true }),
            sent,
        ]).toEqual(expected);
    },
);

test("stop the answer under way in place of sending, and take nothing while disabled", () => {
    // send, stop, then press while disabled
    const given: string[] = [];
    const [isBusy, setBusy] = createSignal(false);
    const [isDisabled, setDisabled] = createSignal(false);
    const { container } = render(() => (
        <Composer
            busy={isBusy()}
            disabled={isDisabled()}
            onSubmit={() => void given.push("send")}
            onStop={() => void given.push("stop")}
        >
            <ComposerFrame>
                <ComposerInput />
                <ComposerSubmit />
            </ComposerFrame>
        </Composer>
    ));
    const button = container.querySelector("button");
    const labels: (string | null | undefined)[] = [];
    const press = (): void => {
        labels.push(button?.getAttribute("aria-label"));
        button?.click();
        flush();
    };
    press();
    setBusy(true);
    flush();
    press();
    setBusy(false);
    setDisabled(true);
    flush();
    press();

    setDisabled(false);
    setBusy(true);
    flush();
    container
        .querySelector("textarea")
        ?.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true }));
    flush();

    expect({ given, labels }).toEqual({
        given: ["send", "stop", "stop"],
        labels: ["Send", "Stop", "Send"],
    });
});

test("send and stop from any element its text renders", () => {
    const given: string[] = [];
    const [isBusy, setBusy] = createSignal(false);
    const { container } = render(() => (
        <Composer
            busy={isBusy()}
            onSubmit={() => void given.push("send")}
            onStop={() => void given.push("stop")}
        >
            <ComposerFrame>
                <ComposerInput
                    render={(attributes) => <div {...attributes} contenteditable="true" />}
                />
            </ComposerFrame>
        </Composer>
    ));
    const press = (key: string): void => {
        container
            .querySelector("[contenteditable]")
            ?.dispatchEvent(new KeyboardEvent("keydown", { key, bubbles: true, cancelable: true }));
        flush();
    };
    press("Enter");
    setBusy(true);
    flush();
    press("Escape");

    expect(given).toEqual(["send", "stop"]);
});
