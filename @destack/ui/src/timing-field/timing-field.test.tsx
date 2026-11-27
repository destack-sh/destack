import type { TimingFunction } from "@destack/style";
import { expect, test } from "@destack/test";
import { createSignal, flush } from "@destack/view";
import { render } from "@destack/view/test";
import { TimingField } from "./index.ts";

test("switch kinds, each starting at its own first timing", () => {
    const [timing, setTiming] = createSignal<TimingFunction>([0.25, 0.1, 0.25, 1]);
    const { container } = render(() => (
        <TimingField label="Easing" value={timing()} onValueChange={setTiming} />
    ));
    choose(container, "Steps");

    expect(timing()).toEqual({ kind: "steps", count: 4, position: "jump-end" });

    choose(container, "Spring");

    expect(timing()).toEqual({ kind: "spring", duration: { milliseconds: 400 }, bounce: 0.2 });
});

/** Press the option of a kind by its title. */
function choose(container: HTMLElement, title: string): void {
    const option = [...container.querySelectorAll<HTMLElement>("button, [role=radio]")].find(
        (element) => element.textContent.trim() === title,
    );
    option?.click();
    flush();
}
