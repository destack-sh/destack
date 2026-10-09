import { expect, test } from "@destack/test";
import { render } from "@destack/view/test";
import { flush } from "@destack/view";
import type { AccentPreset } from "@destack/theme";
import { ColorSwatchPicker } from "./index.ts";

/** List the preset of each chosen swatch of a container. */
function checked(container: Element): string[] {
    return [...container.querySelectorAll("[role=option][aria-selected=true]")].map(
        (swatch) => swatch.getAttribute("data-swatch") ?? "",
    );
}

test("offer the given swatches as options, holding a controlled choice and reporting a person's", () => {
    const chosen: AccentPreset[] = [];
    const { container } = render(() => (
        <ColorSwatchPicker
            presets={["teal", "orange", "plum"]}
            value="orange"
            onValueChange={(value) => chosen.push(value)}
        />
    ));
    const swatches = [...container.querySelectorAll<HTMLElement>("[role=option]")];
    swatches[2]?.click();
    flush();

    expect({
        offered: swatches.map((swatch) => swatch.getAttribute("aria-label")),
        checked: checked(container),
        chosen,
    }).toEqual({ offered: ["teal", "orange", "plum"], checked: ["orange"], chosen: ["plum"] });
});

test("submit the chosen swatch through a hidden input of its name, none while none is chosen", () => {
    const { container } = render(() => (
        <form>
            <ColorSwatchPicker name="color" presets={["teal", "plum"]} aria-label="Color" />
        </form>
    ));
    const submitted = () =>
        container.querySelector<HTMLInputElement>("form input[type=hidden][name=color]")?.value;
    const before = submitted();
    container.querySelectorAll<HTMLElement>("[role=option]")[1]?.click();
    flush();

    expect({
        before,
        after: submitted(),
    }).toEqual({ before: undefined, after: "plum" });
});
