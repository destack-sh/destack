import * as style from "@destack/style";
import { color } from "@destack/theme/tokens.stylex";
import { createEffect, createSignal, type JSX, onSettled, useEnvironment } from "@destack/view";
import { createReducedMotion } from "@destack/view/motion";
import { createEventListener } from "@destack/view/primitives/event-listener";
import { createVisibilityObserver } from "@destack/view/primitives/intersection-observer";
import { loopFrames } from "@destack/view/primitives/raf";

import { frame } from "../../layout/frame.stylex";
import { planet } from "../../layout/mark";
import { screen } from "../../layout/screen.stylex";

/** The seconds every drawing takes to change between stacked and de-stacked. */
export const CHANGE = 2.2;

/** The share of the text's ink every drawing is printed in, faint enough to sit behind the page. */
export const INK = 0.2;

/** The tilt of every drawing's axis, in degrees: the mark's ring. */
export const TILT = planet.ringTilt;

/** A drawing's box in CSS pixels, centred on the drawing's origin. */
export type Box = { width: number; height: number };

/** The canvas every section draws on: two fifths of the frame, the entry and caption tall. */
export const CANVAS: Box = { width: 480, height: 258 };

/** A point in a drawing, in CSS pixels from its origin. */
export type Point = { x: number; y: number };

/** Return a share from 0 to 1 eased in and out. */
export function ease(share: number): number {
    return share * share * (3 - 2 * share);
}

/** Return how far a stage of a change has run, from 0 to 1, given its start and end. */
export function stageOf(whole: number, start: number, end: number): number {
    return ease(Math.min(1, Math.max(0, (whole - start) / (end - start))));
}

/** Return a point between two others by a share from 0 to 1. */
export function between(from: Point, to: Point, share: number): Point {
    return { x: from.x + (to.x - from.x) * share, y: from.y + (to.y - from.y) * share };
}

/** Return a point on an ellipse tilted by the drawing's axis, at an angle in radians. */
export function onOrbit(across: number, down: number, angle: number): Point {
    const flat = { x: across * Math.cos(angle), y: down * Math.sin(angle) };
    const tilt = (TILT * Math.PI) / 180;

    return {
        x: flat.x * Math.cos(tilt) - flat.y * Math.sin(tilt),
        y: flat.x * Math.sin(tilt) + flat.y * Math.cos(tilt),
    };
}

/** Follow a figure's switch as a share from 0 stacked to 1 de-stacked, eased over some seconds. */
export function createChange(
    isOpen: () => boolean,
    host: () => Element | undefined,
    seconds: number,
): () => number {
    // hold the share, starting where the switch is
    const environment = useEnvironment();
    const isReduced = createReducedMotion(host);
    const [shown, setShown] = createSignal(isOpen() ? 1 : 0);

    // ease toward the switch frame by frame, or jump there where motion is reduced
    createEffect(
        () => ({ target: isOpen() ? 1 : 0, isStill: isReduced() }),
        ({ target, isStill }) => {
            // jump where motion is reduced
            if (isStill) {
                setShown(target);
                return;
            }

            // step by the time since the last frame, and stop once there
            let last: number | undefined;
            const stop = loopFrames((now) => {
                // count the seconds since the last frame, and step toward the switch
                const elapsed = last === undefined ? 0 : Math.min(0.1, (now - last) / 1000);
                last = now;
                const step = elapsed / seconds;
                const next =
                    shown() < target
                        ? Math.min(target, shown() + step)
                        : Math.max(target, shown() - step);
                setShown(next);
                if (next === target) {
                    stop();
                }
            }, environment.display);

            return stop;
        },
    );

    return shown;
}

/** Count the seconds a drawing has run, ticking only while it shows, may move, and is running. */
export function createClock(
    host: () => Element | undefined,
    isRunning: () => boolean = () => true,
): () => number {
    // hold the count, the motion preference and whether the drawing shows
    const environment = useEnvironment();
    const isReduced = createReducedMotion(host);
    const isShown = createVisibilityObserver(host, { initialValue: false });
    const [seconds, setSeconds] = createSignal(0);

    // tick frame by frame while the drawing shows and may move
    createEffect(
        () => isShown() && !isReduced() && isRunning(),
        (isTicking) => {
            // hold still while hidden, stopped, or where motion is reduced
            if (!isTicking) {
                return;
            }

            // advance by the time since the last frame
            let last: number | undefined;

            return loopFrames((now) => {
                const elapsed = last === undefined ? 0 : Math.min(0.1, (now - last) / 1000);
                last = now;
                setSeconds((current) => current + elapsed);
            }, environment.display);
        },
    );

    return seconds;
}

/** Hold a drawing in the entry's word column, the middle of its lines on the column's middle. */
export function DrawingFrame(properties: {
    box: Box;
    reference?: (element: HTMLElement) => void;
    children: JSX.Element;
}) {
    // match the height of the entry the drawing sits beside, which grows with a wrapped title
    let drawing: HTMLDivElement | undefined;
    let stage: HTMLDivElement | undefined;
    const [height, setHeight] = createSignal<number | undefined>(undefined);
    const measure = () => {
        const entry = drawing?.parentElement?.querySelector("header");
        setHeight(entry === null || entry === undefined ? undefined : entry.offsetHeight);
    };
    createEventListener(() => window, "resize", measure, { passive: true });

    // shift the drawing by how far the middle of its lines sits from its box's, once drawn
    const [shift, setShift] = createSignal({ x: 0, y: 0 });
    onSettled(() => {
        // size the frame to its entry first
        measure();

        // measure the lines of the drawing as first drawn, at the stage's scale
        const sketch = stage?.querySelector("svg");
        if (sketch === null || sketch === undefined) {
            throw new TypeError("a drawing frame holds no sketch");
        }
        const lines = sketch.getBBox();
        const scale =
            stage === undefined ? 1 : stage.getBoundingClientRect().width / properties.box.width;
        setShift({
            x: -(lines.x + lines.width / 2) * scale,
            y: -(lines.y + lines.height / 2) * scale,
        });
    });

    return (
        <div
            ref={(element) => {
                drawing = element;
                properties.reference?.(element);
            }}
            aria-hidden="true"
            style={height() === undefined ? undefined : { height: `${height()}px` }}
            {...style.attrs(styles.drawing)}
        >
            <div
                ref={stage}
                style={{
                    height: `${properties.box.height}px`,
                    width: `${properties.box.width}px`,
                    translate: `${shift().x}px ${shift().y}px`,
                }}
                {...style.attrs(styles.stage)}
            >
                {properties.children}
            </div>
        </div>
    );
}

/** Draw a drawing's lines over its whole box, centred on its origin, in the drawing's ink. */
export function Sketch(properties: { box: Box; children: JSX.Element }) {
    return (
        <svg
            viewBox={`${-properties.box.width / 2} ${-properties.box.height / 2} ${properties.box.width} ${properties.box.height}`}
            width={properties.box.width}
            height={properties.box.height}
            {...style.attrs(styles.sketch)}
        >
            {properties.children}
        </svg>
    );
}

/** Draw a small globe as the planet is drawn: lined along the axis, twice as densely in shadow. */
export function Globe(properties: { id: string; radius: number; spacing: number }) {
    // measure the globe: its shadow in the mark's proportions, and its latitude rows
    const radius = properties.radius;
    const spacing = properties.spacing;
    const ratio = radius / planet.radius;
    const dark = {
        x: (planet.shadow.x - planet.centre.x) * ratio,
        y: (planet.shadow.y - planet.centre.y) * ratio,
        radius: planet.shadow.radius * ratio,
    };
    const rows = Array.from(
        { length: Math.floor((radius * 2) / spacing) },
        (_, row) => -radius + (row + 0.5) * spacing,
    );
    const rotation = `rotate(${TILT})`;

    return (
        <g>
            <defs>
                <clipPath id={`${properties.id}-globe`}>
                    <circle r={radius} />
                </clipPath>
                <clipPath id={`${properties.id}-shadow`}>
                    <circle cx={dark.x} cy={dark.y} r={dark.radius} />
                </clipPath>
            </defs>
            <circle r={radius} {...style.attrs(styles.cover)} />
            <g clip-path={`url(#${properties.id}-globe)`}>
                <g transform={rotation}>
                    {rows.map((y) => (
                        <line x1={-radius} x2={radius} y1={y} y2={y} />
                    ))}
                </g>
                <g clip-path={`url(#${properties.id}-shadow)`}>
                    <g transform={rotation}>
                        {rows.map((y) => (
                            <line
                                x1={-radius}
                                x2={radius}
                                y1={y + spacing / 2}
                                y2={y + spacing / 2}
                            />
                        ))}
                    </g>
                </g>
            </g>
            <circle r={radius} />
        </g>
    );
}

/** Draw our planet, the mark's: a lined globe with its ring behind it and cut across its front. */
export function RingedPlanet(properties: { id: string; radius: number; spacing: number }) {
    // measure the ring in the mark's proportions to the globe
    const ratio = properties.radius / planet.radius;
    const across = planet.ringReach.across * ratio;
    const down = planet.ringReach.down * ratio;
    const band = planet.ringBand * ratio;
    const gap = (planet.ringGap - planet.ringBand) * ratio * 0.5;
    const rotation = `rotate(${TILT})`;
    const lines = [-band / 2, 0, band / 2].map((offset) => ({
        across: across + offset,
        down: down + offset * (down / across) * 1.6,
    }));
    const front = `M${-across * 2} 0H${across * 2}V${across * 2}H${-across * 2}Z`;

    return (
        <g>
            <defs>
                <clipPath id={`${properties.id}-front`}>
                    <path d={front} transform={rotation} />
                </clipPath>
            </defs>
            <g transform={rotation}>
                {lines.map((line) => (
                    <ellipse rx={line.across} ry={line.down} />
                ))}
            </g>
            <Globe id={properties.id} radius={properties.radius} spacing={properties.spacing} />
            <g clip-path={`url(#${properties.id}-front)`}>
                <g transform={rotation}>
                    <ellipse
                        rx={across}
                        ry={down}
                        stroke-width={band + gap * 2}
                        {...style.attrs(styles.gap)}
                    />
                    {lines.map((line) => (
                        <ellipse rx={line.across} ry={line.down} />
                    ))}
                </g>
            </g>
        </g>
    );
}

/** The drawing styles. */
export const styles = style.create({
    drawing: {
        display: { default: "grid", [screen.belowDesktop]: "none" },
        gridTemplateColumns: "minmax(0, 1fr)",
        gridTemplateRows: "minmax(0, 1fr)",
        height: frame.entry,
        insetInlineStart: 0,
        overflow: "visible",
        placeItems: "center",
        pointerEvents: "none",
        position: "absolute",
        top: 0,
        width: `calc(${frame.inset} + ${frame.word} + 1rem)`,
    },
    stage: {
        position: "relative",
        scale: 0.75,
    },
    sketch: {
        color: color.foreground,
        display: "block",
        fill: "none",
        overflow: "visible",
        stroke: "currentColor",
        strokeWidth: 1,
    },
    ink: {
        opacity: INK,
    },
    cover: {
        fill: color.background,
        stroke: "none",
    },
    dashed: {
        strokeDasharray: "3 3",
        strokeWidth: 1.25,
    },
    dotted: {
        strokeDasharray: "1 4",
    },
    dashedBoundary: {
        strokeDasharray: "6 4",
    },
    gap: {
        stroke: color.background,
    },
    coverRing: {
        fill: "none",
        stroke: "currentColor",
    },
    solid: {
        fill: "currentColor",
        stroke: "none",
    },
});
