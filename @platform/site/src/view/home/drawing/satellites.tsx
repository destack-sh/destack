import { present } from "@destack/schema";
import * as style from "@destack/style";
import { createSignal } from "@destack/view";

import {
    between,
    CANVAS,
    CHANGE,
    createChange,
    createClock,
    DrawingFrame,
    RingedPlanet,
    onOrbit,
    type Point,
    Sketch,
    stageOf,
    styles,
    TILT,
} from "./drawing";

/** The planet's radius, in CSS pixels. */
const RADIUS = 36;

/** The shared orbit's reach across and down, in CSS pixels. */
const ORBIT = { across: 182, down: 62 };

/** Each satellite's own small orbit while it flies alone, in CSS pixels. */
const OWN_ORBIT = { across: 22, down: 8 };

/** The seconds a satellite takes to go once around the shared orbit. */
const YEAR = 48;

/** The seconds a satellite takes to go once around its own small orbit. */
const OWN_YEAR = 9;

/** The ground line the dishes stand on, in CSS pixels below the centre. */
const GROUND = 96;

/** The length of one wave along a tether or link, in CSS pixels. */
const WAVELENGTH = 9;

/** How far a wave swings to each side of its line, in CSS pixels. */
const AMPLITUDE = 2.5;

/** The waves per second that run along every line. */
const PULSE = 1.4;

/** A signal's shape: each dish speaks its own, and the planet one for all. */
type Wave = "square" | "saw" | "pulse" | "sine";

/** The signal each lone satellite and its dish speak, in turn. */
const OWN_WAVES: readonly Wave[] = ["square", "saw", "pulse"];

/** Return a signal's swing at a share along one wave, from -1 to 1. */
function swingOf(wave: Wave, share: number): number {
    switch (wave) {
        // jump between high and low
        case "square":
            return share < 0.5 ? 1 : -1;
        // ramp up, then drop
        case "saw":
            return 2 * share - 1;
        // spike once, then rest
        case "pulse":
            return share < 0.15 ? Math.sin((share / 0.15) * Math.PI) * 1.4 : 0;
        // swing round
        case "sine":
            return Math.sin(2 * Math.PI * share);
    }
}

/** Return a line from one point to another carrying a signal, its waves running from the start. */
function signalOf(from: Point, to: Point, wave: Wave, seconds: number, amplitude: number): string {
    // draw nothing once the line is reeled in to a point
    const length = Math.hypot(to.x - from.x, to.y - from.y);
    if (length < 1) {
        return "";
    }

    // step along the line, swinging across it
    const along = { x: (to.x - from.x) / length, y: (to.y - from.y) / length };
    const steps = Math.max(2, Math.ceil(length / 1.5));
    const points = Array.from({ length: steps + 1 }, (_, step) => {
        // swing across the line by the signal's shape at this distance
        const distance = (length * step) / steps;
        const share = (((distance / WAVELENGTH - seconds * PULSE) % 1) + 1) % 1;
        const swing = swingOf(wave, share) * amplitude;
        const x = from.x + along.x * distance - along.y * swing;
        const y = from.y + along.y * distance + along.x * swing;

        return `${step === 0 ? "M" : "L"}${x.toFixed(1)} ${y.toFixed(1)}`;
    });

    return points.join("");
}

/** A satellite: the point it circles alone, its dish, and its start on the orbit. */
type Satellite = { free: Point; dish: Point; angle: number };

/** The places the satellites fly alone and stand their dishes, from the left. */
const stations: readonly { free: Point; dish: Point }[] = [
    { free: { x: -168, y: -62 }, dish: { x: -168, y: GROUND } },
    { free: { x: -36, y: -92 }, dish: { x: -36, y: GROUND } },
    { free: { x: 140, y: -70 }, dish: { x: 140, y: GROUND } },
];

/** Draw a satellite: a body, two panels of cells, and an antenna, square to the axis. */
function SatelliteShape() {
    return (
        <g transform={`rotate(${TILT})`}>
            <rect x={-5} y={-4} width={10} height={8} {...style.attrs(styles.cover)} />
            <rect x={-5} y={-4} width={10} height={8} />
            <path d="M-1 4L5 -2M2 4L5 1" />
            {[-1, 1].map((side) => (
                <g transform={`translate(${side * 15} 0)`}>
                    <rect x={-8} y={-3.5} width={16} height={7} {...style.attrs(styles.cover)} />
                    <rect x={-8} y={-3.5} width={16} height={7} />
                    <path d="M-4 -3.5V3.5M0 -3.5V3.5M4 -3.5V3.5" />
                    <path d="M-8 0.5L-5 3.5M-4 0.5L-1 3.5M0 0.5L3 3.5M4 0.5L7 3.5" />
                </g>
            ))}
            <path d="M0 4V9M-2 9H2" />
        </g>
    );
}

/** Draw a dish on the ground, its bowl turned up, with the waves it listens for. */
function Dish() {
    return (
        <path d="M-11 -6A11 7 0 0 0 11 -6M0 1V8M-7 8H7M0 -2L4 -10M-5 -14A8 6 0 0 1 9 -16M-8 -19A12 9 0 0 1 12 -22" />
    );
}

/** Draw satellites each tied to a dish with its own signal, or all round our planet on one. */
export function Satellites(properties: { isOpen: boolean; count: number }) {
    // fly one satellite for each of the figure's connectors, spaced evenly round the shared orbit
    const satellites = (): readonly Satellite[] =>
        Array.from({ length: properties.count }, (_, index) => {
            const station = stations[index % stations.length] ?? {
                free: { x: 0, y: 0 },
                dish: { x: 0, y: GROUND },
            };

            return { ...station, angle: (index * 2 * Math.PI) / properties.count };
        });

    // follow the switch: the dishes and their signals let go, the satellites move onto the shared
    //  orbit, and the links to the planet draw, all carrying one signal
    const [host, setHost] = createSignal<HTMLElement>();
    const joined = createChange(() => properties.isOpen, host, CHANGE);
    const seconds = createClock(host);
    const retract = () => stageOf(joined(), 0, 0.35);
    const move = () => stageOf(joined(), 0.25, 0.8);
    const link = () => stageOf(joined(), 0.7, 1);

    // place a satellite: around its own point alone, around the planet once joined, and whether the planet hides it
    const placeOf = (satellite: Satellite, index: number) => {
        // circle the satellite's own point, and go round the shared orbit
        const own = (2 * Math.PI * seconds()) / OWN_YEAR + index * 2;
        const alone = {
            x: satellite.free.x + OWN_ORBIT.across * Math.cos(own),
            y: satellite.free.y + OWN_ORBIT.down * Math.sin(own),
        };
        const angle = satellite.angle + (2 * Math.PI * seconds()) / YEAR;
        const shared = onOrbit(ORBIT.across, ORBIT.down, angle);
        const distance = Math.hypot(shared.x, shared.y);

        return {
            at: between(alone, shared, move()),
            shared,
            surface: { x: (shared.x * RADIUS) / distance, y: (shared.y * RADIUS) / distance },
            isBehind: Math.sin(angle) < 0 && move() > 0.5,
        };
    };

    return (
        <DrawingFrame box={CANVAS} reference={setHost}>
            <Sketch box={CANVAS}>
                <g {...style.attrs(styles.ink)}>
                    {/* draw the ground, each satellite's own orbit and dish, and its signal, reeled in as it is let go */}
                    <g>
                        <path
                            d={`M-226 ${GROUND + 8}H226`}
                            pathLength={1}
                            stroke-dasharray="1 1"
                            stroke-dashoffset={retract()}
                        />
                        {satellites().map((satellite, index) => {
                            // reel the signal in from the dish to the satellite's foot
                            const foot = () => {
                                const at = placeOf(satellite, index).at;

                                return { x: at.x, y: at.y + 10 };
                            };
                            const dish = { x: satellite.dish.x, y: satellite.dish.y - 14 };
                            const wave = present(
                                OWN_WAVES[index % OWN_WAVES.length],
                                "the satellite's own signal",
                            );

                            return (
                                <g>
                                    <path
                                        d={signalOf(
                                            between(dish, foot(), retract()),
                                            foot(),
                                            wave,
                                            seconds(),
                                            AMPLITUDE * (1 - retract()),
                                        )}
                                    />
                                    <ellipse
                                        cx={satellite.free.x}
                                        cy={satellite.free.y}
                                        rx={OWN_ORBIT.across * (1 - move())}
                                        ry={OWN_ORBIT.down * (1 - move())}
                                        {...style.attrs(styles.dotted)}
                                    />
                                    <g
                                        transform={`translate(${satellite.dish.x} ${satellite.dish.y + 40 * retract()})`}
                                    >
                                        <Dish />
                                    </g>
                                </g>
                            );
                        })}
                    </g>

                    {/* draw the shared orbit, fading in as it is joined */}
                    <ellipse
                        rx={ORBIT.across}
                        ry={ORBIT.down}
                        transform={`rotate(${TILT})`}
                        opacity={move()}
                        {...style.attrs(styles.dashed)}
                    />

                    {/* set the satellites behind the planet first, so the planet hides them */}
                    {satellites().map((satellite, index) => (
                        <g
                            transform={`translate(${placeOf(satellite, index).at.x} ${placeOf(satellite, index).at.y})`}
                            opacity={placeOf(satellite, index).isBehind ? 1 : 0}
                        >
                            <SatelliteShape />
                        </g>
                    ))}

                    {/* grow our planet from a point as the satellites join it */}
                    <g transform={`scale(${move()})`}>
                        <RingedPlanet id="satellites" radius={RADIUS} spacing={3.5} />
                    </g>

                    {/* link each joined satellite to the planet, and set those in front */}
                    {satellites().map((satellite, index) => {
                        // follow the satellite's place each frame
                        const place = () => placeOf(satellite, index);

                        return (
                            <g>
                                <path
                                    d={
                                        link() === 0
                                            ? ""
                                            : signalOf(
                                                  place().surface,
                                                  place().shared,
                                                  "sine",
                                                  seconds(),
                                                  AMPLITUDE * link(),
                                              )
                                    }
                                    opacity={place().isBehind ? 0 : link()}
                                />
                                <g
                                    transform={`translate(${place().at.x} ${place().at.y})`}
                                    opacity={place().isBehind ? 0 : 1}
                                >
                                    <SatelliteShape />
                                </g>
                            </g>
                        );
                    })}
                </g>
            </Sketch>
        </DrawingFrame>
    );
}
