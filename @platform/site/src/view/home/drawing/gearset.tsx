import * as style from "@destack/style";
import { createSignal } from "@destack/view";

import {
    between,
    CANVAS,
    CHANGE,
    createChange,
    createClock,
    DrawingFrame,
    type Point,
    Sketch,
    stageOf,
    styles,
} from "./drawing";

/** The gears' module: the pitch circle's diameter per tooth, in CSS pixels. */
const MODULE = 4.2;

/** The teeth on each gear in the train, in turn: big and small mixed, so the train winds. */
const TEETH = [14, 9, 12, 8, 15, 10, 11, 9, 13] as const;

/** The direction from each gear to the next, in degrees, so the train winds left to right. */
const HEADINGS = [-38, 32, -30, 42, -36, 30, -40, 34] as const;

/** How far below the canvas's middle the engine sits, clear of the definition, in CSS pixels. */
const LOW = 22;

/** The spacing of the shading on each gear's side away from the light, in CSS pixels. */
const SHADE = 3.2;

/** The seconds the first gear takes to turn once while the engine runs. */
const TURN = 30;

/** One gear of the train: its teeth, its seat, the turn that meshes it, and where it lies. */
type Gear = { teeth: number; seat: Point; phase: number; loose: Point; lying: number };

/** Return a gear's pitch radius, where it meets the next gear's. */
function pitchOf(teeth: number): number {
    return (MODULE * teeth) / 2;
}

/** Return whether a gear at a point touches none already placed, and keeps clear of the switch. */
function isClearOf(
    point: Point,
    reach: number,
    placed: readonly (Point & { reach: number })[],
): boolean {
    // keep a gap to every placed gear, and stay out of the switch's corner
    const isApart = placed.every(
        (other) => Math.hypot(other.x - point.x, other.y - point.y) > other.reach + reach + 10,
    );
    const isInSwitch = point.x > 20 - reach && point.y > 60 - reach;

    return isApart && !isInSwitch;
}

/** Where each gear lies loose: scattered from a fixed seed, apart and clear of the switch. */
const scattered: readonly Point[] = (() => {
    // draw from a fixed seed, so the server and the browser agree
    let seed = 23;
    const random = () => {
        seed = (seed * 16807) % 2147483647;
        return seed / 2147483647;
    };

    // place each gear where it touches no other, clear of the canvas's edges and the switch's corner
    const placed: (Point & { reach: number })[] = [];
    for (const teeth of TEETH) {
        const reach = pitchOf(teeth) + MODULE;
        let candidate: Point = { x: 0, y: 0 };
        for (let attempt = 0; attempt < 400; attempt++) {
            candidate = {
                x: (random() - 0.5) * (CANVAS.width - reach * 2 - 16),
                y: (random() - 0.5) * (CANVAS.height - reach * 2 - 16),
            };
            if (isClearOf(candidate, reach, placed)) {
                break;
            }
        }
        placed.push({ ...candidate, reach });
    }

    return placed;
})();

/** The train: each gear seated against the last, turned to mesh, centred on the canvas. */
const train: readonly Gear[] = (() => {
    // walk from gear to gear along the headings, each a pitch radius apart from the last
    const seats: Point[] = [{ x: 0, y: 0 }];
    const phases: number[] = [0];
    TEETH.slice(1).forEach((teeth, index) => {
        // step a pitch radius on from the last gear, and turn this one so their teeth interleave
        const previous = seats[index] ?? { x: 0, y: 0 };
        const before = TEETH[index] ?? teeth;
        const heading = HEADINGS[index] ?? 0;
        const distance = pitchOf(before) + pitchOf(teeth);
        const radians = (heading * Math.PI) / 180;
        seats.push({
            x: previous.x + distance * Math.cos(radians),
            y: previous.y + distance * Math.sin(radians),
        });
        const ratio = before / teeth;
        phases.push(-ratio * (phases[index] ?? 0) + heading * (1 + ratio) + 180 / teeth);
    });

    // centre the train, and scatter its gears around the canvas for while they lie loose
    const middle = {
        x:
            (Math.min(...seats.map((seat) => seat.x)) + Math.max(...seats.map((seat) => seat.x))) /
            2,
        y:
            (Math.min(...seats.map((seat) => seat.y)) + Math.max(...seats.map((seat) => seat.y))) /
            2,
    };

    return TEETH.map((teeth, index) => ({
        teeth,
        seat: { x: (seats[index]?.x ?? 0) - middle.x, y: (seats[index]?.y ?? 0) - middle.y + LOW },
        phase: phases[index] ?? 0,
        loose: scattered[index] ?? { x: 0, y: 0 },
        lying: index * 47,
    }));
})();

/** Return a gear's outline: trapezoid teeth around its pitch circle. */
function outlineOf(teeth: number): string {
    // measure the root and tip circles
    const pitch = pitchOf(teeth);
    const root = pitch - 1.25 * MODULE;
    const tip = pitch + MODULE;
    const step = (2 * Math.PI) / teeth;

    // walk each tooth: up its flank, across its top, down, and across the root to the next
    const points = Array.from({ length: teeth }, (_, tooth) => {
        const start = tooth * step;

        return [
            [root, start],
            [tip, start + step * 0.18],
            [tip, start + step * 0.42],
            [root, start + step * 0.6],
        ] as const;
    }).flat();

    return `${points.map(([radius, angle], index) => `${index === 0 ? "M" : "L"}${(radius * Math.cos(angle)).toFixed(2)} ${(radius * Math.sin(angle)).toFixed(2)}`).join("")}Z`;
}

/** Draw one gear per service of the figure lying loose, or meshed into one turning gear train. */
export function Gearset(properties: { isOpen: boolean }) {
    // follow the switch: the gears move to their seats and start turning
    const [host, setHost] = createSignal<HTMLElement>();
    const assembled = createChange(() => properties.isOpen, host, CHANGE);
    const seconds = createClock(host);
    const seat = () => stageOf(assembled(), 0, 0.8);
    const turn = () => ((360 * seconds()) / TURN) * stageOf(assembled(), 0.8, 1);

    return (
        <DrawingFrame box={CANVAS} reference={setHost}>
            <Sketch box={CANVAS}>
                <g {...style.attrs(styles.ink)}>
                    {/* set each gear, from where it lies to its seat, turning in mesh once seated */}
                    {train.map((gear, index) => {
                        // turn each gear against the last, by the ratio of their teeth
                        const ratio = (TEETH[0] ?? gear.teeth) / gear.teeth;
                        const direction = index % 2 === 0 ? 1 : -1;
                        const at = () => between(gear.loose, gear.seat, seat());
                        const angle = () =>
                            gear.lying * (1 - seat()) +
                            gear.phase * seat() +
                            direction * ratio * turn();
                        const pitch = pitchOf(gear.teeth);
                        const reach = pitch + MODULE;
                        const outline = outlineOf(gear.teeth);

                        return (
                            <g transform={`translate(${at().x} ${at().y})`}>
                                <defs>
                                    <clipPath id={`gearset-gear-${index}`}>
                                        <path d={outline} transform={`rotate(${angle()})`} />
                                    </clipPath>
                                </defs>
                                <path
                                    d={outline}
                                    transform={`rotate(${angle()})`}
                                    {...style.attrs(styles.cover)}
                                />

                                {/* shade the gear's far side from the light, the shading holding still as it turns */}
                                <g clip-path={`url(#gearset-gear-${index})`}>
                                    {Array.from(
                                        { length: Math.ceil(reach / SHADE) },
                                        (_, line) => (line + 1) * SHADE,
                                    ).map((offset) => (
                                        <line
                                            x1={offset - reach}
                                            y1={reach}
                                            x2={reach}
                                            y2={offset - reach}
                                        />
                                    ))}
                                </g>

                                <g transform={`rotate(${angle()})`}>
                                    <path d={outline} />
                                    <circle r={pitch * 0.38} />
                                    <circle r={1.8} {...style.attrs(styles.solid)} />
                                    <path
                                        d={`M${-pitch * 0.62} 0H${pitch * 0.62}M0 ${-pitch * 0.62}V${pitch * 0.62}`}
                                        {...style.attrs(styles.dotted)}
                                    />
                                </g>
                            </g>
                        );
                    })}
                </g>
            </Sketch>
        </DrawingFrame>
    );
}
