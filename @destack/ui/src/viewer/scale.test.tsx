import { expect, test } from "@destack/test";
import { createRoot, flush } from "@destack/view";
import { Scale } from "./index.ts";

/** Scale a picture half as wide as its frame, and read its place after each step. */
function steps(...acts: readonly ((scale: Scale) => void)[]) {
    return createRoot((dispose) => {
        const scale = new Scale();
        scale.attach(() => ({
            picture: { width: 200, height: 100 },
            box: new DOMRect(0, 0, 400, 300),
        }));
        const places = acts.map((act) => {
            act(scale);
            flush();
            const { scale: factor, x, y } = scale.place();

            return [factor, Math.round(x), Math.round(y)];
        });
        dispose();

        return places;
    });
}

test("scale between fit and most", () => {
    expect(
        steps(
            (scale) => scale.zoom(0.5),
            (scale) => scale.zoom(3),
            (scale) => scale.zoom(3),
            (scale) => scale.reset(),
        ),
    ).toEqual([
        [1, 0, 0],
        [3, 0, 0],
        [4, 0, 0],
        [1, 0, 0],
    ]);
});

test("keep the point under a zoom", () => {
    // zoom at a point, then move as far as the edges let it
    expect(
        steps(
            (scale) => scale.zoom(4, { x: 250, y: 150 }),
            (scale) => scale.move(1000, 1000),
            (scale) => scale.move(-1000, -1000),
        ),
    ).toEqual([
        [4, -150, 0],
        [4, 200, 50],
        [4, -200, -50],
    ]);
});

test("zoom and move in one step", () => {
    const places = createRoot((dispose) => {
        const scale = new Scale();
        scale.attach(() => ({
            picture: { width: 200, height: 100 },
            box: new DOMRect(0, 0, 400, 300),
        }));
        scale.zoom(4);
        scale.move(30, 0);
        flush();
        const place = scale.place();
        dispose();

        return place;
    });

    expect(places).toEqual({ scale: 4, x: 30, y: 0 });
});
