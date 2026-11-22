import { expect, test } from "@destack/test";
import { Kbd, KbdGroup, KbdShortcut } from "./index.ts";
import { flush } from "@destack/view";
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

test("render a key combination a key each, as the platform labels them once the page settles", () => {
    // render on the test's platform, which labels mod as Control
    const { container } = render(() => <KbdShortcut keys="mod+shift+enter" />);
    flush();

    expect(markup(container)).toBe(
        '<kbd data-slot="kbd-shortcut"><kbd data-slot="kbd">Ctrl</kbd><kbd data-slot="kbd">Shift</kbd><kbd data-slot="kbd">Enter</kbd></kbd>',
    );
});
