import { present } from "@destack/schema";
import { expect, test } from "@destack/test";
import { Locale, Localization } from "@destack/locale";
import { flush, LocaleContext } from "@destack/view";
import { render } from "@destack/view/test";
import { type Constraints, ConstraintField } from "./index.ts";

test("pin both sides by their edges, then unpin one and center a side pinned to neither", () => {
    // start at the top left, pin the right, unpin the left, unpin the right, then pin the bottom
    const given: Constraints[] = [];
    const { container } = render(() => (
        <ConstraintField aria-label="Constraints" onValueChange={(next) => void given.push(next)} />
    ));
    const press = (label: string): void => {
        present(container.querySelector<HTMLElement>(`[aria-label="${label}"]`), label).click();
        flush();
    };
    press("Pin to the right");
    press("Pin to the left");
    press("Pin to the right");
    press("Pin to the bottom");

    expect(given).toEqual([
        { x: "both", y: "start" },
        { x: "end", y: "start" },
        { x: "center", y: "start" },
        { x: "center", y: "both" },
    ]);
});

test("name the start edge by the side it sits on, the right in right to left text", () => {
    const given: Constraints[] = [];
    const { container } = render(() => (
        <LocaleContext value={Localization.of(Locale.parse("ar"), [])}>
            <ConstraintField onValueChange={(next) => given.push(next)} />
        </LocaleContext>
    ));
    const edges = [
        ...container.querySelectorAll<HTMLButtonElement>("[data-slot=constraint-field-edge]"),
    ];
    const names = edges.map((edge) =>
        edge.getAttribute("aria-label")?.replaceAll(/[\u2068\u2069]/gu, ""),
    );
    const right = edges[names.indexOf("Pin to the right")];
    right?.click();
    flush();

    // the right edge is where an Arabic line starts, so pressing it unpins the start and centers across
    expect([names, given]).toEqual([
        ["Pin to the top", "Pin to the right", "Pin to the left", "Pin to the bottom"],
        [{ x: "center", y: "start" }],
    ]);
});
