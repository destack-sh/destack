import type { JSX } from "@solidjs/web";
import { defineExample } from "@destack/package/declare";
import { expect, onTestFinished, test, vi } from "@destack/test";
import { Typeface } from "@destack/theme";
import { testBrowser } from "../test/browser.ts";
import { wait } from "../test/dom.ts";
import { hostExamples, paint, queryOf } from "./host.tsx";

/** Greet everyone. */
function Greeting(): JSX.Element {
    return <p>Hello</p>;
}

/** A plain greeting. */
export const greetingPlain = defineExample({
    of: Greeting,
    name: "plain",
    description: "a plain greeting",
    render: () => <Greeting />,
});

/** Read the page of examples a URL's query asks for. */
function pageOf(search: string): URL {
    return new URL(`http://localhost/.destack/examples/board/launch${search}`);
}

test("paint a query's theme over an element, removing the theme's properties this paint leaves out", () => {
    // paint reduced motion, then the defaults, which follow the device's motion
    const element = document.createElement("div");
    paint(element, queryOf(pageOf("?motion=reduced")));
    const scaleWhileReduced = element.style.getPropertyValue("--destack-motion-scale");
    paint(element, queryOf(pageOf("")));

    expect([scaleWhileReduced, element.style.getPropertyValue("--destack-motion-scale")]).toEqual([
        "0",
        "",
    ]);
});

test("refuse a display setting outside its choices", () => {
    // paint a face no person can choose, and read the refusal's issues
    let issues: unknown = undefined;
    try {
        paint(document.createElement("div"), queryOf(pageOf("?typeface=comic")));
    } catch (error) {
        issues = error instanceof Error && "issues" in error ? error.issues : error;
    }

    expect(issues).toEqual([
        {
            code: "invalid_value",
            values: Typeface.options,
            path: ["typeface"],
            message: `Invalid option: expected one of ${Typeface.options.map((face) => `"${face}"`).join("|")}`,
        },
    ]);
});

test("host the example its path names, rendering it again in place as the locale changes", async () => {
    // open the example's page
    const environment = testBrowser();
    const { navigation } = environment;
    navigation.navigate("/.destack/examples/example/plain?width=320", { history: "replace" });
    const page = document.createElement("main");
    document.body.append(page);
    const dispose = hostExamples(
        page,
        [{ module: "/src/example/host.test.tsx", export: "greetingPlain", name: "plain" }],
        environment,
    );
    onTestFinished(() => page.remove());

    // read the frames the page shows, loaded and then in another locale
    const frames = () =>
        [...page.querySelectorAll<HTMLElement>("[data-slot=example]")].map((frame) => [
            frame.lang,
            frame.style.width,
            frame.textContent,
        ]);
    await vi.waitFor(() => expect(frames()).toEqual([["en", "320px", "Hello"]]));
    const options = [...page.querySelectorAll<HTMLButtonElement>("[role=option]")];
    options.find((option) => option.textContent === "ar-EG")?.click();
    await wait(0);
    const changed = frames();
    dispose();

    expect([changed, frames()]).toEqual([[["ar-EG", "320px", "Hello"]], []]);
});
