import { expect, test } from "@destack/test";
import { present } from "@destack/schema";
import { ActionRegistry, createSignal, flush } from "@destack/view";
import { render } from "@destack/view/test";
import { OpenChange } from "../layer/index.ts";
import {
    Viewer,
    ViewerClose,
    ViewerControl,
    ViewerCount,
    ViewerDownload,
    ViewerHeader,
    ViewerNext,
    ViewerPrevious,
    ViewerStage,
    ViewerStrip,
    ViewerTitle,
} from "./index.ts";

/** The marks a locale isolates a value's direction with. */
const ISOLATES = /[\u2066-\u2069]/gu;

/** Open a viewer of named items on the second. */
function draw(isLooping = false) {
    const [names, setNames] = createSignal(["Lake", "Pass", "Tram"]);
    const controls: ViewerControl<string>[] = [];
    const actions = new ActionRegistry();
    const { container } = render(
        () => {
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
                                    <ViewerDownload href={`/${name()}`} />
                                    <ViewerClose />
                                </ViewerHeader>
                                <ViewerStage control={viewer}>
                                    {(shown) => <p>{shown()}</p>}
                                </ViewerStage>
                                <ViewerStrip control={viewer} label={(each) => each}>
                                    {(each) => <b>{each()}</b>}
                                </ViewerStrip>
                                <ViewerPrevious />
                                <ViewerNext />
                            </>
                        )}
                    </Viewer>
                </>
            );
        },
        { actions },
    );
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

    return { container, viewer, actions, shown, press, setNames };
}

test("move between items by keys", () => {
    const { shown, press, container } = draw();

    expect([
        shown(),
        container.querySelectorAll("[data-slot=viewer-next]").length,
        ...["ArrowRight", "ArrowRight", "Home", "ArrowLeft", "End"].map(press),
        container.querySelectorAll("[data-slot=viewer-next]").length,
    ]).toEqual([
        ["Pass", "2 of 3"],
        1,
        ["Tram", "3 of 3"],
        ["Tram", "3 of 3"],
        ["Lake", "1 of 3"],
        ["Lake", "1 of 3"],
        ["Tram", "3 of 3"],
        0,
    ]);
});

test("move between items by actions", () => {
    const { press } = draw();

    expect(["j", "j", "k"].map(press)).toEqual([
        ["Tram", "3 of 3"],
        ["Tram", "3 of 3"],
        ["Pass", "2 of 3"],
    ]);
});

test("offer what runs to the palette", () => {
    const { container, viewer, actions, press } = draw();
    const offered = () =>
        actions
            .actions(container.querySelector("dialog"))
            .map(({ name }) => name)
            .toSorted();
    const middle = offered();
    press("End");
    const end = offered();
    viewer.dialog.close(new OpenChange("none"));
    flush();

    expect([middle, end, actions.actions(null).map(({ name }) => name)]).toEqual([
        ["item.download", "item.next", "item.previous"],
        ["item.download", "item.previous"],
        [],
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

test("show the item of a thumbnail", () => {
    const { container, shown } = draw();
    const thumbnails = () =>
        [...container.querySelectorAll<HTMLElement>("[data-slot=viewer-thumbnail]")].map((each) => [
            each.getAttribute("aria-label"),
            each.getAttribute("aria-current"),
            each.tabIndex,
        ]);
    const before = thumbnails();
    container.querySelector<HTMLElement>("[data-slot=viewer-thumbnail]")?.click();
    flush();

    expect([before, shown(), thumbnails()]).toEqual([
        [
            ["Lake", null, -1],
            ["Pass", "true", 0],
            ["Tram", null, -1],
        ],
        ["Lake", "1 of 3"],
        [
            ["Lake", "true", 0],
            ["Pass", null, -1],
            ["Tram", null, -1],
        ],
    ]);
});

test("keep the thumbnails as the item is left alone", () => {
    const { container, viewer, press } = draw();
    const thumbnails = () => [...container.querySelectorAll("[data-slot=viewer-thumbnail]")];
    const before = thumbnails();
    viewer.setAlone(true);
    flush();
    press("ArrowRight");

    expect(thumbnails().map((each, i) => each === before[i])).toEqual([true, true, true]);
});

test("leave the item alone until the pointer moves or Tab", () => {
    const { container, viewer, press } = draw();
    const dialog = present(container.querySelector("dialog"), "the dialog");
    const alone = [viewer.isAlone()];
    const step = (run: () => void) => {
        run();
        flush();
        alone.push(viewer.isAlone());
    };
    step(() => viewer.setAlone(true));
    step(() => dialog.dispatchEvent(new PointerEvent("pointermove", { bubbles: true })));
    step(() => viewer.setAlone(true));
    step(() => press("ArrowRight"));
    step(() => press("Tab"));

    expect(alone).toEqual([false, true, false, true, true, false]);
});
