import { expect, test } from "@destack/test";
import { present } from "@destack/schema";
import { createSignal, flush } from "@destack/view";
import { render } from "@destack/view/test";
import {
    Viewer,
    ViewerClose,
    ViewerControl,
    ViewerCount,
    ViewerHeader,
    ViewerNext,
    ViewerPrevious,
    ViewerStage,
    ViewerTitle,
} from "./index.ts";

/** The marks a locale isolates a value's direction with. */
const ISOLATES = /[\u2066-\u2069]/gu;

/** Open a viewer of named items on the second. */
function draw(isLooping = false) {
    const [names, setNames] = createSignal(["Lake", "Pass", "Tram"]);
    const controls: ViewerControl<string>[] = [];
    const { container } = render(() => {
        const viewer = new ViewerControl<string>({
            items: names,
            key: (name) => name,
            isLooping,
        });
        controls.push(viewer);

        return (
            <>
                <div id="thumbnail" ref={viewer.thumbnail("Tram")}>
                    <button type="button">Tram</button>
                </div>
                <Viewer control={viewer}>
                    {(name) => (
                        <>
                            <ViewerHeader>
                                <ViewerTitle>{name()}</ViewerTitle>
                                <ViewerCount />
                                <ViewerClose />
                            </ViewerHeader>
                            <ViewerStage control={viewer}>
                                {(shown) => <p>{shown()}</p>}
                            </ViewerStage>
                            <ViewerPrevious />
                            <ViewerNext />
                        </>
                    )}
                </Viewer>
            </>
        );
    });
    const viewer = present(controls[0], "the viewer");
    viewer.open("Pass");
    flush();
    const shown = () => {
        const dialog = container.querySelector("dialog");
        const read = (slot: string) =>
            dialog?.querySelector(`[data-slot=${slot}]`)?.textContent.replace(ISOLATES, "");

        return dialog?.open === true ? [read("viewer-title"), read("viewer-count")] : "closed";
    };
    const press = (key: string) => {
        const dialog = present(container.querySelector("dialog"), "the dialog");
        dialog.dispatchEvent(
            new KeyboardEvent("keydown", { key, bubbles: true, cancelable: true }),
        );
        flush();

        return shown();
    };

    return { container, viewer, shown, press, setNames };
}

test("move between items by keys", () => {
    const { shown, press, container } = draw();

    expect([
        shown(),
        ...["ArrowRight", "ArrowRight", "Home", "ArrowLeft", "End"].map(press),
        container.querySelector("[data-slot=viewer-next]"),
    ]).toEqual([
        ["Pass", "2 of 3"],
        ["Tram", "3 of 3"],
        ["Tram", "3 of 3"],
        ["Lake", "1 of 3"],
        ["Lake", "1 of 3"],
        ["Tram", "3 of 3"],
        null,
    ]);
});

test("loop from the last item to the first", () => {
    const { press } = draw(true);

    expect(["End", "ArrowRight", "ArrowLeft"].map(press)).toEqual([
        ["Tram", "3 of 3"],
        ["Lake", "1 of 3"],
        ["Tram", "3 of 3"],
    ]);
});

test("close once its item is gone", () => {
    const { shown, setNames } = draw();
    setNames(["Lake", "Tram"]);
    flush();
    flush();

    expect(shown()).toBe("closed");
});

test("focus the thumbnail of the item shown last", async () => {
    const { container, press } = draw();
    press("End");
    container.querySelector<HTMLElement>("[data-slot=viewer-close]")?.click();
    flush();
    await Promise.resolve();

    expect(document.activeElement?.textContent).toBe("Tram");
});

test("render the items beside the one shown", () => {
    const { container, press } = draw(true);
    const panes = () =>
        [...container.querySelectorAll("[data-slot=viewer-pane]")].map((pane) => [
            pane.textContent,
            pane.hasAttribute("data-current"),
            pane.getAttribute("aria-hidden"),
        ]);
    const before = panes();
    press("ArrowRight");

    expect([before, panes()]).toEqual([
        [
            ["Lake", false, "true"],
            ["Pass", true, null],
            ["Tram", false, "true"],
        ],
        [
            ["Pass", false, "true"],
            ["Tram", true, null],
            ["Lake", false, "true"],
        ],
    ]);
});
