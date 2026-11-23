import { expect, test, vi } from "@destack/test";
import { flush } from "@destack/view";
import { render, renderOnServer, wait } from "@destack/view/test";
import * as fixture from "./fixture/collected.tsx";

/** The server's HTML of each component that reads the children mounted in it. */
const PAGES = await renderOnServer("tests/fixture/collected.tsx");

/** The attributes a render generates or writes apart from its markup. */
const GENERATED = new Set([
    "_hk",
    "aria-activedescendant",
    "aria-controls",
    "aria-describedby",
    "aria-labelledby",
    "commandfor",
    "data-anchor-name",
    "data-position-anchor",
    "for",
    "id",
    "marker-end",
    "name",
    "popovertarget",
    "style",
    "value",
]);

/** List a container's elements with their attributes in order, and the text of each leaf. */
function elementsOf(container: Element): string[] {
    return [...container.querySelectorAll("*")].map((element) => {
        // write the attributes by name, without the generated ones
        const attributes = [...element.attributes]
            .filter((attribute) => !GENERATED.has(attribute.name))
            .map((attribute) => `${attribute.name}=${attribute.value}`)
            .toSorted();
        const text = element.childElementCount === 0 ? element.textContent : "";

        return [element.localName, ...attributes, text].join(" ");
    });
}

test.each(Object.entries(fixture))(
    "hydrate a server's %s to the elements a browser renders, without a warning",
    async (name, Component) => {
        // hydrate the server's markup and let the mount run
        const warn = vi.spyOn(console, "warn").mockImplementation(() => {});
        const container = document.createElement("div");
        container.innerHTML = PAGES[name] ?? "";
        render(() => <Component />, { container, hydrate: true });
        await wait(0);
        flush();
        const warnings = [...warn.mock.calls];
        warn.mockRestore();

        // render the same anew
        const fresh = render(() => <Component />).container;
        await wait(0);
        flush();

        expect({ elements: elementsOf(container), warnings }).toEqual({
            elements: elementsOf(fresh),
            warnings: [],
        });
    },
);
