import * as style from "@destack/style";
import { createMemo, createSignal } from "@destack/view";

import {
    CANVAS,
    createClock,
    DrawingFrame,
    RingedPlanet,
    Sketch,
    stageOf,
    styles,
} from "../home/drawing/drawing";

/** How far below the drawing's middle the hosts stand, in CSS pixels. */
const GROUND = 70;

/** How far apart the hosts stand, in CSS pixels. */
const SPACING = 150;

/** The planet's radius, in CSS pixels. */
const RADIUS = 20;

/** How far above a host's top the planet rests, in CSS pixels. */
const HOVER = 34;

/** How high the planet arcs between two hosts, in CSS pixels. */
const ARC = 46;

/** The seconds the planet rests on a host before it moves on. */
const REST = 3;

/** The seconds the planet takes to move from one host to the next. */
const HOP = 1.2;

/** The hosts the planet visits in turn, there and back: a laptop, a server rack and the cloud. */
const ROUTE = [0, 1, 2, 1] as const;

/** Each host's top edge, which the planet rests above. */
const TOPS = [GROUND - 64, GROUND - 124, GROUND - 92] as const;

/** Draw our planet, your software, moving between a laptop, a rack and the cloud it runs on. */
export function Hosts() {
    // count the drawing's seconds: the host the planet rests on, the next, and how far it has moved
    const [host, setHost] = createSignal<HTMLElement>();
    const seconds = createClock(host);
    const leg = () => Math.floor(seconds() / (REST + HOP)) % ROUTE.length;
    const from = () => ROUTE[leg()] ?? 0;
    const to = () => ROUTE[(leg() + 1) % ROUTE.length] ?? 0;
    const hop = () => stageOf(seconds() % (REST + HOP), REST, REST + HOP);

    // place the planet on an arc from one host to the next
    const place = () => {
        // move across and between the two hosts' tops, lifted most halfway
        const x = (from() - 1 + (to() - from()) * hop()) * SPACING;
        const rest = (TOPS[from()] ?? 0) + ((TOPS[to()] ?? 0) - (TOPS[from()] ?? 0)) * hop();
        const lift = ARC * Math.sin(Math.PI * hop());

        return { x, y: rest - HOVER - RADIUS - lift };
    };

    // light the host the planet is nearer to
    const lit = createMemo(() => (hop() < 0.5 ? from() : to()));

    return (
        <DrawingFrame box={CANVAS} reference={setHost}>
            <Sketch box={CANVAS}>
                <g {...style.attrs(styles.ink)}>
                    {/* the ground the hosts stand on */}
                    <line
                        x1={-SPACING - 80}
                        x2={SPACING + 80}
                        y1={GROUND}
                        y2={GROUND}
                        {...style.attrs(styles.dashed)}
                    />

                    {/* a laptop, its screen lit while the planet rests on it */}
                    <g transform={`translate(${-SPACING} 0)`}>
                        <path
                            d={`M-44 ${GROUND - 6}V${GROUND - 64}H44V${GROUND - 6}`}
                            {...style.attrs(styles.cover)}
                        />
                        <path d={`M-44 ${GROUND - 6}V${GROUND - 64}H44V${GROUND - 6}`} />
                        <rect
                            x={-37}
                            y={GROUND - 57}
                            width={74}
                            height={44}
                            {...style.attrs(lit() === 0 ? styles.solid : styles.dotted)}
                            opacity={lit() === 0 ? 0.35 : 1}
                        />
                        <path d={`M-56 ${GROUND - 6}H56L62 ${GROUND}H-62Z`} />
                    </g>

                    {/* a server rack, its lights lit while the planet rests on it */}
                    <g>
                        <rect
                            x={-32}
                            y={GROUND - 124}
                            width={64}
                            height={124}
                            {...style.attrs(styles.cover)}
                        />
                        <rect x={-32} y={GROUND - 124} width={64} height={124} />
                        {[0, 1, 2, 3, 4].map((unit) => {
                            const top = GROUND - 118 + unit * 23;

                            return (
                                <g>
                                    <rect x={-26} y={top} width={52} height={19} />
                                    <line x1={-20} x2={6} y1={top + 7} y2={top + 7} />
                                    <line x1={-20} x2={6} y1={top + 12} y2={top + 12} />
                                    <circle
                                        cx={17}
                                        cy={top + 9.5}
                                        r={2.5}
                                        {...style.attrs(lit() === 1 ? styles.solid : styles.cover)}
                                    />
                                </g>
                            );
                        })}
                    </g>

                    {/* the cloud, solid while the planet rests in it */}
                    <g transform={`translate(${SPACING} 0)`}>
                        <path
                            d={`M-56 ${GROUND - 2}A24 24 0 0 1 -44 ${GROUND - 50}A32 32 0 0 1 14 ${GROUND - 88}A28 28 0 0 1 52 ${GROUND - 56}A24 24 0 0 1 56 ${GROUND - 2}Z`}
                            {...style.attrs(styles.cover)}
                        />
                        <path
                            d={`M-56 ${GROUND - 2}A24 24 0 0 1 -44 ${GROUND - 50}A32 32 0 0 1 14 ${GROUND - 88}A28 28 0 0 1 52 ${GROUND - 56}A24 24 0 0 1 56 ${GROUND - 2}Z`}
                            {...style.attrs(lit() === 2 ? hostStyles.strong : styles.dashed)}
                        />
                    </g>

                    {/* the planet, resting on one host, then arcing over to the next */}
                    <g transform={`translate(${place().x} ${place().y})`}>
                        <RingedPlanet id="hosts" radius={RADIUS} spacing={3.5} />
                    </g>
                </g>
            </Sketch>
        </DrawingFrame>
    );
}

/** The hosts drawing's own styles. */
const hostStyles = style.create({
    strong: {
        strokeWidth: 1.75,
    },
});
