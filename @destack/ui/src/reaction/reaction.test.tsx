import { expect, test } from "@destack/test";
import { flush } from "@destack/view";
import { render, stubPopovers } from "@destack/view/test";
import { Reaction, ReactionGroup } from "./index.ts";

test("add and take back one's own reaction, read by its emoji and count", () => {
    const given: boolean[] = [];
    const { container } = render(() => (
        <ReactionGroup>
            <Reaction
                emoji="🎉"
                count={1}
                defaultPressed={false}
                onPressedChange={(isPressed) => void given.push(isPressed)}
            />
            <Reaction emoji="👀" count={1200} pressed />
        </ReactionGroup>
    ));
    const buttons = [...container.querySelectorAll<HTMLElement>("[data-slot=reaction]")];
    const read = buttons.map((button) => [
        button.getAttribute("aria-label"),
        button.getAttribute("aria-pressed"),
        button.textContent,
    ]);
    for (let press = 0; press < 2; press += 1) {
        buttons[0]?.click();
        flush();
    }

    expect({
        read,
        given,
        label: container.querySelector("[role=toolbar]")?.getAttribute("aria-label"),
    }).toEqual({
        read: [
            ["\u2068🎉\u2069, 1 person", "false", "🎉1"],
            ["\u2068👀\u2069, 1,200 people", "true", "👀1,200"],
        ],
        given: [true, false],
        label: "Reactions",
    });
});

test("list who reacted, and count the rest", () => {
    stubPopovers();
    const people = Array.from({ length: 12 }, (_, index) => `Person ${String(index)}`);
    const { container } = render(() => (
        <ReactionGroup>
            <Reaction emoji="🎉" count={12} people={people} />
        </ReactionGroup>
    ));
    container.querySelector<HTMLElement>("[data-slot=reaction]")?.focus();
    flush();

    expect(document.querySelector("[data-slot=tooltip-content]")?.textContent).toBe(
        `${people.slice(0, 10).join(", ")}, and 2 others`,
    );
});
