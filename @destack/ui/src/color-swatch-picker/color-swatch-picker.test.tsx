import { expect, test } from "@destack/test";
import { render } from "@destack/view/test";
import { flush } from "@destack/view";
import { ColorSwatchPicker } from "./index.ts";

/** List the hue of each chosen swatch of a container. */
function checked(container: Element): string[] {
    return [...container.querySelectorAll("[role=option][aria-selected=true]")].map(
        (swatch) => swatch.getAttribute("data-swatch") ?? "",
    );
}

test("offer the given swatches as options, following a controlled choice and reporting a person's", () => {
    const chosen: string[] = [];
    const { container } = render(() => (
        <ColorSwatchPicker
            colors={["cyan", "orange", "purple"]}
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
    }).toEqual({ offered: ["cyan", "orange", "purple"], checked: ["orange"], chosen: ["purple"] });
});

test("submit the chosen swatch through a hidden input of its name, none while none is chosen", () => {
    const { container } = render(() => (
        <form>
            <ColorSwatchPicker name="color" colors={["cyan", "purple"]} aria-label="Color" />
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
    }).toEqual({ before: undefined, after: "purple" });
});
