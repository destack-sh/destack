import { present } from "@destack/schema";
import * as style from "@destack/style";
import { color } from "@destack/theme/tokens.stylex";
import { createEffect, createSignal, useEnvironment } from "@destack/view";
import { loopFrames } from "@destack/view/primitives/raf";

import { planet } from "../../layout/mark";
import { CANVAS, CHANGE, DrawingFrame, INK, Sketch, stageOf } from "./drawing";

/** The drawing's scale: CSS pixels per unit of the mark. */
const SCALE = 11;

/** The spacing of the globe's latitude lines, in CSS pixels; the shadow side doubles them. */
const LATITUDE = 4.5;

/** The number of ellipses that draw the ring's band. */
const RING_LINES = 5;

/** The gap between the ring and the globe in front, in CSS pixels on each side of the band. */
const GAP = 0.9 * SCALE;

/** The spacing of a small world's latitude lines, in CSS pixels. */
const MOON_LATITUDE = 3;

/** The globe's radius in CSS pixels. */
const globe = planet.radius * SCALE;

/** The ring's reach across and down in CSS pixels. */
const reach = { across: planet.ringReach.across * SCALE, down: planet.ringReach.down * SCALE };

/** The ring's band width in CSS pixels. */
const band = planet.ringBand * SCALE;

/** The shadow circle in CSS pixels, relative to the globe's centre. */
const shadow = {
    x: (planet.shadow.x - planet.centre.x) * SCALE,
    y: (planet.shadow.y - planet.centre.y) * SCALE,
    radius: planet.shadow.radius * SCALE,
};

/** The latitudes across the globe, every line on the lit side and every half line in shadow. */
const latitudes = Array.from(
    { length: Math.floor((2 * globe) / LATITUDE) + 1 },
    (_, index) => -globe + index * LATITUDE,
);

/** The ring's ellipses, evenly across its band. */
const ringLines = Array.from({ length: RING_LINES }, (_, index) => {
    const offset = -band / 2 + (band * index) / (RING_LINES - 1);

    return {
        across: reach.across + offset,
        down: reach.down + offset * (reach.down / reach.across) * 1.6,
    };
});

/** The drawing's box: the one canvas every section draws on. */
const box = CANVAS;

/** One small world, on its own orbit around where the planet forms while the stack is split. */
type Moon = {
    /** The orbit's reach across, in CSS pixels; its reach down follows the ring's proportions. */
    orbit: number;
    /** The world's radius, in CSS pixels. */
    radius: number;
    /** The seconds the world takes to go once around. */
    year: number;
    /** Where along its orbit the world starts, in radians. */
    phase: number;
};

/** The flattening of every orbit: the ring's own, so the system lies in the ring's plane. */
const FLATTENING = (reach.down / reach.across) * 1.25;

/** The small worlds, from the innermost out, each on an orbit of its own. */
const moons: readonly Moon[] = [
    { orbit: 0.48, radius: 11, year: 68, phase: 0.4 },
    { orbit: 0.7, radius: 16, year: 94, phase: 3.3 },
    { orbit: 0.92, radius: 9, year: 122, phase: 1.6 },
    { orbit: 1.1, radius: 19, year: 158, phase: 4.6 },
    { orbit: 1.27, radius: 12, year: 194, phase: 2.5 },
].map((moon) => ({ ...moon, orbit: moon.orbit * reach.across }));

/** The share of the change by which every world has landed, inner worlds first. */
const LANDED = 0.6;

/** The share of the change at which the orbits start to close into the ring. */
const RINGING = 0.45;

/** The sum of the worlds' areas, in square CSS pixels over pi, which the planet gathers. */
const MASS = moons.reduce((sum, moon) => sum + moon.radius * moon.radius, 0);

/** Return the share of the change over which a world falls in, inner worlds first. */
function fallOf(index: number): { start: number; end: number } {
    const end = 0.2 + ((LANDED - 0.2) * index) / (moons.length - 1);

    return { start: Math.max(0, end - 0.3), end };
}

/** Return where a world is at a time, fallen toward the centre by a share. */
function placeOf(moon: Moon, seconds: number, fallen: number): { x: number; y: number } {
    // go around the orbit, faster as it closes in, like a world falling inward
    const angle = moon.phase + (2 * Math.PI * seconds) / moon.year + fallen * fallen * 4;
    const across = moon.orbit * (1 - fallen);
    const flat = { x: across * Math.cos(angle), y: across * FLATTENING * Math.sin(angle) };

    // tilt the orbit with the ring
    const tilt = (planet.ringTilt * Math.PI) / 180;

    return {
        x: flat.x * Math.cos(tilt) - flat.y * Math.sin(tilt),
        y: flat.x * Math.sin(tilt) + flat.y * Math.cos(tilt),
    };
}

/** Return a world's shadow circle, in the mark's proportions, for a world of a radius. */
function shadowOf(radius: number): { x: number; y: number; radius: number } {
    const ratio = radius / planet.radius;

    return {
        x: (shadow.x / SCALE) * ratio,
        y: (shadow.y / SCALE) * ratio,
        radius: (shadow.radius / SCALE) * ratio,
    };
}

/** Draw the mark's planet while de-stacked, and the small worlds it breaks into while stacked. */
export function PlanetDrawing(properties: { isOpen: boolean; isMoving: boolean }) {
    // hold the drawing's clock, and how far the moons have gathered into the planet
    const environment = useEnvironment();
    const [seconds, setSeconds] = createSignal(0);
    const [gathered, setGathered] = createSignal(properties.isOpen ? 1 : 0);

    // run the clock and ease the gathering while the drawing may move, or settle at the switch's state
    createEffect(
        () => ({ isMoving: properties.isMoving, isOpen: properties.isOpen }),
        ({ isMoving, isOpen }) => {
            // settle at once where the drawing holds still
            const target = isOpen ? 1 : 0;
            if (!isMoving) {
                setGathered(target);
                return;
            }

            // advance by the time since the last frame, easing the gathering toward the switch
            let last: number | undefined;

            return loopFrames((now) => {
                // count the seconds since the last frame, then step the clock and the gathering
                const elapsed = last === undefined ? 0 : Math.min(0.1, (now - last) / 1000);
                last = now;
                setSeconds((current) => current + elapsed);
                const step = elapsed / CHANGE;
                setGathered((current) =>
                    current < target
                        ? Math.min(target, current + step)
                        : Math.max(target, current - step),
                );
            }, environment.display);
        },
    );

    // stage the change: each world falls in and adds its mass, then the orbits close into the ring
    const fallen = (index: number) => {
        const fall = fallOf(index);

        return stageOf(gathered(), fall.start, fall.end);
    };
    const mass = () =>
        moons.reduce((sum, moon, index) => {
            const fall = fallOf(index);

            return sum + moon.radius * moon.radius * stageOf(gathered(), fall.end - 0.06, fall.end);
        }, 0);
    const size = () => Math.sqrt(mass() / MASS);
    const ringed = () => stageOf(gathered(), RINGING, 1);
    const banded = () => stageOf(gathered(), 0.85, 1);
    const rotation = `rotate(${planet.ringTilt})`;
    const front = `M${-box.width} 0H${box.width}V${box.width}H${-box.width}Z`;

    // close each world's orbit into one line of the ring, dashed until it is the ring
    const ellipses = moons.map((moon, index) => {
        const line = present(ringLines[index], "the ring's line for the world");

        return {
            across: () => moon.orbit + (line.across - moon.orbit) * ringed(),
            down: () => moon.orbit * FLATTENING + (line.down - moon.orbit * FLATTENING) * ringed(),
            dash: () => `${3 + 60 * ringed()} ${3 * (1 - ringed())}`,
        };
    });

    return (
        <DrawingFrame box={box}>
            <Sketch box={box}>
                <defs>
                    <clipPath id="planet-globe">
                        <circle r={globe * size()} />
                    </clipPath>
                    <clipPath id="planet-shadow">
                        <circle
                            cx={shadow.x * size()}
                            cy={shadow.y * size()}
                            r={shadow.radius * size()}
                        />
                    </clipPath>
                    <clipPath id="planet-front">
                        <path d={front} transform={rotation} />
                    </clipPath>
                </defs>

                <g opacity={INK}>
                    {/* draw each world's orbit behind the planet, closing into a line of its ring */}
                    <g transform={rotation}>
                        {ellipses.map((ellipse) => (
                            <ellipse
                                rx={ellipse.across()}
                                ry={ellipse.down()}
                                stroke-dasharray={ellipse.dash()}
                                {...style.attrs(styles.orbit)}
                            />
                        ))}
                    </g>

                    {/* grow the globe by each world that lands, its lines twice as dense in shadow */}
                    <circle r={globe * size()} {...style.attrs(styles.cover)} />
                    <g clip-path="url(#planet-globe)">
                        <g transform={rotation}>
                            {latitudes.map((y) => (
                                <line x1={-globe} x2={globe} y1={y} y2={y} />
                            ))}
                        </g>
                        <g clip-path="url(#planet-shadow)">
                            <g transform={rotation}>
                                {latitudes.map((y) => (
                                    <line
                                        x1={-globe}
                                        x2={globe}
                                        y1={y + LATITUDE / 2}
                                        y2={y + LATITUDE / 2}
                                    />
                                ))}
                            </g>
                        </g>
                    </g>
                    <circle r={globe * size()} />

                    {/* ripple out from the globe where each world lands */}
                    {moons.map((_, index) => {
                        const fall = fallOf(index);
                        const ripple = () => stageOf(gathered(), fall.end - 0.04, fall.end + 0.16);

                        return (
                            <circle
                                r={globe * size() + 4 + 30 * ripple()}
                                opacity={ripple() > 0 && ripple() < 1 ? 1 - ripple() : 0}
                                {...style.attrs(styles.ripple)}
                            />
                        );
                    })}

                    {/* cut the ring's front across the globe, with a gap of page around its band */}
                    <g clip-path="url(#planet-front)">
                        <g transform={rotation}>
                            <ellipse
                                rx={reach.across}
                                ry={reach.down}
                                stroke-width={band + GAP * 2}
                                opacity={banded()}
                                {...style.attrs(styles.gap)}
                            />
                            {ellipses.map((ellipse) => (
                                <ellipse
                                    rx={ellipse.across()}
                                    ry={ellipse.down()}
                                    stroke-dasharray={ellipse.dash()}
                                    {...style.attrs(styles.orbit)}
                                />
                            ))}
                        </g>
                    </g>
                </g>
            </Sketch>

            {/* set the small worlds on their own layers, so they glide without redrawing their lines */}
            {moons.map((moon, index) => {
                const place = () => placeOf(moon, seconds(), fallen(index));

                return (
                    <World
                        moon={moon}
                        index={index}
                        x={place().x}
                        y={place().y}
                        scale={1 - 0.4 * fallen(index)}
                        opacity={1 - stageOf(fallen(index), 0.8, 1)}
                    />
                );
            })}
        </DrawingFrame>
    );
}

/** Draw one small world, lined and shaded as the planet is. */
function World(properties: {
    moon: Moon;
    index: number;
    x: number;
    y: number;
    scale: number;
    opacity: number;
}) {
    // measure the world: its shadow and its latitude rows
    const radius = properties.moon.radius;
    const dark = shadowOf(radius);
    const rotation = `rotate(${planet.ringTilt})`;
    const rows = Array.from(
        { length: Math.floor((radius * 2) / MOON_LATITUDE) },
        (_, line) => -radius + (line + 0.5) * MOON_LATITUDE,
    );

    return (
        <div
            style={{
                opacity: String(properties.opacity),
                transform: `translate3d(${properties.x}px, ${properties.y}px, 0) scale(${properties.scale})`,
            }}
            {...style.attrs(styles.world)}
        >
            <svg
                viewBox={`${-radius - 1} ${-radius - 1} ${radius * 2 + 2} ${radius * 2 + 2}`}
                width={radius * 2 + 2}
                height={radius * 2 + 2}
                opacity={INK}
                {...style.attrs(styles.globe)}
            >
                <defs>
                    <clipPath id={`planet-world-${properties.index}`}>
                        <circle r={radius} />
                    </clipPath>
                    <clipPath id={`planet-world-shadow-${properties.index}`}>
                        <circle cx={dark.x} cy={dark.y} r={dark.radius} />
                    </clipPath>
                </defs>
                <circle r={radius} {...style.attrs(styles.cover)} />
                <g clip-path={`url(#planet-world-${properties.index})`}>
                    <g transform={rotation}>
                        {rows.map((y) => (
                            <line x1={-radius} x2={radius} y1={y} y2={y} />
                        ))}
                    </g>
                    <g clip-path={`url(#planet-world-shadow-${properties.index})`}>
                        <g transform={rotation}>
                            {rows.map((y) => (
                                <line
                                    x1={-radius}
                                    x2={radius}
                                    y1={y + MOON_LATITUDE / 2}
                                    y2={y + MOON_LATITUDE / 2}
                                />
                            ))}
                        </g>
                    </g>
                </g>
                <circle r={radius} />
            </svg>
        </div>
    );
}

/** The planet drawing styles. */
const styles = style.create({
    world: {
        left: "50%",
        pointerEvents: "none",
        position: "absolute",
        top: "50%",
        willChange: "transform, opacity",
    },
    globe: {
        color: color.foreground,
        display: "block",
        fill: "none",
        overflow: "visible",
        stroke: "currentColor",
        strokeWidth: 1,
        transform: "translate(-50%, -50%)",
    },
    orbit: {
        strokeWidth: 1.25,
    },
    ripple: {
        strokeDasharray: "1 3",
    },
    cover: {
        fill: color.background,
        stroke: "none",
    },
    gap: {
        stroke: color.background,
    },
});
