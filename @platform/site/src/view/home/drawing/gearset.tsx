import { present } from "@destack/schema";
import * as style from "@destack/style";
import { createMemo, createSignal } from "@destack/view";

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

/** The share of a tooth's depth below the pitch circle: a dedendum of 1.25 in 2.25 modules. */
const DEDENDUM = 1.25 / 2.25;

/** The points each gear's outline passes through, enough for the finest teeth's corners. */
const SAMPLES = 360;

/** A tooth's shape: the standard gear's, or one of the shapes each vendor cuts its own way. */
type Tooth = "standard" | "square" | "saw" | "lobe" | "fine";

/** A gear's cut: the shape and number of its teeth, its pitch radius, and its teeth's depth. */
type Cut = { tooth: Tooth; teeth: number; pitch: number; depth: number };

/** How each gear is cut while the stack is split: each its own size and tooth, none meshing. */
const LOOSE: readonly Cut[] = [
    { tooth: "square", teeth: 7, pitch: 25, depth: 8 },
    { tooth: "saw", teeth: 15, pitch: 18, depth: 7 },
    { tooth: "lobe", teeth: 5, pitch: 31, depth: 10 },
    { tooth: "fine", teeth: 40, pitch: 24, depth: 2.5 },
    { tooth: "square", teeth: 4, pitch: 11, depth: 6 },
    { tooth: "lobe", teeth: 9, pitch: 18, depth: 6 },
    { tooth: "saw", teeth: 8, pitch: 28, depth: 11 },
    { tooth: "fine", teeth: 24, pitch: 13, depth: 2 },
    { tooth: "square", teeth: 12, pitch: 21, depth: 5 },
];

/** One gear of the train: its two cuts, its seat and meshing turn, and how it lies loose. */
type Gear = { own: Cut; standard: Cut; seat: Point; phase: number; loose: Point; lying: number };

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
    for (const [index, teeth] of TEETH.entries()) {
        const own = present(LOOSE[index], "the gear's own cut");
        const reach = Math.max(reachOf(standardOf(teeth)), reachOf(own));
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
        own: present(LOOSE[index], "the gear's own cut"),
        standard: standardOf(teeth),
        seat: { x: (seats[index]?.x ?? 0) - middle.x, y: (seats[index]?.y ?? 0) - middle.y + LOW },
        phase: phases[index] ?? 0,
        loose: present(scattered[index], "the gear's loose place"),
        lying: index * 47,
    }));
})();

/** Return the standard cut of a gear of some teeth, which meshes with every other. */
function standardOf(teeth: number): Cut {
    return { tooth: "standard", teeth, pitch: pitchOf(teeth), depth: 2.25 * MODULE };
}

/** Return how far a cut's teeth reach from its middle. */
function reachOf(cut: Cut): number {
    return cut.pitch + cut.depth * (1 - DEDENDUM);
}

/** Return how far out a tooth reaches at a share across it, from 0 at its root to 1 at its tip. */
function riseOf(tooth: Tooth, share: number): number {
    switch (tooth) {
        // rise up a flank, across the top, down, and along the root
        case "standard":
            return share < 0.18
                ? share / 0.18
                : share < 0.42
                  ? 1
                  : share < 0.6
                    ? 1 - (share - 0.42) / 0.18
                    : 0;
        // step straight up and down
        case "square":
            return share < 0.04
                ? share / 0.04
                : share < 0.5
                  ? 1
                  : share < 0.54
                    ? 1 - (share - 0.5) / 0.04
                    : 0;
        // ramp up the whole tooth, then drop
        case "saw":
            return share < 0.9 ? share / 0.9 : 1 - (share - 0.9) / 0.1;
        // swell round and back
        case "lobe":
            return 0.5 - 0.5 * Math.cos(2 * Math.PI * share);
        // cut many shallow standard teeth
        case "fine":
            return riseOf("standard", share);
    }
}

/** Return a gear's outline at a share from its own cut to the standard one. */
function outlineOf(own: Cut, standard: Cut, share: number): string {
    // walk round the gear, blending how far each cut reaches at each angle
    const radiusOf = (cut: Cut, angle: number) => {
        const across = ((angle * cut.teeth) / (2 * Math.PI)) % 1;
        const root = cut.pitch - cut.depth * DEDENDUM;

        return root + cut.depth * riseOf(cut.tooth, across);
    };
    const points = Array.from({ length: SAMPLES }, (_, sample) => {
        const angle = (2 * Math.PI * sample) / SAMPLES;
        const radius = radiusOf(own, angle) * (1 - share) + radiusOf(standard, angle) * share;

        return `${sample === 0 ? "M" : "L"}${(radius * Math.cos(angle)).toFixed(2)} ${(radius * Math.sin(angle)).toFixed(2)}`;
    });

    return `${points.join("")}Z`;
}

/** Draw one gear per service: each cut its own way, or all cut alike and meshed into one train. */
export function Gearset(properties: { isOpen: boolean }) {
    // follow the switch: the teeth change to the standard, then the gears move to their seats and turn
    const [host, setHost] = createSignal<HTMLElement>();
    const assembled = createChange(() => properties.isOpen, host, CHANGE);
    const seconds = createClock(host);
    const cut = createMemo(() => stageOf(assembled(), 0, 0.35));
    const seat = () => stageOf(assembled(), 0.3, 0.85);
    const turn = () => ((360 * seconds()) / TURN) * stageOf(assembled(), 0.85, 1);

    return (
        <DrawingFrame box={CANVAS} reference={setHost}>
            <Sketch box={CANVAS}>
                <g {...style.attrs(styles.ink)}>
                    {/* set each gear, from where it lies to its seat, turning in mesh once seated */}
                    {train.map((gear, index) => {
                        // turn each gear against the last, by the ratio of their teeth
                        const ratio = (TEETH[0] ?? gear.standard.teeth) / gear.standard.teeth;
                        const direction = index % 2 === 0 ? 1 : -1;
                        const at = () => between(gear.loose, gear.seat, seat());
                        const angle = () =>
                            gear.lying * (1 - seat()) +
                            gear.phase * seat() +
                            direction * ratio * turn();

                        // cut the gear between its own teeth and the standard's
                        const outline = createMemo(() => outlineOf(gear.own, gear.standard, cut()));
                        const pitch = () =>
                            gear.own.pitch + (gear.standard.pitch - gear.own.pitch) * cut();
                        const reach = Math.max(reachOf(gear.own), reachOf(gear.standard));

                        return (
                            <g transform={`translate(${at().x} ${at().y})`}>
                                <defs>
                                    <clipPath id={`gearset-gear-${index}`}>
                                        <path d={outline()} transform={`rotate(${angle()})`} />
                                    </clipPath>
                                </defs>
                                <path
                                    d={outline()}
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
                                    <path d={outline()} />
                                    <circle r={pitch() * 0.38} />
                                    <circle r={1.8} {...style.attrs(styles.solid)} />
                                    <path
                                        d={`M${-pitch() * 0.62} 0H${pitch() * 0.62}M0 ${-pitch() * 0.62}V${pitch() * 0.62}`}
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
