import * as style from "@destack/style";
import { createSignal } from "@destack/view";

import { CANVAS, createClock, DrawingFrame, Sketch, styles } from "../home/drawing/drawing";

/** The dial's radius to its rim, in CSS pixels. */
const RIM = 110;

/** How far below the drawing's middle the needle's hub sits, in CSS pixels. */
const HUB_DROP = 52;

/** The dial's sweep from its first tick to its last, in degrees, centred on straight up. */
const SWEEP = 240;

/** The dial's ticks: how many spaces between them, every fifth long. */
const TICKS = 40;

/** The seconds of one reading: the needle rising with usage, then easing back. */
const READING = 7;

/** Return a point on the dial at a radius and an angle in degrees, clockwise from straight up. */
function dialOf(radius: number, degrees: number): { x: number; y: number } {
    const angle = ((degrees - 90) * Math.PI) / 180;

    return { x: radius * Math.cos(angle), y: HUB_DROP + radius * Math.sin(angle) };
}

/** Return the arc of the dial at a radius, from one angle to another in degrees. */
function arcOf(radius: number, from: number, to: number): string {
    // find the arc's ends, and whether it runs the long way round
    const start = dialOf(radius, from);
    const end = dialOf(radius, to);
    const isLarge = to - from > 180 ? 1 : 0;

    return `M${start.x.toFixed(1)} ${start.y.toFixed(1)}A${radius} ${radius} 0 ${isLarge} 1 ${end.x.toFixed(1)} ${end.y.toFixed(1)}`;
}

/** Draw a dial gauge, its needle rising and falling with usage, settling with a small wobble. */
export function Gauge() {
    // count the drawing's seconds, and read the needle's angle at each
    const [host, setHost] = createSignal<HTMLElement>();
    const seconds = createClock(host);
    const needle = () => {
        // rise and fall over a reading, with a damped wobble as it settles
        const time = seconds() % READING;
        const level = 0.25 + 0.45 * (0.5 - 0.5 * Math.cos((2 * Math.PI * time) / READING));
        const wobble = 0.02 * Math.sin(seconds() * 9) * Math.exp(-((time % 3.5) * 1.2));

        return -SWEEP / 2 + SWEEP * (level + wobble);
    };
    const tip = () => dialOf(RIM - 22, needle());
    const tail = () => dialOf(-16, needle());

    // lay the ticks round the sweep, every fifth long
    const ticks = Array.from({ length: TICKS + 1 }, (_, index) => {
        const degrees = -SWEEP / 2 + (SWEEP * index) / TICKS;
        const isLong = index % 5 === 0;

        return { from: dialOf(RIM - (isLong ? 18 : 10), degrees), to: dialOf(RIM - 4, degrees) };
    });

    return (
        <DrawingFrame box={CANVAS} reference={setHost}>
            <Sketch box={CANVAS}>
                <g {...style.attrs(styles.ink)}>
                    {/* the dial: its rim, its ticks, and an inner scale */}
                    <path d={arcOf(RIM, -SWEEP / 2, SWEEP / 2)} />
                    {ticks.map((tick) => (
                        <line x1={tick.from.x} y1={tick.from.y} x2={tick.to.x} y2={tick.to.y} />
                    ))}
                    <path
                        d={arcOf(RIM - 34, -SWEEP / 2, SWEEP / 2)}
                        {...style.attrs(styles.dashed)}
                    />
                    <path
                        d={arcOf(RIM - 40, SWEEP / 2 - 40, SWEEP / 2)}
                        {...style.attrs(gaugeStyles.band)}
                    />

                    {/* the needle on its hub */}
                    <line
                        x1={tail().x}
                        y1={tail().y}
                        x2={tip().x}
                        y2={tip().y}
                        {...style.attrs(gaugeStyles.needle)}
                    />
                    <circle cy={HUB_DROP} r={9} {...style.attrs(styles.cover)} />
                    <circle cy={HUB_DROP} r={9} />
                    <circle cy={HUB_DROP} r={3} {...style.attrs(styles.solid)} />
                </g>
            </Sketch>
        </DrawingFrame>
    );
}

/** The gauge's own styles. */
const gaugeStyles = style.create({
    needle: {
        strokeLinecap: "round",
        strokeWidth: 2.5,
    },
    band: {
        strokeWidth: 6,
    },
});
