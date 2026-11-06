import { present } from "@destack/schema";
import * as style from "@destack/style";
import { createSignal } from "@destack/view";

import {
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

/** The lines between the stars, in order: the globe round, then the ring across its front. */
const lines: readonly (readonly [Star, Star])[] = [
    ...globeStars.map(
        (star, index) =>
            [
                star,
                present(globeStars[(index + 1) % globeStars.length], "the globe's next star"),
            ] as const,
    ),
    ...ringStars
        .slice(1)
        .map((star, index) => [present(ringStars[index], "the ring's last star"), star] as const),
];

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

/** Draw a star chart: scattered stars, or the same stars joined into the mark's figure. */
export function Constellation(properties: { isOpen: boolean }) {
    // follow the switch: the lines draw one after another, then the boundary
    const [host, setHost] = createSignal<HTMLElement>();
    const joined = createChange(() => properties.isOpen, host, CHANGE);
    const drawnOf = (index: number) =>
        stageOf(joined(), (index / lines.length) * 0.75, ((index + 1.5) / lines.length) * 0.75);
    const framed = () => stageOf(joined(), 0.7, 1);

    return (
        <DrawingFrame box={CANVAS} reference={setHost}>
            <Sketch box={CANVAS}>
                <g {...style.attrs(styles.ink)}>
                    {/* draw the chart's coordinate grid: two arcs of right ascension and two of declination */}
                    <g {...style.attrs(styles.dotted)}>
                        <path d="M-150 -140Q-170 0 -150 140M110 -140Q132 0 110 140" />
                        <path d="M-260 -60Q0 -86 260 -60M-260 70Q0 46 260 70" />
                    </g>

                    {/* draw the boundary, and each line in its turn */}
                    <path d={boundary} opacity={framed()} {...style.attrs(styles.dashedBoundary)} />
                    {lines.map(([start, end], index) => (
                        <line
                            x1={start.x}
                            y1={start.y}
                            x2={end.x}
                            y2={end.y}
                            pathLength={1}
                            stroke-dasharray="1 1"
                            stroke-dashoffset={1 - drawnOf(index)}
                        />
                    ))}

                    {/* scatter the faint stars, then set the figure's by brightness, the brightest with a glint */}
                    {field.map((star) => (
                        <circle
                            cx={star.x}
                            cy={star.y}
                            r={star.radius}
                            {...style.attrs(styles.solid)}
                        />
                    ))}
                    {stars.map((star) => (
                        <g transform={`translate(${star.x} ${star.y})`}>
                            <circle
                                r={0.9 + star.brightness * 0.8}
                                {...style.attrs(styles.solid)}
                            />
                            {star.brightness >= 3 ? (
                                <path
                                    d={`M${-4 - star.brightness * 1.5} 0H${4 + star.brightness * 1.5}M0 ${-4 - star.brightness * 1.5}V${4 + star.brightness * 1.5}`}
                                    stroke-width={0.6}
                                    transform={`rotate(${TILT})`}
                                />
                            ) : undefined}
                        </g>
                    ))}
                </g>
            </Sketch>
        </DrawingFrame>
    );
}
