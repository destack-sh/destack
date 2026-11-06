import { present } from "@destack/schema";
import * as style from "@destack/style";
import { createSignal } from "@destack/view";

import { planet } from "../../layout/mark";
import {
    CANVAS,
    CHANGE,
    createChange,
    DrawingFrame,
    type Point,
    Sketch,
    stageOf,
    styles,
    TILT,
} from "./drawing";

/** The box's half width along each face, in CSS pixels. */
const BOX = 52;

/** The globe's radius once it shows through, in CSS pixels. */
const RADIUS = 84;

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

/** The box's top corners. */
const top = {
    back: cornerOf(-BOX, -BOX, BOX),
    right: cornerOf(BOX, -BOX, BOX),
    front: cornerOf(BOX, BOX, BOX),
    left: cornerOf(-BOX, BOX, BOX),
};

/** The box's bottom corners that show: right, front and left. */
const bottom = {
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

/** Draw a sealed black box, or the box opened to show our planet inside, see-through. */
export function BlackBox(properties: { isOpen: boolean; layers: number }) {
    // follow the switch: the box's hatching is erased line by line, its glass clears, its edges retract,
    //  and our planet inside grows out to its full size
    const [host, setHost] = createSignal<HTMLElement>();
    const opened = createChange(() => properties.isOpen, host, CHANGE);
    const clear = () => stageOf(opened(), 0.35, 0.5);
    const edges = () => stageOf(opened(), 0.45, 0.8);
    const grown = () => 0.55 + 0.45 * stageOf(opened(), 0.4, 1);

    // measure the ring in the mark's proportions to the globe, and space the layers inside it
    const ratio = RADIUS / planet.radius;
    const ring = { across: planet.ringReach.across * ratio, down: planet.ringReach.down * ratio };
    const depths = () =>
        Array.from(
            { length: properties.layers - 1 },
            (_, index) => RADIUS * (1 - (index + 1) / properties.layers),
        );
    const front = `M${-CANVAS.width} 0H${CANVAS.width}V${CANVAS.width}H${-CANVAS.width}Z`;

    return (
        <DrawingFrame box={CANVAS} reference={setHost}>
            <Sketch box={CANVAS}>
                <defs>
                    {faces.map((face, index) => (
                        <clipPath id={`blackbox-face-${index}`}>
                            <path d={face.outline} />
                        </clipPath>
                    ))}
                    <clipPath id="blackbox-front">
                        <path d={front} transform={`rotate(${TILT})`} />
                    </clipPath>
                </defs>

                <g {...style.attrs(styles.ink)}>
                    {/* draw our planet inside the box, see-through to its layers, growing out once the box is open */}
                    <g transform={`scale(${grown()})`}>
                        <g transform={`rotate(${TILT})`}>
                            <ellipse rx={ring.across} ry={ring.down} />
                            <ellipse rx={ring.across - 8} ry={ring.down - 4} />
                        </g>
                        <circle r={RADIUS} {...style.attrs(styles.cover)} />
                        <circle r={RADIUS} />
                        {depths().map((depth, index) => (
                            <circle
                                r={depth}
                                {...style.attrs(index % 2 === 0 ? styles.dashed : styles.dotted)}
                            />
                        ))}
                        <circle r={3} {...style.attrs(styles.solid)} />
                        <g clip-path="url(#blackbox-front)" transform={`rotate(${TILT})`}>
                            <ellipse
                                rx={ring.across - 4}
                                ry={ring.down - 2}
                                stroke-width={20}
                                {...style.attrs(styles.gap)}
                            />
                            <ellipse rx={ring.across} ry={ring.down} />
                            <ellipse rx={ring.across - 8} ry={ring.down - 4} />
                        </g>
                    </g>

                    {/* draw the sealed box over it: each face's hatching erased line by line, then its glass and edges */}
                    <g transform="translate(0 -4)">
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
                                    <path
                                        d={face.outline}
                                        pathLength={1}
                                        stroke-dasharray="1 1"
                                        stroke-dashoffset={-edges()}
                                    />
                                </g>
                            );
                        })}
                    </g>
                </g>
            </Sketch>
        </DrawingFrame>
    );
}
