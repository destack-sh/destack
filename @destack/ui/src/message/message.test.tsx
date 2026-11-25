import { expect, test } from "@destack/test";
import { flush, LocaleContext } from "@destack/view";
import { Localization } from "@destack/locale";
import { markup, render, testWorld } from "@destack/view/test";
import { ToolbarButton } from "../toolbar/index.ts";
import {
    Message,
    MessageToolbar,
    MessageAuthor,
    MessageAvatar,
    MessageContent,
    MessageFooter,
    MessageHeader,
} from "./index.ts";

/** Send a finger's event to an element at a place. */
function touch(
    element: Element,
    type: string,
    points: readonly (readonly [number, number])[],
): boolean {
    const event = new Event(type, { bubbles: true, cancelable: true });
    const touches = points.map(([clientX, clientY]) => ({ clientX, clientY }));
    element.dispatchEvent(Object.assign(event, { touches }));
    flush();

    return event.defaultPrevented;
}

/** Find the message of a container. */
function messageOf(container: Element): Element {
    const message = container.querySelector("[data-slot=message]");
    if (message === null) {
        throw new TypeError("the test message did not render");
    }

    return message;
}

test("render a message's avatar, header, body, footer and actions", () => {
    const { container } = render(() => (
        <Message align="end">
            <MessageAvatar>AL</MessageAvatar>
            <MessageContent>
                <MessageHeader>
                    <MessageAuthor>Ada</MessageAuthor>
                </MessageHeader>
                Hello
                <MessageFooter>Read</MessageFooter>
            </MessageContent>
            <MessageToolbar aria-label="Actions">
                <ToolbarButton>Reply</ToolbarButton>
            </MessageToolbar>
        </Message>
    ));

    expect(markup(container)).toBe(
        [
            '<div data-slot="message" data-align="end">',
            '<div data-slot="message-avatar">AL</div>',
            '<div data-slot="message-content">',
            '<div data-slot="message-header"><span data-slot="message-author">Ada</span></div>',
            'Hello<div data-slot="message-footer">Read</div></div>',
            '<div role="toolbar" aria-orientation="horizontal" data-slot="message-toolbar" aria-label="Actions">',
            '<fieldset role="none">',
            '<button data-variant="ghost" data-size="icon" type="button" data-slot="toolbar-button">Reply</button>',
            "</fieldset></div></div>",
        ].join(""),
    );
});

test("show the actions by a finger that rests, until a press elsewhere", async () => {
    const { simulation, tab } = testWorld();
    const { container } = render(
        () => (
            <Message>
                <MessageContent>Hello</MessageContent>
                <MessageToolbar aria-label="Actions">
                    <ToolbarButton>Reply</ToolbarButton>
                </MessageToolbar>
            </Message>
        ),
        { environment: tab },
    );
    const message = messageOf(container);
    const isActive = (): boolean => message.hasAttribute("data-active");
    const rest = async (milliseconds: number): Promise<boolean> => {
        await simulation.jump(simulation.clock.now() + milliseconds);
        flush();

        return isActive();
    };

    // rest too short, drift too far, use two fingers, then rest
    touch(message, "touchstart", [[100, 100]]);
    const early = await rest(400);
    const short = touch(message, "touchend", []);
    touch(message, "touchstart", [[100, 100]]);
    touch(message, "touchmove", [[100, 120]]);
    const drifted = await rest(600);
    touch(message, "touchstart", [
        [100, 100],
        [120, 100],
    ]);
    const double = await rest(600);
    touch(message, "touchstart", [[100, 100]]);
    const rested = await rest(600);
    const long = touch(message, "touchend", []);

    // press inside, then elsewhere
    message.dispatchEvent(new Event("pointerdown", { bubbles: true }));
    flush();
    const inside = isActive();
    document.body.dispatchEvent(new Event("pointerdown", { bubbles: true }));
    flush();

    expect({ early, short, long, drifted, double, rested, inside, elsewhere: isActive() }).toEqual({
        early: false,
        short: false,
        long: true,
        drifted: false,
        double: false,
        rested: true,
        inside: true,
        elsewhere: false,
    });
});

test.for([
    ["en", 1],
    ["ar", -1],
] as const)("answer a message pulled aside far enough in %s", ([tag, sign]) => {
    Object.defineProperty(document.documentElement, "clientWidth", {
        value: 400,
        configurable: true,
    });
    let answers = 0;
    const { container } = render(() => (
        <LocaleContext value={Localization.of(tag, [])}>
            <Message onSwipe={() => (answers += 1)}>Hello</Message>
        </LocaleContext>
    ));
    const message = messageOf(container);
    const pull = (from: number, by: number, down = 0): [boolean, number] => {
        touch(message, "touchstart", [[from, 100]]);
        touch(message, "touchmove", [[from + sign * by, 100 + down]]);
        const isSwiping = message.hasAttribute("data-swiping");
        touch(message, "touchend", []);

        return [isSwiping, answers];
    };

    // pull a little, back, from the edge, down, then far enough
    expect([pull(200, 30), pull(200, -80), pull(10, 80), pull(200, 80, 80), pull(200, 80)]).toEqual(
        [
            [true, 0],
            [false, 0],
            [false, 0],
            [false, 0],
            [true, 1],
        ],
    );
});
