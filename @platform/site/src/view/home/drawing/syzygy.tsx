import * as style from "@destack/style";
import { createSignal } from "@destack/view";

import {
    CANVAS,
    CHANGE,
    createChange,
    DrawingFrame,
    Globe,
    onOrbit,
    Sketch,
    stageOf,
    styles,
    TILT,
} from "./drawing";

/** The flattening of every orbit. */
const FLATTENING = 0.34;

/** The planets from the sun out: orbit, size, and their stacked angle in degrees. */
const planets = [
    { across: 78, radius: 11, scattered: 138 },
    { across: 136, radius: 15, scattered: 296 },
    { across: 196, radius: 13, scattered: 214 },
] as const;

/** The rays of the sun's corona, evenly around it. */
const RAYS = 16;

/** Draw the figure's apps as planets scattered round their orbits, or lined up into one stack. */
export function Syzygy(properties: { isOpen: boolean }) {
    // follow the switch: the planets slide along their orbits to 0°, then the line draws through them
    const [host, setHost] = createSignal<HTMLElement>();
    const aligned = createChange(() => properties.isOpen, host, CHANGE);
    const slide = () => stageOf(aligned(), 0, 0.75);
    const line = () => stageOf(aligned(), 0.6, 1);
    const reach = 222;

    return (
        <DrawingFrame box={CANVAS} reference={setHost}>
            <Sketch box={CANVAS}>
                <g {...style.attrs(styles.ink)}>
                    {/* draw the orbits, and the axis the planets line up on */}
                    <g transform={`rotate(${TILT})`}>
                        {planets.map((planet) => (
                            <ellipse
                                rx={planet.across}
                                ry={planet.across * FLATTENING}
                                {...style.attrs(styles.dashed)}
                            />
                        ))}
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

                    {/* draw the sun with its corona */}
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

                    {/* set each planet on its orbit, sliding into line */}
                    {planets.map((planet, index) => {
                        const degrees = () => planet.scattered + (360 - planet.scattered) * slide();
                        const at = () =>
                            onOrbit(
                                planet.across,
                                planet.across * FLATTENING,
                                (degrees() * Math.PI) / 180,
                            );

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
