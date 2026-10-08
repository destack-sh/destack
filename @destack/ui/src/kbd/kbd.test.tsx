import { expect, test } from "@destack/test";
import { Kbd, KbdGroup } from "./index.ts";
import { markup, render } from "@destack/view/test";

test("render the keys of a shortcut as nested kbd elements", () => {
    const { container } = render(() => (
        <KbdGroup>
            <Kbd>⌘</Kbd>
            <Kbd>K</Kbd>
        </KbdGroup>
    ));
    expect(markup(container)).toBe(
        '<kbd data-slot="kbd-group"><kbd data-slot="kbd">⌘</kbd><kbd data-slot="kbd">K</kbd></kbd>',
    );
});
