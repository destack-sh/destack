import typescript from "@destack/code/grammar/typescript";
import { testSimulation } from "@destack/test/world";
import { afterAll, expect, test, vi } from "@destack/test";
import { BrowserEnvironment } from "@destack/environment/browser";
import { ActionRegistry, type JSX, PageRoot, renderToString } from "@destack/view";
import type { Example } from "@destack/package/declare";
import manifest from "../package.json" with { type: "json" };
import { CodeBlockContent } from "../src/code-block/index.ts";
import { Diff } from "../src/diff/index.ts";
import { Sidebar, SidebarContent, SidebarProvider, SidebarTrigger } from "../src/sidebar/index.ts";
import { Toaster } from "../src/toast/index.ts";

/** The class attributes StyleX writes, whose names hash the styles. */
const CLASS_ATTRIBUTE = / class="[^"]*"/gu;

/** The bodies of inline icons. */
const ICON_BODY = /(<svg[^>]*>).*?(<\/svg>)/gu;

/** The state units and part builders the package exports, which show no examples. */
const STATE = new Set([
    "action",
    "build",
    "collection",
    "drag",
    "focus",
    "interaction",
    "motion",
    "part",
    "position",
    "selection",
    "toggle-state",
    "virtualizer",
]);

/** The components the package exports, leaving out wrapped chart entries, stylesheets and state. */
const COMPONENTS = Object.keys(manifest.exports)
    .filter((path) => !path.includes("*") && !path.endsWith(".css"))
    .map((path) => path.slice(2))
    .filter((name) => !STATE.has(name));

/** The examples module of each component, loaded once before the tests run. */
const EXAMPLES: readonly (readonly [string, object])[] = await Promise.all(
    COMPONENTS.map(
        async (name) => [name, await import(`../src/${name}/${name}.example.tsx`)] as const,
    ),
);

/** The warnings of every render, among them Solid's one warning of a server write per process. */
const warn = vi.spyOn(console, "warn").mockImplementation(() => {});
afterAll(() => warn.mockRestore());

test("render every line of long code and a long diff to a string, the last ones in color", async () => {
    // render more lines than come near a screen, far past what is parsed at once
    await using simulation = testSimulation();
    const server = BrowserEnvironment.server(simulation.environment("server"));
    const lines = Array.from(
        { length: 200 },
        (_, line) => `const value${String(line)} = 1; // ${"x".repeat(500)}\n`,
    );
    const after = lines.map((line) => line.replace("const", "let"));
    const html = renderToString(() => (
        <PageRoot environment={server} actions={new ActionRegistry()}>
            <CodeBlockContent code={lines.join("")} grammar={typescript} />
            <Diff
                aria-label="Values"
                before={lines.join("")}
                after={after.join("")}
                grammar={typescript}
            />
        </PageRoot>
    ));
    const count = (pattern: RegExp): number => [...html.matchAll(pattern)].length;

    // each line of the code and both lines of each change have a colored semicolon
    expect({
        lines: count(/data-slot="code-block-line"/gu),
        rows: count(/data-kind="(?:added|removed)"/gu),
        marks: count(/data-token="punctuation"/gu),
    }).toEqual({ lines: 200, rows: 400, marks: 600 });
});

test("render a sidebar and a toaster to a string without a browser, open and with no toasts", async () => {
    // render on a server's environment in a world of its own
    await using simulation = testSimulation();
    const server = BrowserEnvironment.server(simulation.environment("server"));
    const html = renderToString(() => (
        <PageRoot environment={server} actions={new ActionRegistry()}>
            <SidebarProvider>
                <Sidebar>
                    <SidebarContent>Notebooks</SidebarContent>
                </Sidebar>
                <SidebarTrigger />
                <Toaster />
            </SidebarProvider>
        </PageRoot>
    ));

    // the server renders the defaults with their hydration keys: an expanded sidebar on a wide screen and an empty stack
    expect(html.replaceAll(CLASS_ATTRIBUTE, "").replaceAll(ICON_BODY, "$1$2")).toBe(
        '<div _hk=0040 data-slot="sidebar-wrapper">' +
            '<div _hk=00430 id="000" data-slot="sidebar" data-open data-variant="sidebar" data-side="left"><div>' +
            '<div _hk=0043140 data-slot="sidebar-content">Notebooks</div></div></div>' +
            '<button _hk=00490 data-variant="ghost" data-size="icon" type="button" aria-label="Toggle sidebar" aria-controls="000" aria-expanded="true" data-slot="sidebar-trigger">' +
            '<svg _hk=00491 viewBox="0 0 256 256" fill="currentColor" width="1em" height="1em" aria-hidden="true" data-slot="icon"></svg></button>' +
            '<section _hk=004i1 aria-label="Notifications" aria-live="polite" aria-relevant="additions text" aria-atomic="false" tabindex="-1" data-slot="toaster" data-position="bottom-right">' +
            '<ol popover="manual" data-slot="toaster-stack" style="--x-height:0px"></ol></section></div>' +
            '<div _hk=004k104 id="001" popover="hint" role="tooltip" data-slot="tooltip-content" data-closed data-side="right" data-align="center"></div>',
    );
});

test("render every exported component's examples to strings without a browser, writing no state", async () => {
    // render the examples of each component the package exports on a server's environment
    await using simulation = testSimulation();
    const server = BrowserEnvironment.server(simulation.environment("server"));
    const rendered = new Set<string>();
    const failures: string[] = [];
    for (const [name, examples] of EXAMPLES) {
        for (const [exported, example] of Object.entries(examples)) {
            // skip the module URL the server compile adds to component modules
            if (!isExample(example) || example.render === undefined) {
                continue;
            }

            // render the example, recording an empty result or an error by its export
            try {
                const html = renderToString(() => (
                    <PageRoot environment={server} actions={new ActionRegistry()}>
                        {example.render?.()}
                    </PageRoot>
                ));
                if (html === "") {
                    failures.push(`${exported}: empty`);
                }
            } catch (error) {
                failures.push(`${exported}: ${String(error)}`);
            }
            rendered.add(name);
        }
    }

    // render every example without a write, which Solid warns of, and at least one of each component
    expect([failures, warn.mock.calls, [...rendered]]).toEqual([[], [], COMPONENTS]);
});

/** Report whether an export is an example, which renders an element. */
function isExample(value: unknown): value is Example<object, JSX.Element> {
    return typeof value === "object" && value !== null && "render" in value && "of" in value;
}
