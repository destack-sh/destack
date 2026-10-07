import { expect, onTestFinished, test } from "@destack/test";
import type { JSX } from "@solidjs/web";
import { useLocale } from "../page/locale.ts";
import { defineExample } from "@destack/package/declare";
import { type ExampleFrame, renderExample } from "./frame.ts";

/** The properties of a greeting. */
interface GreetingProperties {
    /** The person greeted. */
    readonly name: string;
    /** How loudly. */
    readonly tone?: "quiet" | "loud";
}

/** Greet a person in the reader's language. */
function Greeting(properties: GreetingProperties): JSX.Element {
    const locale = useLocale();

    return (
        <p>
            {locale.tag} {locale.direction}:{" "}
            {properties.tone === "loud" ? properties.name.toUpperCase() : properties.name}
        </p>
    );
}

/** A loud greeting of Ada. */
const greetingLoud = defineExample({
    of: Greeting,
    name: "loud",
    description: "a loud greeting of Ada",
    properties: { tone: "loud" },
    render: (properties) => <Greeting name="Ada" {...properties} />,
});

/** Render examples into the document and remove them after the test, returning their frames. */
function frames(...rendered: ExampleFrame[]): HTMLElement[] {
    return rendered.map((properties) => {
        // mount the example into its own element
        const element = document.createElement("main");
        document.body.append(element);
        onTestFinished(renderExample(element, properties));
        onTestFinished(() => element.remove());

        // read the frame the example renders in
        const frame = element.firstElementChild;
        if (!(frame instanceof HTMLElement)) {
            throw new TypeError("the example renders no frame");
        }

        return frame;
    });
}

test("render an example in its environment's locale, direction, width and theme settings, giving components the frame's direction", () => {
    const [arabic, plain, mirrored] = frames(
        {
            example: greetingLoud,
            environment: {
                locale: "ar-EG",
                direction: "rtl",
                width: 320,
                theme: { appearance: "dark", textSize: "large" },
            },
        },
        { example: greetingLoud, properties: { tone: "quiet" } },
        { example: greetingLoud, environment: { locale: "en", direction: "rtl" } },
    );

    // the frame carries the environment, and the example reads its locale, the frame's direction and its properties
    expect(
        [arabic, plain, mirrored].map((element) => [
            element?.textContent,
            element?.getAttribute("lang"),
            element?.getAttribute("dir"),
            element?.style.width,
            element?.style.getPropertyValue("color-scheme"),
        ]),
    ).toEqual([
        ["ar-EG rtl: ADA", "ar-EG", "rtl", "320px", "dark"],
        ["en ltr: Ada", "en", "ltr", "", "light dark"],
        ["en rtl: ADA", "en", "rtl", "", "light dark"],
    ]);
});
