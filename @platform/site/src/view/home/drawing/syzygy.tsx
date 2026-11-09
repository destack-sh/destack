import * as style from "@destack/style";
import { createSignal } from "@destack/view";

import {
    between,
    CANVAS,
    CHANGE,
    createChange,
    createClock,
    DrawingFrame,
    Globe,
    onOrbit,
    type Point,
    Sketch,
    stageOf,
    styles,
    TILT,
} from "./drawing";

/** The flattening of every orbit. */
const FLATTENING = 0.34;

/** The planets from the sun out: each with a sun of its own while stacked, and its one orbit. */
const planets = [
    { across: 78, radius: 11, sun: { x: -150, y: -38 }, loose: 44, phase: 138, year: 9 },
    { across: 136, radius: 15, sun: { x: 6, y: 52 }, loose: 54, phase: 296, year: 13 },
    { across: 196, radius: 13, sun: { x: 156, y: -34 }, loose: 48, phase: 214, year: 11 },
] as const;

/** The rays of a sun's corona, evenly around it. */
const RAYS = 16;

/** The origin, where the one sun forms. */
const ORIGIN: Point = { x: 0, y: 0 };

/** Draw the apps as planets each round a sun of its own, or round one sun and lined up. */
export function Syzygy(properties: { isOpen: boolean }) {
    // follow the switch: the suns merge, the orbits widen round the one sun, the planets line up
    const [host, setHost] = createSignal<HTMLElement>();
    const aligned = createChange(() => properties.isOpen, host, CHANGE);
    const merge = () => stageOf(aligned(), 0, 0.6);
    const slide = () => stageOf(aligned(), 0.3, 0.85);
    const line = () => stageOf(aligned(), 0.75, 1);
    const reach = 222;

    // turn the planets round their own suns only while stacked, so they line up from where they stand
    const turned = createClock(host, () => aligned() === 0);

    return (
        <DrawingFrame box={CANVAS} reference={setHost}>
            <Sketch box={CANVAS}>
                <g {...style.attrs(styles.ink)}>
                    {/* draw each orbit round its own sun, widening round the one sun as they merge */}
                    {planets.map((planet) => {
                        const centre = () => between(planet.sun, ORIGIN, merge());
                        const across = () =>
                            planet.loose + (planet.across - planet.loose) * merge();

                        return (
                            <ellipse
                                transform={`translate(${centre().x} ${centre().y}) rotate(${TILT})`}
                                rx={across()}
                                ry={across() * FLATTENING}
                                {...style.attrs(styles.dashed)}
                            />
                        );
                    })}

                    {/* draw the axis the planets line up on */}
                    <g transform={`rotate(${TILT})`} opacity={line()}>
                        <line
                            x1={-reach}
                            y1={0}
                            x2={-reach * 0.15}
                            y2={0}
                            {...style.attrs(styles.dotted)}
                        />
                        <line
                            x1={0}
                            y1={0}
                            x2={reach}
                            y2={0}
                            pathLength={1}
                            stroke-dasharray="1 1"
                            stroke-dashoffset={1 - line()}
                        />
                    </g>

                    {/* draw each sun with its corona, moving in until the three are one */}
                    {planets.map((planet) => {
                        const centre = () => between(planet.sun, ORIGIN, merge());

                        return (
                            <g transform={`translate(${centre().x} ${centre().y})`}>
                                {Array.from({ length: RAYS }, (_, ray) => {
                                    const angle = (ray * 2 * Math.PI) / RAYS;
                                    const length = ray % 2 === 0 ? 20 : 15;

                                    return (
                                        <line
                                            x1={11 * Math.cos(angle)}
                                            y1={11 * Math.sin(angle)}
                                            x2={length * Math.cos(angle)}
                                            y2={length * Math.sin(angle)}
                                        />
                                    );
                                })}
                                <circle r={8} {...style.attrs(styles.cover)} />
                                <circle r={8} />
                                <circle r={3} {...style.attrs(styles.solid)} />
                            </g>
                        );
                    })}

                    {/* set each planet on its orbit, sliding into line */}
                    {planets.map((planet, index) => {
                        // turn from where the planet stands to the line
                        const start = () => (planet.phase + (360 * turned()) / planet.year) % 360;
                        const degrees = () => start() + (360 - start()) * slide();

                        // circle the moving sun on a widening orbit
                        const at = () => {
                            // move the orbit's centre and widen it, then place the planet on it
                            const centre = between(planet.sun, ORIGIN, merge());
                            const across = planet.loose + (planet.across - planet.loose) * merge();
                            const offset = onOrbit(
                                across,
                                across * FLATTENING,
                                (degrees() * Math.PI) / 180,
                            );

                            return { x: centre.x + offset.x, y: centre.y + offset.y };
                        };

                        return (
                            <g transform={`translate(${at().x} ${at().y})`}>
                                <Globe id={`syzygy-${index}`} radius={planet.radius} spacing={3} />
                            </g>
                        );
                    })}
                </g>
            </Sketch>
        </DrawingFrame>
    );
}
