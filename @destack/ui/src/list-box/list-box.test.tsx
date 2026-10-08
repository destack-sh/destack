import { expect, test } from "@destack/test";
import { flush } from "@destack/view";
import { render } from "@destack/view/test";
import { ListBox, ListBoxItem, ListBoxSection } from "./index.ts";

test("expose a single selection's options with the selected one holding the tab stop, and select another on click", () => {
    // render two sections of notebooks and click the third
    const chosen: (string | undefined)[] = [];
    const actions: string[] = [];
    const { container } = render(() => (
        <ListBox
            aria-label="Notebook"
            selection={{ defaultValue: "work", onValueChange: (value) => chosen.push(value) }}
            onAction={(option) => actions.push(option.value())}
        >
            <ListBoxSection>
                <ListBoxItem value="trips">Trips</ListBoxItem>
                <ListBoxItem value="work">Work</ListBoxItem>
            </ListBoxSection>
            <ListBoxItem value="taxes" disabled onSelect={(value) => actions.push(`own ${value}`)}>
                Taxes
            </ListBoxItem>
            <ListBoxItem value="recipes" onSelect={(value) => actions.push(`own ${value}`)}>
                Recipes
            </ListBoxItem>
        </ListBox>
    ));
    flush();
    for (const option of container.querySelectorAll<HTMLElement>("[role=option]")) {
        option.click();
        flush();
    }

    // the unavailable option ignores its click, and the last click wins the single selection
    expect({
        options: [...container.querySelectorAll("[role=option]")].map((option) =>
            [
                option.textContent,
                option.getAttribute("aria-selected"),
                option.getAttribute("tabindex"),
                option.getAttribute("aria-disabled"),
            ].join(" "),
        ),
        chosen,
        actions,
    }).toEqual({
        options: ["Trips false -1 ", "Work false -1 ", "Taxes false -1 true", "Recipes true 0 "],
        chosen: ["trips", "work", "recipes"],
        actions: ["trips", "work", "own recipes", "recipes"],
    });
});

test("toggle the values of a multiple selection, marking the list box multiselectable", () => {
    const { container } = render(() => (
        <ListBox aria-label="Tags" selection={{ multiple: true, defaultValue: ["travel"] }}>
            <ListBoxItem value="travel">travel</ListBoxItem>
            <ListBoxItem value="food">food</ListBoxItem>
        </ListBox>
    ));
    for (const option of container.querySelectorAll<HTMLElement>("[role=option]")) {
        option.click();
        flush();
    }

    expect({
        multiselectable: container
            .querySelector("[role=listbox]")
            ?.getAttribute("aria-multiselectable"),
        selected: [...container.querySelectorAll("[role=option]")].map((option) =>
            option.getAttribute("aria-selected"),
        ),
    }).toEqual({ multiselectable: "true", selected: ["false", "true"] });
});
