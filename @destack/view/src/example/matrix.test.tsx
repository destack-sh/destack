import { t } from "@destack/locale";
import { defineExample } from "@destack/package/declare";
import { expect, onTestFinished, test } from "@destack/test";
import type { JSX } from "@solidjs/web";
import { testBrowser } from "../test/browser.ts";
import type { MatrixAxes } from "../page/message.ts";
import type { ExampleFrame } from "./frame.ts";
import { Matrix } from "./matrix.ts";

/** The properties of a greeting. */
interface GreetingProperties {
    /** How loudly, quietly when absent. */
    readonly tone?: "quiet" | "loud";
}

/** Greet Ada, quietly or loudly. */
function Greeting(properties: GreetingProperties): JSX.Element {
    return <p>{properties.tone === "loud" ? "ADA" : "Ada"}</p>;
}

/** A greeting of Ada. */
export const greetingDefault = defineExample({
    of: Greeting,
    name: "default",
    description: t`A greeting of Ada.`,
    properties: { tone: "quiet" },
    render: (properties) => <Greeting {...properties} />,
});

/** The tones of a greeting, as columns. */
const TONES: MatrixAxes = {
    columns: {
        name: "tone",
        values: [
            { value: "quiet", title: "Quiet" },
            { value: "loud", title: "Loud" },
        ],
    },
};

/** Draw an example's matrix along the tones into the document, removed after the test. */
function draw(name: string, frame: ExampleFrame): HTMLElement {
    // draw the matrix into its own element
    const element = document.createElement("main");
    document.body.append(element);
    const matrix = new Matrix(element, name, frame, { tone: "quiet" }, TONES, testBrowser(), {
        choose: () => undefined,
        open: () => undefined,
    });
    onTestFinished(() => {
        matrix.dispose();
        element.remove();
    });

    return element;
}

/** Read each cell of a drawn matrix: its label, whether it is inert, and what it shows. */
function cellsOf(element: HTMLElement): (string | boolean | null)[][] {
    return [...element.querySelectorAll<HTMLButtonElement>("button")].map((pick) => {
        const body = pick.previousElementSibling;

        return [
            pick.ariaLabel,
            body instanceof HTMLElement && body.inert,
            body?.textContent ?? null,
        ];
    });
}

test("draw each cell in the page, look-only, with the cell's values", () => {
    const element = draw("greeting.default", { example: greetingDefault });

    // each cell renders the example in place with its tone, out of the focus and the pointer
    expect(cellsOf(element)).toEqual([
        ["Quiet", true, "Ada"],
        ["Loud", true, "ADA"],
    ]);
});
