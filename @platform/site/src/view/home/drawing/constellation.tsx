import { present } from "@destack/schema";
import * as style from "@destack/style";
import { createSignal } from "@destack/view";

import {
    between,
    CANVAS,
    CHANGE,
    createChange,
    DrawingFrame,
    onOrbit,
    type Point,
    Sketch,
    stageOf,
    styles,
    TILT,
} from "./drawing";

/** A star of the figure: where it sits, and its brightness from 1 to 4. */
type Star = Point & { brightness: number };

/** The globe's radius in the figure, in CSS pixels. */
const GLOBE = 66;

/** The ring's reach across and down, at the mark's tilt, in CSS pixels. */
const RING = { across: 196, down: 50 };

/** The globe's stars, a loop of eight round its edge. */
const globeStars: readonly Star[] = [
    { brightness: 4, angle: -96 },
    { brightness: 2, angle: -48 },
    { brightness: 3, angle: -4 },
    { brightness: 1, angle: 44 },
    { brightness: 2, angle: 92 },
    { brightness: 1, angle: 136 },
    { brightness: 3, angle: 182 },
    { brightness: 1, angle: 228 },
].map(({ angle, ...star }) => ({
    ...star,
    x: GLOBE * Math.cos((angle * Math.PI) / 180),
    y: GLOBE * Math.sin((angle * Math.PI) / 180),
}));

/** The ring's stars along its ellipse, from the far left, round the front, to the far right. */
const ringStars: readonly Star[] = [
    { brightness: 3, angle: 186 },
    { brightness: 1, angle: 158 },
    { brightness: 2, angle: 122 },
    { brightness: 2, angle: 62 },
    { brightness: 1, angle: 24 },
    { brightness: 3, angle: -6 },
].map(({ angle, ...star }) => ({
    ...star,
    ...onOrbit(RING.across, RING.down, (angle * Math.PI) / 180),
}));

/** Every star of the figure. */
const stars: readonly Star[] = [...globeStars, ...ringStars];

/** The lines between the stars, by index: the globe round, then the ring across its front. */
const lines: readonly (readonly [number, number])[] = [
    ...globeStars.map((_, index) => [index, (index + 1) % globeStars.length] as const),
    ...ringStars
        .slice(1)
        .map((_, index) => [globeStars.length + index, globeStars.length + index + 1] as const),
];

/** The lines joined one at a time before the rest join at once: one app at a time, then all. */
const ONE_BY_ONE = 3;

/** The share of the change each of the first lines takes to draw. */
const STEP = 0.15;

/** The share of the change at which the first star has arrived and the pen sets off. */
const SET_OFF = 0.06;

/** The share of the change at which the pen's lines are done and the rest join at once. */
const AT_ONCE = SET_OFF + ONE_BY_ONE * STEP;

/** Where each of the figure's stars lies scattered while stacked, from a fixed seed. */
const scattered: readonly Point[] = (() => {
    // draw from a fixed seed, so the server and the browser agree
    let seed = 41;
    const random = () => {
        seed = (seed * 16807) % 2147483647;
        return seed / 2147483647;
    };

    return stars.map(() => ({
        x: (random() - 0.5) * (CANVAS.width - 40),
        y: (random() - 0.5) * (CANVAS.height - 60),
    }));
})();

/** Return the share of the change over which a star of the figure moves into place. */
function arrivalOf(index: number): { start: number; end: number } {
    // bring the first star at once, the pen's next stars as it reaches them, and the rest together
    if (index === 0) {
        return { start: 0, end: SET_OFF };
    } else if (index <= ONE_BY_ONE) {
        return { start: SET_OFF + (index - 1) * STEP, end: SET_OFF + index * STEP };
    } else {
        return { start: AT_ONCE, end: AT_ONCE + 0.18 };
    }
}

/** Return the share of the change over which a line draws: the pen's in turn, the rest together. */
function drawingOf(index: number): { start: number; end: number } {
    return index < ONE_BY_ONE
        ? { start: SET_OFF + index * STEP, end: SET_OFF + (index + 1) * STEP }
        : { start: AT_ONCE + 0.08, end: AT_ONCE + 0.34 };
}

/** The faint stars around the figure, from a fixed seed so server and browser agree. */
const field: readonly (Point & { radius: number })[] = (() => {
    // draw from a fixed seed
    let seed = 7;
    const random = () => {
        seed = (seed * 16807) % 2147483647;
        return seed / 2147483647;
    };

    return Array.from({ length: 46 }, () => ({
        x: (random() - 0.5) * CANVAS.width,
        y: (random() - 0.5) * CANVAS.height,
        radius: random() < 0.85 ? 0.6 : 1.1,
    }));
})();

/** The figure's boundary, as a star atlas dashes round a constellation. */
const boundary =
    "M-232 -20L-120 -20L-120 -100L120 -100L120 -66L236 -66L236 50L96 50L96 92L-232 92Z";

/** Draw a star chart: scattered stars, or the stars joined one at a time, then all at once. */
export function Constellation(properties: { isOpen: boolean }) {
    // follow the switch: a pen joins the first stars one at a time, then the rest join at once
    const [host, setHost] = createSignal<HTMLElement>();
    const joined = createChange(() => properties.isOpen, host, CHANGE);
    const arrivedOf = (index: number) => {
        const arrival = arrivalOf(index);

        return stageOf(joined(), arrival.start, arrival.end);
    };
    const drawnOf = (index: number) => {
        const drawing = drawingOf(index);

        return stageOf(joined(), drawing.start, drawing.end);
    };
    const placeOf = (index: number) =>
        between(
            present(scattered[index], "the star's scattered place"),
            present(stars[index], "the star"),
            arrivedOf(index),
        );
    const framed = () => stageOf(joined(), 0.8, 1);

    // follow the pen along the line it draws, while the first lines draw
    const pen = () => {
        // find the first of the pen's lines still drawing
        const index = lines.findIndex((_, line) => line < ONE_BY_ONE && drawnOf(line) < 1);
        const line = lines[index];
        if (line === undefined || joined() <= SET_OFF) {
            return undefined;
        }

        return between(placeOf(line[0]), placeOf(line[1]), drawnOf(index));
    };

    return (
        <DrawingFrame box={CANVAS} reference={setHost}>
            <Sketch box={CANVAS}>
                <g {...style.attrs(styles.ink)}>
                    {/* draw the chart's coordinate grid: two arcs of right ascension and two of declination */}
                    <g {...style.attrs(styles.dotted)}>
                        <path d="M-150 -140Q-170 0 -150 140M110 -140Q132 0 110 140" />
                        <path d="M-260 -60Q0 -86 260 -60M-260 70Q0 46 260 70" />
                    </g>

                    {/* draw the boundary, and each line between its stars where they are */}
                    <path d={boundary} opacity={framed()} {...style.attrs(styles.dashedBoundary)} />
                    {lines.map(([start, end], index) => (
                        <line
                            x1={placeOf(start).x}
                            y1={placeOf(start).y}
                            x2={placeOf(end).x}
                            y2={placeOf(end).y}
                            pathLength={1}
                            stroke-dasharray="1 1"
                            stroke-dashoffset={1 - drawnOf(index)}
                        />
                    ))}

                    {/* scatter the faint stars */}
                    {field.map((star) => (
                        <circle
                            cx={star.x}
                            cy={star.y}
                            r={star.radius}
                            {...style.attrs(styles.solid)}
                        />
                    ))}

                    {/* set the figure's stars by brightness, the brightest with a glint, wherever they are */}
                    {stars.map((star, index) => {
                        const glint = 4 + star.brightness * 1.5;

                        return (
                            <g transform={`translate(${placeOf(index).x} ${placeOf(index).y})`}>
                                <circle
                                    r={0.9 + star.brightness * 0.8}
                                    {...style.attrs(styles.solid)}
                                />
                                {star.brightness >= 3 ? (
                                    <path
                                        d={`M${-glint} 0H${glint}M0 ${-glint}V${glint}`}
                                        stroke-width={0.6}
                                        transform={`rotate(${TILT})`}
                                    />
                                ) : undefined}
                            </g>
                        );
                    })}

                    {/* draw the pen at the tip of the line it draws */}
                    {pen() === undefined ? undefined : (
                        <g transform={`translate(${pen()?.x ?? 0} ${pen()?.y ?? 0})`}>
                            <circle r={2.4} {...style.attrs(styles.solid)} />
                            <circle r={6} {...style.attrs(styles.dotted)} />
                        </g>
                    )}
                </g>
            </Sketch>
        </DrawingFrame>
    );
}
