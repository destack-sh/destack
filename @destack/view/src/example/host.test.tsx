import { expect, test } from "@destack/test";
import { Typeface } from "@destack/theme";
import { paint, queryOf } from "./host.tsx";

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
