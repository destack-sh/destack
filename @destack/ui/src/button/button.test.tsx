import { BrowserSimulation } from "@destack/environment/browser/simulation";
import { testSimulation } from "@destack/test/world";
import { expect, test } from "@destack/test";
import { createSignal, flush } from "@destack/view";
import * as style from "@destack/style";
import { Button, buttonVariants } from "./index.ts";
import { classes, markup, render } from "@destack/view/test";
import { renderPart } from "../part/index.ts";

/** The styles of a part that draws a button. */
const outer = style.create({ raised: { opacity: 0.5 } });

test("render a native button with its variant, size and attributes", () => {
    const { container } = render(() => (
        <Button variant="outline" size="sm" type="submit" aria-label="Save note" disabled>
            Save
        </Button>
    ));
    expect(markup(container)).toBe(
        '<button data-slot="button" data-variant="outline" data-size="sm" disabled="" type="submit" aria-label="Save note">Save</button>',
    );
});

test("keep a button's styles under a styled part that draws it", () => {
    const { container } = render(() => (
        <>
            <Button xstyle={outer.raised} />
            {renderPart("div", "outer", { render: (part) => <Button {...part} /> }, outer.raised)}
        </>
    ));
    const [plain, drawn] = container.querySelectorAll("button");

    expect(drawn?.className).toBe(plain?.className);
});

test("restyle a button when its variant changes", () => {
    const [variant, setVariant] = createSignal<"default" | "destructive">("default");
    const { container } = render(() => <Button variant={variant()} />);
    const before = classes(container);
    setVariant("destructive");
    flush();

    // the button follows its variant without rendering again
    expect(classes(container)).not.toEqual(before);
    expect(markup(container)).toBe(
        '<button data-slot="button" data-variant="destructive" data-size="default" type="button"></button>',
    );
});

test("style a link like a button of the same variant and size", () => {
    const { container } = render(() => (
        <>
            <Button variant="outline" size="sm" />
            <a
                href="/pricing"
                {...style.attrs(buttonVariants({ variant: "outline", size: "sm" }))}
            />
        </>
    ));

    // the link takes the button's classes exactly
    const [button, link] = classes(container);
    expect(link).toBe(button);
});

test("render a link with the button's part attributes, its native button's attributes left out", () => {
    const { container } = render(() => (
        <Button
            variant="outline"
            type="submit"
            aria-label="Open notes"
            render={(part) => <a href="/notes" {...part} />}
        >
            Notes
        </Button>
    ));
    expect(markup(container)).toBe(
        '<a href="/notes" data-slot="button" data-variant="outline" data-size="default" aria-label="Open notes">Notes</a>',
    );
});

test("mark a loading button busy and refuse clicks, with a spinner once it takes a while", async () => {
    await using simulation = testSimulation();
    const tab = new BrowserSimulation(simulation).environment("tab");
    const [isLoading, setLoading] = createSignal(true);
    const clicks: string[] = [];
    const { container } = render(
        () => (
            <Button loading={isLoading()} onClick={() => clicks.push("save")}>
                Save
            </Button>
        ),
        { environment: tab },
    );
    const button = container.querySelector("button");
    const read = () => [
        button?.getAttribute("aria-busy"),
        button?.getAttribute("aria-disabled"),
        button?.disabled,
        container.querySelector("[data-slot=spinner]") !== null,
    ];
    const pass = async (milliseconds: number) => {
        await simulation.jump(simulation.clock.now() + milliseconds);
        flush();

        return read();
    };
    button?.click();
    const atOnce = read();
    const later = await pass(200);
    setLoading(false);
    flush();
    const done = read();
    button?.click();
    const gone = await pass(400);

    expect({ atOnce, later, done, gone, clicks }).toEqual({
        atOnce: ["true", "true", false, false],
        later: ["true", "true", false, true],
        done: [null, null, false, true],
        gone: [null, null, false, false],
        clicks: ["save"],
    });
});
