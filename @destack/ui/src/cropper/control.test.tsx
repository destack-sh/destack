import { expect, test } from "@destack/test";
import { flush } from "@destack/view";
import { renderHook } from "@destack/view/test";
import { type CropArea, CropperControl, type CropperOptions, type CropPosition } from "./index.ts";

/** Crop an image of a size in a view of a size. */
function cropper(options: CropperOptions, width = 800, height = 600): CropperControl {
    const control = renderHook(() => new CropperControl(options)).result;
    const image = document.createElement("img");
    Object.defineProperties(image, {
        naturalWidth: { value: width },
        naturalHeight: { value: height },
    });
    control.load(image);
    control.resize({ width: 400, height: 400 });
    flush();

    return control;
}

/** Read a part as its place and size, rounded. */
function read({ x, y, width, height }: CropArea): readonly number[] {
    return [x, y, width, height].map(Math.round);
}

/** Read the place of the frame and of the image, and the pixels of the screen one takes. */
function placed(control: CropperControl): readonly number[] {
    const { x, y, width, height } = control.frame();
    const origin = control.origin();

    return [x, y, width, height, origin.x, origin.y, control.scale()].map(
        (value) => Math.round(value * 100) / 100,
    );
}

/** Do something to a cropper, and read the part it has then. */
function after(control: CropperControl, change: () => void): readonly number[] {
    change();
    flush();

    return read(control.area());
}

test("take the most of an image, by a ratio and without", () => {
    expect([
        read(cropper({}).area()),
        read(cropper({ aspectRatio: 1 }).area()),
        read(cropper({ aspectRatio: 4 }).area()),
        read(
            cropper({
                aspectRatio: 1,
                defaultValue: { x: -50, y: 500, width: 100, height: 400 },
            }).area(),
        ),
    ]).toEqual([
        [0, 0, 800, 600],
        [100, 0, 600, 600],
        [0, 200, 800, 200],
        [0, 200, 400, 400],
    ]);
});

test("move and zoom a part within the image, a point kept in its place", () => {
    const control = cropper({ aspectRatio: 1 });

    expect([
        after(control, () => control.zoomTo(2)),
        after(control, () => control.zoomTo(3, { x: 250, y: 150 })),
        after(control, () => control.moveBy(-500, 90)),
        after(control, () => control.moveBy(5000, 5000)),
        after(control, () => control.zoomTo(99)),
        after(control, () => control.zoomTo(0)),
    ]).toEqual([
        [250, 150, 300, 300],
        [250, 150, 200, 200],
        [0, 240, 200, 200],
        [600, 400, 200, 200],
        [600, 400, 200, 200],
        [200, 0, 600, 600],
    ]);
});

test.for([
    [
        "top",
        [
            [300, 240, 200, 160],
            [320, 240, 160, 160],
            [300, 384, 200, 16],
            [200, 0, 400, 400],
        ],
    ],
    [
        "top-right",
        [
            [300, 240, 260, 160],
            [300, 140, 260, 260],
            [300, 384, 16, 16],
            [300, 0, 400, 400],
        ],
    ],
    [
        "right",
        [
            [300, 200, 260, 200],
            [300, 170, 260, 260],
            [300, 200, 16, 200],
            [300, 50, 500, 500],
        ],
    ],
    [
        "bottom-right",
        [
            [300, 200, 260, 240],
            [300, 200, 260, 260],
            [300, 200, 16, 16],
            [300, 200, 400, 400],
        ],
    ],
    [
        "bottom",
        [
            [300, 200, 200, 240],
            [280, 200, 240, 240],
            [300, 200, 200, 16],
            [200, 200, 400, 400],
        ],
    ],
    [
        "bottom-left",
        [
            [360, 200, 140, 240],
            [260, 200, 240, 240],
            [484, 200, 16, 16],
            [100, 200, 400, 400],
        ],
    ],
    [
        "left",
        [
            [360, 200, 140, 200],
            [360, 230, 140, 140],
            [484, 200, 16, 200],
            [0, 50, 500, 500],
        ],
    ],
    [
        "top-left",
        [
            [360, 240, 140, 160],
            [340, 240, 160, 160],
            [484, 384, 16, 16],
            [100, 0, 400, 400],
        ],
    ],
] as const)(
    "move the %s of a part, the opposite kept",
    ([side, expected]: readonly [CropPosition, readonly (readonly number[])[]]) => {
        const defaultValue = { x: 300, y: 200, width: 200, height: 200 };
        const free = cropper({ defaultValue });
        const ratio = cropper({ defaultValue, aspectRatio: 1 });

        expect([
            after(free, () => free.resizeBy(side, 60, 40)),
            after(ratio, () => ratio.resizeBy(side, 60, 40)),
            after(free, () =>
                free.resizeBy(
                    side,
                    side.includes("left") ? 9999 : -9999,
                    side.includes("top") ? 9999 : -9999,
                ),
            ),
            after(ratio, () =>
                ratio.resizeBy(
                    side,
                    side.includes("left") ? -9999 : 9999,
                    side.includes("top") ? -9999 : 9999,
                ),
            ),
        ]).toEqual(expected);
    },
);

test("turn and mirror the same part, and take the most again", () => {
    const control = cropper({ defaultValue: { x: 10, y: 20, width: 300, height: 200 } });

    expect([
        after(control, () => control.rotate()),
        after(control, () => control.flip()),
        after(control, () => control.flip()),
        after(control, () => control.rotate(-1)),
        after(control, () => control.reset()),
    ]).toEqual([
        [380, 10, 200, 300],
        [20, 10, 200, 300],
        [380, 10, 200, 300],
        [10, 20, 300, 200],
        [0, 0, 800, 600],
    ]);
});

test("turn a mirrored image the way it looks, and mirror it along both axes", () => {
    const control = cropper({ defaultValue: { x: 10, y: 20, width: 300, height: 200 } });
    const state = (): readonly unknown[] => [
        control.rotation(),
        control.mirror().horizontal,
        control.mirror().vertical,
        ...read(control.area()),
    ];
    const states: (readonly unknown[])[] = [];
    for (const change of [
        () => control.flip(),
        () => control.rotate(),
        () => control.flip("vertical"),
        () => control.rotate(),
    ]) {
        change();
        flush();
        states.push(state());
    }

    expect(states).toEqual([
        [0, true, false, 490, 20, 300, 200],
        [270, true, false, 380, 490, 200, 300],
        [270, true, true, 380, 10, 200, 300],
        [0, true, true, 490, 380, 300, 200],
    ]);
});

test("follow how the owner turns and mirrors it, and keep the least size", () => {
    const given: unknown[] = [];
    const control = cropper({
        rotation: 90,
        flip: { horizontal: true, vertical: false },
        minWidth: 300,
        minHeight: 100,
        onRotationChange: (rotation) => void given.push(rotation),
        onFlipChange: (flip) => void given.push(flip),
        onZoomChange: (zoom) => void given.push(zoom),
    });
    control.rotate();
    control.flip();
    control.zoomTo(9);
    flush();

    expect([control.turned(), control.maxZoom(), given]).toEqual([
        { width: 600, height: 800 },
        2,
        [0, { horizontal: false, vertical: false }, 2],
    ]);
});

test("tell the part a person settled on as it is then", () => {
    const committed: (readonly number[])[] = [];
    const control = cropper({
        defaultValue: { x: 10, y: 20, width: 300, height: 200 },
        onValueCommitted: (value) => void committed.push(read(value)),
    });
    control.moveBy(5, 5);
    control.commit();
    control.rotate();
    control.flip();
    control.reset();

    expect(committed).toEqual([
        [15, 25, 300, 200],
        [375, 15, 200, 300],
        [25, 15, 200, 300],
        [0, 0, 800, 600],
    ]);
});

test("place the frame over the image, or the image below a frame that stays", () => {
    const moving = cropper({ defaultValue: { x: 400, y: 300, width: 400, height: 300 } });
    const still = cropper({ fixedCropArea: true, aspectRatio: 1 });
    expect([placed(moving), placed(still)]).toEqual([
        [200, 200, 176, 132, 24, 68, 0.44],
        [24, 24, 352, 352, -34.67, 24, 0.59],
    ]);
});

test("leave a disabled cropper as it is", () => {
    const given: CropArea[] = [];
    const control = cropper({ disabled: true, onValueChange: (value) => void given.push(value) });

    expect([after(control, () => control.moveBy(5, 5)), given]).toEqual([[0, 0, 800, 600], []]);
});
