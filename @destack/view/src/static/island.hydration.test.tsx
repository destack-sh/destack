import { expect, test, vi } from "@destack/test";
import { flush } from "solid-js";
import { Localization, SOURCE_LOCALE } from "@destack/locale";
import { testBrowser } from "../test/browser.ts";
import { installHydration, wait } from "../test/dom.ts";
import { hydrateIsland } from "./hydrate.ts";
import { present } from "@destack/schema";
import { renderOnServer } from "../test/hydration.ts";
import { Package } from "@destack/package";
import { Counter, Disclosure } from "./island.fixture.tsx";
import { createComponent } from "solid-js";
import { IslandSlot } from "./island.tsx";

/** The package the island records its telemetry for. */
const PAGE_PACKAGE = Package.parse({
    id: "package-019f7480-0000-7000-8000-0000000000c1",
    name: "@example/islands",
    version: "2026.9.0",
});

/** The server's HTML of the page whose counter alone hydrates. */
const { Page: html = "" } = await renderOnServer("src/static/island.fixture.tsx");

test("render a page without hydration but for its island, then hydrate the island alone by its id", async () => {
    // collect the warnings hydration gives for nodes it misses
    const warn = vi.spyOn(console, "warn").mockImplementation(() => {});

    // hydrate the island's root under the bootstrap the page's hydration script installs, then click it twice
    const container = document.createElement("div");
    container.innerHTML = html;
    document.body.append(container);
    const root = present(
        container.querySelector<HTMLElement>('[data-island="/src/islands/counter.tsx"]'),
        "the island root",
    );
    const uninstall = installHydration();
    const dispose = hydrateIsland(
        root,
        Counter,
        { start: 3 },
        {
            package: PAGE_PACKAGE,
            environment: testBrowser(),
            localization: Localization.of(SOURCE_LOCALE, []),
        },
    );
    await wait(0);
    flush();
    const button = present(root.querySelector("button"), "the counter");
    button.click();
    flush();
    button.click();
    flush();
    const warnings = [...warn.mock.calls];
    const text = container.textContent;
    warn.mockRestore();
    dispose();
    uninstall();

    // keep the page's heading as plain HTML, and count in the island
    expect({ html, text, warnings }).toEqual({
        html: [
            "<main><h1>Static</h1><!--$-->",
            '<div data-island="/src/islands/counter.tsx" data-island-id="/src/islands/counter.tsx#0" data-island-properties="{&quot;start&quot;:3}" style="display:contents">',
            "<button _hk=/src/islands/counter.tsx#00000000>Count: <!--$-->3<!--/--></button>",
            "</div><!--/-->",
            '<!--$--><div data-island="/src/islands/disclosure.tsx" data-island-id="/src/islands/disclosure.tsx#1" data-island-properties="{&quot;label&quot;:&quot;More&quot;}" data-island-children="" style="display:contents">',
            "<section _hk=/src/islands/disclosure.tsx#10000000><button><!--$-->More<!--/-->: <!--$-->closed<!--/--></button>",
            '<!--$--><div data-island-slot style="display:contents"><p>Static <b>text</b></p></div><!--/--></section>',
            "</div><!--/--></main>",
        ].join(""),
        text: "StaticCount: 5More: closedStatic text",
        warnings: [],
    });
});

test("hydrate an island around children the server rendered, keeping them as they are", async () => {
    // hydrate the disclosure's root with its slot, then open it
    const warn = vi.spyOn(console, "warn").mockImplementation(() => {});
    const container = document.createElement("div");
    container.innerHTML = html;
    document.body.append(container);
    const root = present(
        container.querySelector<HTMLElement>('[data-island="/src/islands/disclosure.tsx"]'),
        "the disclosure's root",
    );
    const uninstall = installHydration();
    const dispose = hydrateIsland(
        root,
        Disclosure,
        { label: "More", children: createComponent(IslandSlot, {}) },
        {
            package: PAGE_PACKAGE,
            environment: testBrowser(),
            localization: Localization.of(SOURCE_LOCALE, []),
        },
    );
    await wait(0);
    flush();
    present(root.querySelector("button"), "the disclosure's button").click();
    flush();
    const warnings = [...warn.mock.calls];
    const section = root.innerHTML;
    warn.mockRestore();
    dispose();
    uninstall();
    container.remove();

    // open the disclosure, its static children kept
    expect({ section, warnings }).toEqual({
        section:
            '<section _hk="/src/islands/disclosure.tsx#10000000"><button><!--$-->More<!--/-->: <!--$-->open<!--/--></button><!--$--><div data-island-slot="" style="display:contents"><p>Static <b>text</b></p></div><!--/--></section>',
        warnings: [],
    });
});
