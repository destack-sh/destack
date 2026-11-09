import * as style from "@destack/style";
import { createSignal } from "@destack/view";

import {
    CANVAS,
    CHANGE,
    createChange,
    DrawingFrame,
    Sketch,
    stageOf,
    styles,
} from "../home/drawing/drawing";

/** How far below the drawing's middle the silos stand, in CSS pixels. */
const GROUND = 92;

/** A silo's half width, in CSS pixels. */
const HALF = 22;

/** The spacing of the bands round a silo's body, in CSS pixels. */
const BAND = 16;

/** The height every silo grows or shrinks to once joined into one store, in CSS pixels. */
const JOINED = 128;

/** How high the joined store's one roof rises over its walls, in CSS pixels. */
const ROOF = 40;

/** The silos, one per tool: where each stands across and how tall its body is, in CSS pixels. */
const silos = [
    { x: -150, height: 104 },
    { x: -76, height: 138 },
    { x: 0, height: 118 },
    { x: 76, height: 148 },
    { x: 150, height: 96 },
] as const;

/** Where each silo stands once joined: side by side, wall to wall, around the middle. */
const packed = silos.map((_, index) => (index - (silos.length - 1) / 2) * HALF * 2);

/** Draw your tools as a row of silos, joined wall to wall into one store once de-stacked. */
export function Silos(properties: { isOpen: boolean }) {
    // follow the switch: the silos close in and level, then their walls open and one roof spans them
    const [host, setHost] = createSignal<HTMLElement>();
    const joined = createChange(() => properties.isOpen, host, CHANGE);
    const travel = () => stageOf(joined(), 0, 0.6);
    const open = () => stageOf(joined(), 0.5, 0.8);
    const roof = () => stageOf(joined(), 0.6, 1);
    const reach = HALF * silos.length;

    return (
        <DrawingFrame box={CANVAS} reference={setHost}>
            <Sketch box={CANVAS}>
                <g {...style.attrs(styles.ink)}>
                    {/* the ground the silos stand on */}
                    <line
                        x1={-200}
                        x2={200}
                        y1={GROUND}
                        y2={GROUND}
                        {...style.attrs(styles.dashed)}
                    />

                    {/* each silo: walls, bands and a domed roof, closing in and levelling as they join */}
                    {silos.map((silo, index) => {
                        // move the silo toward its place in the store, and level its height with the others
                        const x = () => silo.x + ((packed[index] ?? 0) - silo.x) * travel();
                        const height = () => silo.height + (JOINED - silo.height) * travel();
                        const top = () => GROUND - height();
                        const bands = () =>
                            Array.from(
                                { length: Math.floor(height() / BAND) - 1 },
                                (_, band) => top() + (band + 1) * BAND,
                            );
                        const isFirst = index === 0;
                        const isLast = index === silos.length - 1;

                        return (
                            <g transform={`translate(${x()} 0)`}>
                                <rect
                                    x={-HALF}
                                    y={top()}
                                    width={HALF * 2}
                                    height={height()}
                                    {...style.attrs(styles.cover)}
                                />
                                {bands().map((y) => (
                                    <line x1={-HALF} x2={HALF} y1={y} y2={y} />
                                ))}

                                {/* its walls, the inner ones opening to dashes once joined */}
                                <line
                                    x1={-HALF}
                                    x2={-HALF}
                                    y1={top()}
                                    y2={GROUND}
                                    opacity={isFirst ? 1 : 1 - open()}
                                />
                                <line
                                    x1={HALF}
                                    x2={HALF}
                                    y1={top()}
                                    y2={GROUND}
                                    opacity={isLast ? 1 : 1 - open()}
                                />
                                {isLast ? undefined : (
                                    <line
                                        x1={HALF}
                                        x2={HALF}
                                        y1={top()}
                                        y2={GROUND}
                                        opacity={open()}
                                        {...style.attrs(styles.dotted)}
                                    />
                                )}

                                {/* its own dome and spout, giving way to the store's one roof */}
                                <g opacity={1 - open()}>
                                    <path
                                        d={`M${-HALF} ${top()}A${HALF} ${HALF * 0.8} 0 0 1 ${HALF} ${top()}`}
                                    />
                                    <line
                                        x1={HALF}
                                        x2={HALF + 10}
                                        y1={top() + 10}
                                        y2={top() + 10}
                                    />
                                </g>
                            </g>
                        );
                    })}

                    {/* the store's one roof, drawn across every silo once their walls open */}
                    {roof() > 0 ? (
                        <path
                            d={`M${-reach} ${GROUND - JOINED}A${reach} ${ROOF} 0 0 1 ${reach} ${GROUND - JOINED}`}
                            pathLength={1}
                            stroke-dasharray="1 1"
                            stroke-dashoffset={1 - roof()}
                        />
                    ) : undefined}
                </g>
            </Sketch>
        </DrawingFrame>
    );
}
