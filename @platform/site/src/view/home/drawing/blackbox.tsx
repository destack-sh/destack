import { present } from "@destack/schema";
import * as style from "@destack/style";
import { createSignal } from "@destack/view";

import {
    CANVAS,
    CHANGE,
    createChange,
    DrawingFrame,
    type Point,
    RingedPlanet,
    Sketch,
    stageOf,
    styles,
} from "./drawing";

/** The box's half width along each face, in CSS pixels. */
const BOX = 52;

/** The globe's radius inside the box, in CSS pixels: room for its ring within the box's outline. */
const RADIUS = 44;

/** The angles of the box's top front edges, in degrees clockwise from the right. */
const EDGES = { left: -150, right: -30 } as const;

/** The hatching of the box's faces, from the lit top to the dark side: line spacing and angle. */
const HATCHING = [
    { spacing: 7, angle: 0 },
    { spacing: 4.5, angle: 60 },
    { spacing: 2.6, angle: -60 },
] as const;

/** Return where a corner of the box lands in an isometric view, from across, deep and up. */
function cornerOf(across: number, deep: number, up: number): Point {
    return {
        x: (across - deep) * Math.cos(Math.PI / 6),
        y: (across + deep) * Math.sin(Math.PI / 6) - up,
    };
}

/** Return a closed outline through some corners. */
function outlineOf(corners: readonly Point[]): string {
    return `M${corners.map((point) => `${point.x.toFixed(1)} ${point.y.toFixed(1)}`).join("L")}Z`;
}

/** Return a point on the globe's outline at an angle in degrees, clockwise from the right. */
function rimOf(radius: number, degrees: number): Point {
    const angle = (degrees * Math.PI) / 180;

    return { x: radius * Math.cos(angle), y: radius * Math.sin(angle) };
}

/** Return the outline of a slice of the globe, from its centre between two angles in degrees. */
function sliceOf(radius: number, from: number, to: number): string {
    const start = rimOf(radius, from);
    const end = rimOf(radius, to);

    return `M0 0L${start.x.toFixed(1)} ${start.y.toFixed(1)}A${radius} ${radius} 0 0 1 ${end.x.toFixed(1)} ${end.y.toFixed(1)}Z`;
}

/** The box's top corners. */
const top = {
    back: cornerOf(-BOX, -BOX, BOX),
    right: cornerOf(BOX, -BOX, BOX),
    front: cornerOf(BOX, BOX, BOX),
    left: cornerOf(-BOX, BOX, BOX),
};

/** The box's bottom corners: the three that show, and the back one behind the box. */
const bottom = {
    back: cornerOf(-BOX, -BOX, -BOX),
    right: cornerOf(BOX, -BOX, -BOX),
    front: cornerOf(BOX, BOX, -BOX),
    left: cornerOf(-BOX, BOX, -BOX),
};

/** The box's three visible faces, top, left and right, each hatched darker toward the shadow. */
const faces: readonly { outline: string; spacing: number; angle: number }[] = [
    outlineOf([top.back, top.right, top.front, top.left]),
    outlineOf([top.left, top.front, bottom.front, bottom.left]),
    outlineOf([top.front, top.right, bottom.right, bottom.front]),
].map((outline, index) => ({ outline, ...present(HATCHING[index], "the face's hatching") }));

/** The box's three edges hidden behind it, all meeting at the back bottom corner. */
const hidden: readonly (readonly [Point, Point])[] = [
    [bottom.back, bottom.right],
    [bottom.back, bottom.left],
    [bottom.back, top.back],
];

/** Draw a sealed black box, or the box turned to glass, our planet inside cut open to its core. */
export function BlackBox(properties: { isOpen: boolean; layers: number }) {
    // follow the switch: the box's hatching is erased line by line, its glass clears to show its
    //  hidden edges and the planet, and the planet opens down to its core under the top face
    const [host, setHost] = createSignal<HTMLElement>();
    const opened = createChange(() => properties.isOpen, host, CHANGE);
    const clear = () => stageOf(opened(), 0.3, 0.5);
    const cut = () => stageOf(opened(), 0.5, 0.95);

    // widen the cut from straight up to the box's top front edges, and space the layers inside it
    const middle = (EDGES.left + EDGES.right) / 2;
    const left = () => middle + (EDGES.left - middle) * cut();
    const right = () => middle + (EDGES.right - middle) * cut();
    const slice = () => sliceOf(RADIUS, left(), right());
    const depths = Array.from(
        { length: properties.layers - 1 },
        (_, index) => RADIUS * (1 - (index + 1) / properties.layers),
    );
    const edges = () => [rimOf(RADIUS, left()), rimOf(RADIUS, right())];

    return (
        <DrawingFrame box={CANVAS} reference={setHost}>
            <Sketch box={CANVAS}>
                <defs>
                    {faces.map((face, index) => (
                        <clipPath id={`blackbox-face-${index}`}>
                            <path d={face.outline} />
                        </clipPath>
                    ))}
                    <clipPath id="blackbox-cut">
                        <path d={slice()} />
                    </clipPath>
                </defs>

                <g {...style.attrs(styles.ink)}>
                    {/* draw the box's hidden edges, dashed as a drawing shows what lies behind */}
                    <g opacity={clear()} {...style.attrs(styles.dashed)}>
                        {hidden.map(([from, to]) => (
                            <line x1={from.x} y1={from.y} x2={to.x} y2={to.y} />
                        ))}
                    </g>

                    {/* draw our planet inside the box, cut to its core under the box's top face */}
                    <RingedPlanet id="blackbox" radius={RADIUS} spacing={4} />
                    {cut() > 0 ? (
                        <g>
                            <path d={slice()} {...style.attrs(styles.cover)} />
                            <g clip-path="url(#blackbox-cut)">
                                {depths.map((depth, index) => (
                                    <circle
                                        r={depth}
                                        {...style.attrs(
                                            index % 2 === 0 ? styles.dashed : styles.dotted,
                                        )}
                                    />
                                ))}
                            </g>
                            {edges().map((point) => (
                                <line x1={0} y1={0} x2={point.x} y2={point.y} />
                            ))}
                            <circle r={3} {...style.attrs(styles.solid)} />
                        </g>
                    ) : undefined}

                    {/* draw the box's faces over it: each sealed, then its hatching erased line by line */}
                    {faces.map((face, index) => {
                        // stagger the erasing across the face's lines, one face after another
                        const rows = Array.from(
                            { length: Math.ceil(400 / face.spacing) },
                            (_, row) => -200 + row * face.spacing,
                        );
                        const erasedOf = (row: number) => {
                            const start = 0.04 * index + (row / rows.length) * 0.28;

                            return stageOf(opened(), start, start + 0.1);
                        };

                        return (
                            <g>
                                <path
                                    d={face.outline}
                                    opacity={1 - clear()}
                                    {...style.attrs(styles.cover)}
                                />
                                <g clip-path={`url(#blackbox-face-${index})`}>
                                    <g transform={`rotate(${face.angle})`}>
                                        {rows.map((y, row) => (
                                            <line
                                                x1={-200}
                                                x2={200}
                                                y1={y}
                                                y2={y}
                                                pathLength={1}
                                                stroke-dasharray="1 1"
                                                stroke-dashoffset={erasedOf(row)}
                                            />
                                        ))}
                                    </g>
                                </g>
                                <path d={face.outline} />
                            </g>
                        );
                    })}
                </g>
            </Sketch>
        </DrawingFrame>
    );
}
