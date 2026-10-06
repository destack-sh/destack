import { expect, test } from "@destack/test";
import type { JSX } from "@solidjs/web";
import type { Example } from "@destack/package/declare";
import manifest from "../package.json" with { type: "json" };
import { renderToString } from "@solidjs/web";
import { Sidebar, SidebarContent, SidebarProvider, SidebarTrigger } from "../src/sidebar/index.ts";
import { Toaster } from "../src/toast/index.ts";

/** The class attributes StyleX writes, whose names hash the styles. */
const CLASS_ATTRIBUTE = / class="[^"]*"/gu;

/** The bodies of inline icons. */
const ICON_BODY = /(<svg[^>]*>).*?(<\/svg>)/gu;

/** The examples module of each component the package exports, loaded once before the tests run. */
const EXAMPLES: readonly (readonly [string, object])[] = await Promise.all(
    Object.keys(manifest.exports)
        .map((path) => path.slice(2))
        .map(async (name) => [name, await import(`../src/${name}/${name}.example.tsx`)] as const),
);

test("render a sidebar and a toaster to a string without a browser, open and with no toasts", () => {
    const html = renderToString(() => (
        <SidebarProvider>
            <Sidebar>
                <SidebarContent>Notebooks</SidebarContent>
            </Sidebar>
            <SidebarTrigger />
            <Toaster />
        </SidebarProvider>
    ));

    // the server renders the defaults: an expanded sidebar on a wide screen and an empty stack
    expect(html.replaceAll(CLASS_ATTRIBUTE, "").replaceAll(ICON_BODY, "$1$2")).toBe(
        '<div data-slot="sidebar-wrapper">' +
            '<div id="0" data-slot="sidebar" data-state="expanded" data-variant="sidebar" data-side="left">' +
            '<div><div data-slot="sidebar-content">Notebooks</div></div></div>' +
            '<button data-slot="sidebar-trigger" data-variant="ghost" data-size="icon-sm" aria-label="Toggle sidebar" aria-controls="0" aria-expanded="true">' +
            '<svg viewBox="0 0 256 256" fill="currentColor" width="1em" height="1em" aria-hidden="true"></svg></button>' +
            '<section aria-label="Notifications" aria-live="polite" aria-relevant="additions text" aria-atomic="false" tabindex="-1" data-slot="toaster" data-position="bottom-right">' +
            '<ol popover="manual" data-slot="toaster-stack"></ol></section></div>',
    );
});

test("render every exported component's examples to strings without a browser", async () => {
    // render the examples of each component the package exports
    const rendered: string[] = [];
    for (const [name, examples] of EXAMPLES) {
        for (const [exported, example] of Object.entries(examples)) {
            // skip the module URL the server compile adds to component modules
            const render = isExample(example) ? example.render : undefined;
            if (render === undefined) {
                continue;
            }
            expect(
                renderToString(() => render()),
                exported,
            ).not.toBe("");
            rendered.push(name);
        }
    }

    // each component shows at least one example
    expect(new Set(rendered).size).toBe(Object.keys(manifest.exports).length);
});

/** Report whether an export is an example, which renders an element. */
function isExample(value: unknown): value is Example<object, JSX.Element> {
    return typeof value === "object" && value !== null && "render" in value && "of" in value;
}
