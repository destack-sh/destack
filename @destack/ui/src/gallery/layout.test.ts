import { expect, test } from "@destack/test";
import {
    type Box,
    COLLAGE_COLUMNS,
    CollageLayout,
    GridLayout,
    type Layout,
    MasonryLayout,
    RowLayout,
} from "./index.ts";

/** The ratios of photos, taken over and over. */
const RATIOS = [1.5, 4 / 3, 0.75, 1, 16 / 9, 9 / 16, 2.4];

/** The widths of a phone, a column, a page and a wide one. */
const WIDTHS = [320, 390, 600, 900, 1440];

/** The layouts with the most items each places. */
const LAYOUTS: readonly (readonly [string, Layout, number])[] = [
    ["grid", new GridLayout({ minimum: 120 }), Number.POSITIVE_INFINITY],
    ["rows", new RowLayout({ height: 130 }), Number.POSITIVE_INFINITY],
    ["masonry", new MasonryLayout({ minimum: 140 }), Number.POSITIVE_INFINITY],
    ["collage", new CollageLayout({ count: 5 }), 5],
    ["large collage", new CollageLayout({ count: 9 }), 9],
];

/** Read the ratios of a count of photos, beginning at one of them. */
function photos(count: number, from: number): readonly number[] {
    return Array.from(
        { length: count },
        (_photo, i) => RATIOS[(from + i * 3) % RATIOS.length] ?? 1,
    );
}

/** Report whether two boxes overlap, by more than rounding. */
function overlaps(one: Box, other: Box): boolean {
    const edge = 0.01;

    return (
        one.x + one.width > other.x + edge &&
        other.x + other.width > one.x + edge &&
        one.y + one.height > other.y + edge &&
        other.y + other.height > one.y + edge
    );
}

test("place every item once, apart and inside", () => {
    // lay out each count of items at each width, and collect what is wrong
    const wrong: unknown[] = [];
    for (const [name, layout, most] of LAYOUTS) {
        for (const width of WIDTHS) {
            for (let count = 0; count <= 40; count += 1) {
                const boxes = layout.place(photos(count, width % 7), width);
                const isPlaced = boxes.length === Math.min(count, most);
                const isInside = boxes.every(
                    (box) =>
                        box.x >= 0 &&
                        box.y >= 0 &&
                        box.width > 0 &&
                        box.height > 0 &&
                        box.x + box.width <= width + 0.01,
                );
                const isApart = boxes.every((box, i) =>
                    boxes.slice(i + 1).every((other) => !overlaps(box, other)),
                );
                if (!isPlaced || !isInside || !isApart) {
                    wrong.push([name, width, count, { isPlaced, isInside, isApart }]);
                }
            }
        }
    }

    expect(wrong).toEqual([]);
});

test("fill each row near its height", () => {
    // read the rows of many photos at each width
    const rows = WIDTHS.map((width) => {
        const ratios = photos(200, 0);
        const boxes = new RowLayout({ height: 130 }).place(ratios, width);
        const byRow = [...Map.groupBy(boxes, ({ y }) => y).values()];
        const ends = byRow.slice(0, -1).map((row) => {
            const last = row.at(-1);

            return last === undefined ? 0 : last.x + last.width;
        });
        const heights = byRow.slice(0, -1).map(([first]) => first?.height ?? 0);

        return {
            isFilled: ends.every((end) => Math.abs(end - width) < 0.01),
            isUndistorted: boxes.every(
                (box, i) => Math.abs(box.width / box.height - (ratios[i] ?? 0)) < 0.001,
            ),
            lowest: Math.round(Math.min(...heights)),
            highest: Math.round(Math.max(...heights)),
            isLastLow: (byRow.at(-1)?.[0]?.height ?? 0) <= 130,
        };
    });

    expect(rows).toEqual([
        { isFilled: true, isUndistorted: true, lowest: 115, highest: 180, isLastLow: true },
        { isFilled: true, isUndistorted: true, lowest: 112, highest: 163, isLastLow: true },
        { isFilled: true, isUndistorted: true, lowest: 102, highest: 127, isLastLow: true },
        { isFilled: true, isUndistorted: true, lowest: 114, highest: 131, isLastLow: true },
        { isFilled: true, isUndistorted: true, lowest: 118, highest: 134, isLastLow: true },
    ]);
});

test("place items under the shortest column", () => {
    const boxes = new MasonryLayout({ minimum: 100, gap: 0 }).place([1, 0.5, 2, 1, 1], 300);

    expect(boxes.map(({ x, y, height }) => [x, y, height])).toEqual([
        [0, 0, 100],
        [100, 0, 200],
        [200, 0, 50],
        [200, 50, 100],
        [0, 100, 100],
    ]);
});

test("give a collage's first item half", () => {
    const collage = new CollageLayout({ count: 5, ratio: 1, gap: 0 });

    expect(
        [1, 2, 3, 4, 5, 9].map((count) =>
            collage
                .place(photos(count, 0), 200)
                .map(({ x, y, width, height }) => [x, y, width, height].join(" ")),
        ),
    ).toEqual([
        ["0 0 200 200"],
        ["0 0 100 200", "100 0 100 200"],
        ["0 0 100 200", "100 0 100 100", "100 100 100 100"],
        ["0 0 100 200", "100 0 50 100", "150 0 50 100", "100 100 100 100"],
        ["0 0 100 200", "100 0 50 100", "150 0 50 100", "100 100 50 100", "150 100 50 100"],
        ["0 0 100 200", "100 0 50 100", "150 0 50 100", "100 100 50 100", "150 100 50 100"],
    ]);
});

test("fill a collage's frame", () => {
    // sum the areas of each count's items
    const filled = Array.from({ length: 12 }, (_each, i) => {
        const count = i + 1;
        const areas = Array.from({ length: count }, (_item, at) => CollageLayout.area(at, count));

        return areas.reduce((sum, { width, height }) => sum + width * height, 0);
    });

    expect(filled).toEqual(
        Array.from({ length: 12 }, (_each, i) => COLLAGE_COLUMNS * CollageLayout.rows(i + 1)),
    );
});
