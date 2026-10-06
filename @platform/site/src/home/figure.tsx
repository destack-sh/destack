import { color } from "@destack/theme/tokens.stylex";
import { present } from "@destack/schema";
import * as stylex from "@destack/style";
import { createMemo, createSignal, For, type JSX, onSettled } from "@destack/view";

import { isDarkPage, isWeakGraphics } from "../effect/gl";
import { charge } from "../effect/goo";
import { type Bob, Ice } from "../effect/ice";
import { Sparks } from "../effect/sparks";
import {
    nightWater,
    paperWater,
    stir,
    travel,
    Water,
    type WaterPalette,
    waterSpill,
    waveAt,
} from "../effect/water";
import { commandEvents } from "../command/command";
import { lattice } from "../style/lattice.stylex";
import { plate } from "../style/plate.stylex";
import { tokens } from "../style/tokens.stylex";
import { Band, Card, DuctTape, type Entity } from "./card";
import {
    boardCells,
    columnCentres,
    columnLefts,
    columnWidth,
    quarterLefts,
    quarterWidth,
    rowCells,
} from "./board";
import { Flotsam } from "./flotsam";
import { Remix, sceneSources, scenes, slotApps, todayScenes, usesOf } from "./remix";
import { telemetry } from "@destack/telemetry";
import { log } from "../site/telemetry.ts";

/** The media query for screens narrower than the desktop frame, where the drawing spans the whole frame. */
const narrow = "@media (max-width: 1099px)";
/** The media query for phone-width screens. */
const mobile = "@media (max-width: 767px)";
/** The media query for readers who prefer reduced motion. */
const still = "@media (prefers-reduced-motion: reduce)";

/** The rows above the waterline today. */
const dryRows = 2;
/** How close to the waterline the pointer stirs the water, in CSS pixels. */
const stirRange = 90;
/** How far the pointer moves along the water between ripples, in CSS pixels. */
const stirStep = 18;
/** The share of a vendor card's height that sinks below the waterline, so it barely floats. */
const cardDraft = 0.1;
/** The milliseconds of calm on the water before things start to drift past. */
const adriftDelay = 20000;

/** The milliseconds the shattered ice waits before it clumps back together as the water returns. */
const reformDelay = 1100;

/** A point in drawing pixels. */
type Point = { x: number; y: number };

/** The two configurations the figure compares. */
type Stack = "today" | "destack";

/** One layer of the stack in both configurations. */
type Layer = {
    /** The layer name. */
    name: string;
    /** What you do with the layer, verb first. */
    claim: Record<Stack, string>;
    /** What the layer is made of. */
    detail: Record<Stack, string>;
    /** What the layer costs you, as a line on the bill counted from the scene's people and vendors. */
    item: Record<Stack, (count: Count) => string>;
};

/** The people and vendors in the scene on screen, which the bill counts. */
type Count = {
    /** The people and agents signed in. */
    people: number;
    /** The distinct vendors they rent from. */
    vendors: number;
};

/** The six layers, from the users of the stack down to where it runs. */
const layers: readonly Layer[] = [
    {
        name: "Users",
        claim: { today: "Juggle logins", destack: "Bring everyone in" },
        detail: { today: "Their accounts", destack: "One account, Every agent" },
        item: {
            today: ({ people, vendors }) => `${people * vendors} logins`,
            destack: ({ people }) => `${people} accounts`,
        },
    },
    {
        name: "Apps",
        claim: { today: "Duct-tape silos", destack: "Remix software" },
        detail: { today: "Closed apps", destack: "TS, HTML, CSS" },
        item: {
            today: ({ people, vendors }) => `${people * vendors} licences`,
            destack: () => "0 licences",
        },
    },
    {
        name: "Services",
        claim: { today: "Wait on roadmaps", destack: "Share one API" },
        detail: { today: "Private APIs", destack: "HTTP, OpenAPI" },
        item: {
            today: ({ vendors }) => `${vendors} APIs`,
            destack: () => "1 API",
        },
    },
    {
        name: "Data",
        claim: { today: "Rent your data", destack: "Own your data" },
        detail: { today: "Vendor formats", destack: "SQL, JSON, MD, S3" },
        item: {
            today: ({ vendors }) => `${vendors} silos`,
            destack: () => "1 data store",
        },
    },
    {
        name: "Source",
        claim: { today: "Trust blindly", destack: "Fork the code" },
        detail: { today: "Closed source", destack: "Git, npm" },
        item: { today: ({ vendors }) => `${vendors} black boxes`, destack: () => "0 black boxes" },
    },
    {
        name: "Hosts",
        claim: { today: "Pay their markup", destack: "Run anywhere" },
        detail: { today: "Their cloud", destack: "Node, Docker, Workers" },
        item: {
            today: ({ vendors }) => `${vendors} compute bills`,
            destack: () => "1 compute bill",
        },
    },
];

/** The layers each vendor keeps under water, one per submerged row. */
const locked: Readonly<Record<string, readonly Entity[]>> = {
    notion: sunk("Theirs", ["API: 10 req/s", "Export: zip", "Closed source", "On AWS"], "aws.svg"),
    figma: sunk(
        "Theirs",
        ["Plugin sandbox", "Files: .fig only", "Closed source", "On AWS"],
        "aws.svg",
    ),
    typeform: sunk(
        "Theirs",
        ["Paid webhooks", "Responses: theirs", "Closed source", "On AWS"],
        "aws.svg",
    ),
    airtable: sunk("Theirs", ["API: 5 req/s", "Export: CSV", "Closed source", "On AWS"], "aws.svg"),
    dropbox: sunk("Theirs", ["App review", "Links: theirs", "Closed source", "Own data centres"]),
    slack: sunk(
        "Theirs",
        ["API throttled", "Export: owners", "Closed source", "On AWS"],
        "aws.svg",
    ),
    granola: sunk(
        "Theirs",
        ["Read-only API", "Notes: theirs", "Closed source", "On AWS"],
        "aws.svg",
    ),
    calendly: sunk(
        "Theirs",
        ["Paid webhooks", "Invitees: theirs", "Closed source", "On GCP"],
        "googlecloud.svg",
    ),
    gdocs: sunk("Theirs", ["API quotas", "No Vault", "Closed source", "On GCP"], "googlecloud.svg"),
    linear: sunk(
        "Theirs",
        ["API: 2.5k/h", "Export: CSV", "Closed source", "On GCP"],
        "googlecloud.svg",
    ),
    github: sunk("Theirs", ["API: 5k/h", "Repos only", "Closed platform", "Own data centres"]),
    tracker: rented(
        ["supabase.svg"],
        ["supabase.svg"],
        ["github.png"],
        ["vercel.png", "sentry.png"],
    ),
    portal: rented(["clerk.png", "stripe.svg"], ["neon.svg"], ["github.png"], ["vercel.png"]),
    pipeline: rented(["replit.svg", "resend.svg"], ["neon.svg"], ["replit.svg"], ["replit.svg"]),
    budget: rented(["firebase.svg"], ["firebase.svg"], ["stackblitz.svg"], ["netlify.svg"]),
};

/** The milliseconds each silo rides its iceberg before the next swap. */
const swapTime = 6500;

/** The layers every Destack app shares, one per band row, with the packages each holds. */
const shared: readonly {
    entity: Entity;
    items: readonly Entity[];
}[] = [
    {
        entity: { label: "Services", icon: "services", role: "Shared logic", tint: "#6b5ca5" },
        items: [
            {
                label: "Access",
                tint: "#a0485f",
                icon: "auth",
                role: "Roles",
            },
            {
                label: "Settings",
                tint: "#6d7f86",
                icon: "settings",
                role: "JSON",
            },
            {
                label: "Search",
                tint: "#3d6fb0",
                icon: "search",
                role: "Full text",
            },
            {
                label: "AI",
                tint: "#6b5ca5",
                icon: "ai",
                role: "Your own",
            },
        ],
    },
    {
        entity: { label: "Data", icon: "storage", role: "Your data", tint: "#2f7d8c" },
        items: [
            {
                label: "DB",
                tint: "#2f7d8c",
                icon: "storage",
                role: "SQL",
            },
            {
                label: "Bucket",
                tint: "#b8862b",
                icon: "bucket",
                role: "S3",
            },
            {
                label: "Vault",
                tint: "#12313c",
                icon: "vault",
                role: "Secrets",
            },
            {
                label: "Audit",
                tint: "#5b7f2e",
                icon: "audit",
                role: "Change log",
            },
        ],
    },
    {
        entity: { label: "Source", icon: "source", role: "Your code", tint: "#c64a17" },
        items: [
            {
                label: "Repository",
                tint: "#c64a17",
                icon: "source",
                role: "Git",
            },
            {
                label: "Registry",
                tint: "#a0485f",
                icon: "registry",
                role: "npm",
            },
            {
                label: "Build",
                tint: "#4f8a5b",
                icon: "build",
                role: "Node",
            },
            {
                label: "Templates",
                tint: "#3d6fb0",
                icon: "template",
                role: "Starters",
            },
        ],
    },
];

/** The hosts a space can run on. */
const hosts: readonly Entity[] = [
    {
        label: "Your laptop",
        icon: "computer",
        role: "You run, we tunnel",
        tint: "#3d6fb0",
    },
    {
        label: "Your server",
        icon: "hosts",
        role: "You run, we tunnel",
        tint: "#4f8a5b",
    },
    {
        label: "Your cloud",
        icon: "cloud",
        role: "You run everything",
        tint: "#6b5ca5",
    },
    {
        label: "Our cloud",
        icon: "cloud",
        role: "We run everything",
        tint: "#c64a17",
    },
];

/** The length of a strip of duct tape, in pixels. */
const tapeLength = 72;
/** How far each end of a strip of duct tape grips into its card, in pixels. */
const tapeGrip = 13;
/** How far below each card's middle the two ends of each strip are stuck, in pixels. */
const tapeDrops: readonly (readonly [number, number])[] = [
    [-5, 3],
    [4, -4],
];

/** The connectors taped between the vendor apps; the strides that pick them stay coprime to their count, so every swap beside a strip gives it a new one. */
const tapeLabels = [
    "APIs",
    "Zapier",
    "MCPs",
    "Webhooks",
    "n8n",
    "CSV",
    "Glue code",
    "Make",
    "Cron",
    "Scripts",
    "Plugins",
];
/** The gaps between the three icebergs that duct tape spans. */
const tapeGaps = [0, 1];

/** The seconds a berg takes to follow the water most of the way, so it moves like a heavy body. */
const bergInertia = 0.9;

/** How far the bergs slide to and fro, in CSS pixels. */
const swayRange = 3;

/** Compare apps today, as icebergs, with the open Destack stack revealed by draining the water. */
export function StackFigure(properties: { onChange: (isOpen: boolean) => void }) {
    // hold the chosen stack, the scene, the canvases, and the effect timers
    const [stack, setStack] = createSignal<Stack>("today");
    const [isPainted, setIsPainted] = createSignal(false);
    const [scene, setScene] = createSignal(0);
    const [isLive, setIsLive] = createSignal(false);
    const [isAdrift, setIsAdrift] = createSignal(false);
    const [today, setToday] = createSignal(0);
    const [surfacedAt, setSurfacedAt] = createSignal(0);
    const [lit, setLit] = createSignal<number>();
    const isOpen = createMemo(() => stack() === "destack");
    const count = createMemo((): Count => {
        // count the people and vendors in the scene on screen
        const shown = todayScenes[today()];
        if (shown === undefined) {
            throw new Error(`missing scene ${today()}`);
        }

        return {
            people: shown.upper.length,
            vendors: new Set(shown.lower.map((card) => card.id)).size,
        };
    });
    let figureElement: HTMLElement | undefined;
    let drawingElement: HTMLDivElement | undefined;
    let canvasElement: HTMLCanvasElement | undefined;
    let iceCanvasElement: HTMLCanvasElement | undefined;
    let water: Water | undefined;
    let sparks: Sparks | undefined;
    let sparkCanvasElement: HTMLCanvasElement | undefined;
    let ice: Ice | undefined;
    let settle: ReturnType<typeof setTimeout> | undefined;
    let calm: ReturnType<typeof setTimeout> | undefined;

    // read the rendered elements or throw when the figure lacks them
    const elements = () => {
        if (
            !figureElement ||
            !drawingElement ||
            !canvasElement ||
            !iceCanvasElement ||
            !sparkCanvasElement
        ) {
            throw new TypeError("the stack figure rendered without its drawing and canvases");
        }

        return {
            figure: figureElement,
            drawing: drawingElement,
            canvas: canvasElement,
            iceCanvas: iceCanvasElement,
            sparkCanvas: sparkCanvasElement,
        };
    };

    // set things adrift after a while on still water
    const drift = () => {
        clearTimeout(calm);
        setIsAdrift(false);
        calm = setTimeout(() => setIsAdrift(true), adriftDelay);
    };
    const reveals = layers.map(() => 0);
    let surface = 0;

    // keep the drawing's place within the water canvas, measured only when the layout changes
    const frame = { offset: 0, shift: 0, width: 0, height: 0, depth: 0 };
    const measure = () => {
        // read the drawing's offset and size within the water canvas
        const bounds = elements().drawing.getBoundingClientRect();
        const origin = elements().canvas.getBoundingClientRect();
        frame.offset = bounds.top - origin.top;
        frame.shift = bounds.left - origin.left;
        frame.width = bounds.width;
        frame.height = bounds.height;
        frame.depth = elements().canvas.clientHeight;
    };

    // return the waterline of a configuration in canvas pixels
    const waterlineOf = (next: Stack) =>
        next === "destack"
            ? frame.depth + 12
            : frame.offset + (frame.height * dryRows) / layers.length;

    // reveal each row by how far the waterline has passed it
    const follow = (level: number) => {
        // light the rows the waterline has passed
        setIsPainted(true);
        surface = level;
        const { offset, height } = frame;
        const row = height / layers.length;
        const { style } = elements().figure;
        for (const index of reveals.keys()) {
            // leave the dry rows above the waterline alone
            if (index < dryRows) {
                continue;
            }
            const passed = (level - offset - row * index) / row;
            const reveal = Math.max(0, Math.min(1, passed));
            style.setProperty(`--reveal-${index}`, reveal.toFixed(3));
            reveals[index] = reveal;
        }
    };

    // stir the water when the pointer skims it, harder the closer it comes
    let stirredAt: number | undefined;
    const skim = (event: PointerEvent) => {
        // light the layer on the pointer's row over the drawing or the legend
        const bounds = elements().figure.getBoundingClientRect();
        const row = Math.floor(((event.clientY - bounds.top) / bounds.height) * layers.length);
        const layer = Math.max(0, Math.min(layers.length - 1, row));
        setLit(layer);

        // measure the pointer against the waterline
        const x = event.clientX - bounds.left;
        const distance = Math.abs(event.clientY - bounds.top - surface);
        if (!isOpen() && distance < stirRange) {
            if (stirredAt === undefined || Math.abs(x - stirredAt) > stirStep) {
                stir(x + waterSpill, 7 * (1 - distance / stirRange));
                stirredAt = x;
            }
        } else {
            stirredAt = undefined;
        }
    };

    // select a configuration, keep the reader's choice, and move the water
    const select = (next: Stack) => {
        // store the choice and tell the page
        setStack(next);
        properties.onChange(next === "destack");
        charge(0);

        // toss the flotsam back up as the water refills, or clear it away
        clearTimeout(calm);
        if (next === "today") {
            setIsAdrift(false);
            calm = setTimeout(() => {
                setIsAdrift(true);
                setSurfacedAt(performance.now());
            }, travel);
        } else {
            setIsAdrift(false);
        }

        // spray sparks where the ice breaks while that line is on screen
        if (next === "destack" && sparks) {
            const bounds = elements().drawing.getBoundingClientRect();
            const origin = elements().sparkCanvas.getBoundingClientRect();
            const y = frame.offset + (frame.height * dryRows) / layers.length;
            if (origin.top + y > 0 && origin.top + y < window.innerHeight) {
                for (const centre of columnCentres) {
                    sparks.burst(
                        bounds.left - origin.left + (bounds.width * centre) / boardCells,
                        y,
                        14,
                    );
                }
            }
        }

        // shatter the ice as the water drains, or clump it together just before it returns
        ice?.breakTo(next === "destack" ? 1 : 0, next === "destack" ? 0 : reformDelay);

        // bring the open stack to life only once the water has fully drained
        clearTimeout(settle);
        setIsLive(false);
        if (next === "destack") {
            settle = setTimeout(() => setIsLive(true), travel);
        }

        // move the water, or reveal every row at once without it
        if (water) {
            water.moveTo(waterlineOf(next));
        } else {
            follow(next === "destack" ? Number.MAX_SAFE_INTEGER : 0);
        }
    };

    // start the ice and water once the figure is in the page
    onSettled(() => {
        // require the rendered drawing and canvases
        const { figure, drawing, canvas, iceCanvas, sparkCanvas } = elements();

        // flip the stack whenever the page's switch asks for it
        const flip = () => select(isOpen() ? "today" : "destack");
        document.addEventListener(commandEvents.switchStack, flip);

        // read the motion preference and set the flotsam adrift
        const isStill = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
        drift();
        const scheme = window.matchMedia("(prefers-color-scheme: dark)");

        // swap one silo at a time while the stack is locked
        const swapping = isStill
            ? undefined
            : setInterval(() => {
                  if (!isOpen()) {
                      setToday((today() + 1) % todayScenes.length);
                  }
              }, swapTime);

        // start the ice and water, or leave the figure dry when the browser has no WebGL
        const waterline = () => (frame.height * dryRows) / layers.length;
        const bergs = columnCentres.map((centre) => ({
            centre,
            mass: { lift: 0, sway: 0, tilt: 0, at: 0 },
            tape: { centre: { x: 0, y: 0 }, tilt: 0 },
        }));
        const bergAt = (berg: number) => {
            // reject a berg index outside the three columns
            const found = bergs[berg];
            if (!found) {
                throw new TypeError(`missing berg ${berg}`);
            }

            return found;
        };
        let riders:
            | { element: HTMLElement; berg: number; lift: number; rest: number; height: number }[]
            | undefined;
        let sunken: { element: HTMLElement; berg: number; depth: number }[] | undefined;

        // float a heavy berg on the waves: heave and lean a little with the water under it, and slide slowly to and fro
        const bobOf = (berg: number, seconds: number): Bob => {
            // read the waves under the berg's centre
            const { centre, mass: state } = bergAt(berg);
            const x = frame.shift + (frame.width / boardCells) * centre;
            const heave = Math.sin(seconds * 0.45 + berg * 2.1) * 1.5;
            const slope = waveAt(x + 60, seconds) - waveAt(x - 60, seconds);
            const target = {
                lift: waveAt(x, seconds) * 0.6 + heave,
                sway: Math.sin(seconds * 0.25 + berg * 2.4) * swayRange,
                tilt:
                    ((Math.atan2(slope, 120) * 180) / Math.PI) * 0.5 +
                    Math.sin(seconds * 0.3 + berg * 1.4) * 0.7,
            };

            // ease the heavy berg toward what the water asks of it
            const step = state.at === 0 ? 1 : 1 - Math.exp(-(seconds - state.at) / bergInertia);
            state.at = seconds;
            state.lift += (target.lift - state.lift) * step;
            state.sway += (target.sway - state.sway) * step;
            state.tilt += (target.tilt - state.tilt) * step;

            return { lift: state.lift, sway: state.sway, tilt: state.tilt };
        };

        // return the bottom of a card's resting place, ignoring drag and float offsets
        const restingBottom = (element: HTMLElement) => {
            let bottom = element.offsetHeight;
            for (
                let node: Element | null = element;
                node instanceof HTMLElement && node !== drawing;
            ) {
                bottom += node.offsetTop;
                node = node.offsetParent;
            }

            return bottom;
        };
        let strips: SVGElement[] | undefined;

        // stick a tape to where its cards actually are, then span, turn, and stretch it
        const stick = (tape: SVGElement, gap: number) => {
            // read the bergs on both sides of the gap and where the tape grips them
            const left = bergAt(gap);
            const right = bergAt(gap + 1);
            const drops = tapeDrops[gap];
            if (!drops) {
                throw new TypeError(`missing tape drops for gap ${gap}`);
            }

            // find each tape end on its tilted card
            const cell = frame.width / boardCells;
            const middle = (frame.height / (rowCells * layers.length)) * rowCells * 1.5;
            const half = (cell * columnWidth) / 2 - tapeGrip;
            const anchor = (
                berg: { tape: { centre: Point; tilt: number } },
                side: number,
                drop: number,
            ) => {
                const angle = (berg.tape.tilt * Math.PI) / 180;
                const x = side * half;

                return {
                    x: berg.tape.centre.x + x * Math.cos(angle) - drop * Math.sin(angle),
                    y: berg.tape.centre.y + x * Math.sin(angle) + drop * Math.cos(angle),
                };
            };
            const from = anchor(left, 1, drops[0]);
            const to = anchor(right, -1, drops[1]);
            const rest = cell * (left.centre + right.centre) * 0.5;
            const length = Math.hypot(to.x - from.x, to.y - from.y);
            const turn = (Math.atan2(to.y - from.y, to.x - from.x) * 180) / Math.PI;
            tape.style.translate = `${((from.x + to.x) / 2 - rest).toFixed(2)}px ${((from.y + to.y) / 2 - middle).toFixed(2)}px`;
            tape.style.rotate = `${turn.toFixed(2)}deg`;
            tape.style.scale = `${(length / tapeLength).toFixed(4)} 1`;
        };
        measure();
        try {
            const centres = columnCentres.map((centre) => centre / boardCells);
            ice = new Ice(iceCanvas, centres, !isStill && !isWeakGraphics(), bobOf, (bobs) => {
                // collect the cards riding the bergs and the cards sunk inside them, once
                riders ??= [...figure.querySelectorAll<HTMLElement>("[data-bob]")].map(
                    (element) => ({
                        element,
                        berg: Number(element.dataset["bob"]),
                        lift: 0,
                        rest: restingBottom(element),
                        height: element.offsetHeight,
                    }),
                );
                sunken ??= [...figure.querySelectorAll<HTMLElement>("[data-sunk]")].map(
                    (element) => ({
                        element,
                        berg: Number(element.dataset["sunk"]),
                        depth: Number(element.dataset["depth"]),
                    }),
                );
                const cell = frame.width / boardCells;
                const bobAt = (berg: number) => {
                    // reject a card riding a berg the ice does not float
                    const bob = bobs[berg];
                    if (!bob) {
                        throw new TypeError(`missing bob for berg ${berg}`);
                    }

                    return bob;
                };

                // read where each showing silo is dragged or springing home to, less its centring, before any writes
                const drags = riders.map((rider) => {
                    // skip hidden silos
                    const card = rider.element.parentElement;
                    if (!card) {
                        throw new TypeError("a riding card has no silo");
                    }
                    if (card.style.opacity === "0") {
                        return { rider, drag: undefined };
                    }

                    // read the rendered offset
                    const offset = new DOMMatrixReadOnly(getComputedStyle(card).transform);

                    return { rider, drag: { x: offset.m41, y: offset.m42 + rider.height / 2 } };
                });

                // float each vendor card low above its berg, moving with it
                for (const { rider, drag } of drags) {
                    // move the rider with its berg
                    const bob = bobAt(rider.berg);
                    const berg = bergAt(rider.berg);
                    const sink = waterline() - rider.rest + rider.height * cardDraft;
                    rider.lift = sink + bob.lift;
                    berg.tape.tilt = bob.tilt;
                    rider.element.style.setProperty("--lift", `${rider.lift.toFixed(2)}px`);
                    rider.element.style.setProperty("--sway", `${bob.sway.toFixed(2)}px`);
                    rider.element.style.setProperty("--tilt", `${bob.tilt.toFixed(2)}deg`);

                    // hold the showing silo's tapes where it floats, dragged or not
                    if (drag) {
                        berg.tape.centre = {
                            x: cell * berg.centre + bob.sway + drag.x,
                            y: rider.rest - rider.height + rider.lift + drag.y,
                        };
                    }
                }
                strips ??= [...figure.querySelectorAll<SVGElement>("[data-tape]")];
                for (const tape of strips) {
                    stick(tape, Number(tape.dataset["tape"]));
                }

                // swing each sunk card around its berg's pivot on the waterline
                const row = frame.height / layers.length;
                for (const card of sunken) {
                    const bob = bobAt(card.berg);
                    const angle = (bob.tilt * Math.PI) / 180;
                    const depth = row * (card.depth + 0.5);
                    const x = bob.sway - depth * Math.sin(angle);
                    const y = bob.lift + depth * Math.cos(angle) - depth;
                    card.element.style.translate = `${x.toFixed(2)}px ${y.toFixed(2)}px`;
                    card.element.style.rotate = `${bob.tilt.toFixed(2)}deg`;
                }
            });
            ice.place(waterline(), frame.height - waterline(), frame.shift);
            water = new Water(
                canvas,
                palette(),
                waterlineOf(stack()),
                !isStill && !isWeakGraphics(),
                follow,
            );
            if (!isStill) {
                sparks = new Sparks(sparkCanvas);
            }
            water.shader.request();
        } catch (error) {
            log.error("water.render.failed", telemetry.exceptionAttributes(error, false));
        }

        // repaint on theme changes and keep the ice and water on the waterline through resizes
        const repaint = () => water?.paint(palette());
        const themes = new MutationObserver(repaint);
        themes.observe(document.documentElement, { attributeFilter: ["data-theme"] });
        scheme.addEventListener("change", repaint);
        const resize = new ResizeObserver(() => {
            // measure the drawing again, forget the riders' resting places, and float everything anew
            measure();
            riders = undefined;
            ice?.place(waterline(), frame.height - waterline(), frame.shift);
            water?.place(waterlineOf(stack()));
        });
        resize.observe(drawing);

        // pause the ice, the water and every animation while the figure is off screen
        const sight = new IntersectionObserver((entries) => {
            for (const entry of entries) {
                ice?.shader.show(entry.isIntersecting);
                water?.shader.show(entry.isIntersecting);
                figure.toggleAttribute("data-asleep", !entry.isIntersecting);
            }
        });
        sight.observe(figure);

        return () => {
            // stop listening, and stop the observers, timers, and frames
            document.removeEventListener(commandEvents.switchStack, flip);
            sight.disconnect();
            themes.disconnect();
            resize.disconnect();
            scheme.removeEventListener("change", repaint);
            clearTimeout(settle);
            clearTimeout(calm);
            clearInterval(swapping);
            sparks?.stop();
            ice?.shader.dispose();
            water?.shader.dispose();
        };
    });

    return (
        <figure
            ref={figureElement}
            aria-label="Apps today compared with Destack"
            onPointerMove={skim}
            onPointerLeave={() => setLit(undefined)}
            style={{
                "--reveal-2": "0",
                "--reveal-3": "0",
                "--reveal-4": "0",
                "--reveal-5": "0",
                "--card-marks": isOpen() ? "1" : "0",
            }}
            {...stylex.attrs(lattice.frame, lattice.ruleBottom, styles.figure)}
        >
            {/* say what each layer lets you do, verb first */}
            <For each={layers}>
                {(layer, index) => (
                    <div
                        style={{
                            "--row": String(index() + 1),
                            ...(index() < dryRows
                                ? {}
                                : { opacity: `calc(0.85 + 0.15 * var(--reveal-${index()}))` }),
                        }}
                        {...stylex.attrs(styles.claim)}
                    >
                        {/* set the layer's number beside the count it costs */}
                        <span {...stylex.attrs(styles.numberLine)}>
                            <span
                                {...stylex.attrs(
                                    styles.number,
                                    (isOpen() || lit() === index()) && styles.numberLit,
                                )}
                            >
                                0{index() + 1} {layer.name}
                            </span>
                            <Swap
                                row={index()}
                                isOpen={isOpen()}
                                today={layer.item.today(count())}
                                destack={layer.item.destack(count())}
                                style={styles.itemText}
                            />
                        </span>
                        {/* set the verb and the things the layer is made of on one baseline */}
                        <span {...stylex.attrs(styles.claimLine)}>
                            <Swap
                                row={index()}
                                isOpen={isOpen()}
                                today={layer.claim.today}
                                destack={layer.claim.destack}
                                style={styles.claimText}
                            />
                            <Swap
                                row={index()}
                                isOpen={isOpen()}
                                today={layer.detail.today}
                                destack={layer.detail.destack}
                                isPlated
                                style={styles.detailText}
                            />
                        </span>
                    </div>
                )}
            </For>

            {/* drift flotsam along the waterline, in front of the cards and under the water, once it has been calm a while */}
            <Flotsam isAdrift={isAdrift()} surfacedAt={surfacedAt()} waterline="var(--waterline)" />

            {/* draw both configurations in the six middle columns */}
            <div ref={drawingElement} {...stylex.attrs(lattice.ruleRight, styles.drawing)}>
                <canvas
                    ref={iceCanvasElement}
                    aria-hidden="true"
                    {...stylex.attrs(styles.ice, !isPainted() && styles.unpainted)}
                />

                {/* place every entity on its row and column */}
                {/* sink each silo's hidden layers inside its berg, showing only the silo that rides it now */}
                {columnLefts.map((left, berg) =>
                    [0, 1, 2, 3].map((index) => (
                        <div
                            data-sunk={String(berg)}
                            data-depth={String(index)}
                            style={{
                                left: `calc(${tokens.cell} * ${left})`,
                                width: `calc(${tokens.cell} * ${columnWidth})`,
                                top: `calc(${tokens.cellRow} * ${(dryRows + index) * 9 + 4.5})`,
                                opacity: `calc(1 - var(--reveal-${dryRows + index}))`,
                            }}
                            {...stylex.attrs(styles.column, styles.sunkSlot)}
                        >
                            {silosOf(berg).map((id) => (
                                <div
                                    class={
                                        stylex.attrs(
                                            styles.sunkCard,
                                            siloOn(present(todayScenes[today()], "scene"), berg) !==
                                                id && styles.sunkAway,
                                        ).class
                                    }
                                >
                                    <Card
                                        entity={lockedLayer(id, index)}
                                        kind="locked"
                                        style={styles.fill}
                                    />
                                </div>
                            ))}
                        </div>
                    )),
                )}
                {tapeGaps.map((index) => (
                    <DuctTape
                        label={tapeLabel(index, present(todayScenes[today()], "scene"))}
                        gap={index}
                        left={`calc(${tokens.cell} * ${(centreOf(index) + centreOf(index + 1)) / 2})`}
                        style={[styles.tape, isOpen() ? styles.tapeGone : styles.tapeBack]}
                    />
                ))}
                {shared.map((entity, index) => (
                    <div
                        style={{
                            "--row": String(dryRows + index + 1),
                            "--cascade": `${820 + index * 220}ms`,
                            "--band-reveal": `var(--reveal-${dryRows + index})`,
                            "pointer-events": isOpen() ? "auto" : "none",
                            ...growOutOfPlates(`var(--reveal-${dryRows + index})`),
                        }}
                        {...stylex.attrs(styles.band)}
                    >
                        <Band
                            entity={entity.entity}
                            items={entity.items}
                            active={isLive() ? activeOf(scene(), index) : []}
                        />
                    </div>
                ))}
                {/* move users, agents, and apps around freely over the layers below */}
                <Remix
                    isOpen={isOpen()}
                    today={today()}
                    isLive={isLive()}
                    revealOf={(row) => {
                        // read how far the water has left a row
                        const reveal = reveals[row];
                        if (reveal === undefined) {
                            throw new TypeError(`missing row ${row}`);
                        }

                        return reveal;
                    }}
                    onScene={setScene}
                />

                {hosts.map((host, index) => (
                    <div
                        style={{
                            left: `calc(${tokens.cell} * ${quarterLefts[index]})`,
                            top: `calc(${tokens.cellRow} * 49.5)`,
                            width: `calc(${tokens.cell} * ${quarterWidth})`,
                            opacity: "var(--reveal-5)",
                            "pointer-events": isOpen() ? "auto" : "none",
                            translate: "0 calc((1 - var(--reveal-5)) * 40%)",
                        }}
                        {...stylex.attrs(styles.column)}
                    >
                        <Card entity={host} kind="plain" style={styles.fill} />
                    </div>
                ))}
            </div>

            {/* stand in for the water until the shader paints its first frame */}
            <div
                aria-hidden="true"
                {...stylex.attrs(styles.pool, (isPainted() || isOpen()) && styles.poolGone)}
            />
            <canvas
                ref={canvasElement}
                aria-hidden="true"
                style={{
                    "clip-path": `inset(0 ${waterSpill}px ${waterSpill}px)`,
                    height: `calc(100% + ${waterSpill}px)`,
                    left: `-${waterSpill}px`,
                    width: `calc(100% + ${waterSpill * 2}px)`,
                }}
                {...stylex.attrs(styles.water, !isPainted() && styles.unpainted)}
            />

            {/* glow sparks over the water */}
            <canvas ref={sparkCanvasElement} aria-hidden="true" {...stylex.attrs(styles.sparks)} />

            {/* tint the lit layer across the drawing and its legend row, fading from row to row */}
            {[...layers.keys()].map((index) => (
                <span
                    aria-hidden="true"
                    style={{ "grid-row": String(index + 1) }}
                    {...stylex.attrs(styles.lit, lit() !== index && styles.litGone)}
                />
            ))}

            {/* draw the frame and the legend's rule over the water, so no line breaks where it rises */}
            <span aria-hidden="true" {...stylex.attrs(styles.frameRules)} />
            <span aria-hidden="true" {...stylex.attrs(styles.columnRule)} />
        </figure>
    );
}

/** Return the items a shared layer lights up in an open scene: the services its apps call, their stores, or its source step. */
function activeOf(scene: number, band: number): readonly string[] {
    // follow each app of the scene to its service and on to its store
    const uses = present(scenes[scene], "scene").lower.flatMap((card) => usesOf(card.id));
    const services = uses.map(([service]) => service);
    const stores = uses.map(([, store]) => store);

    // pick the items of the one band
    const active = [services, stores, [present(sceneSources[scene], "scene source"), "Build"]][
        band
    ];
    if (!active) {
        throw new TypeError(`shared band ${band} lights nothing`);
    }

    return active;
}

/** Return the centre of a board column, in cells. */
function centreOf(berg: number): number {
    const centre = columnCentres[berg];
    if (centre === undefined) {
        throw new TypeError(`missing column ${berg}`);
    }

    return centre;
}

/** Return the silos that take turns riding a berg. */
function silosOf(berg: number): readonly string[] {
    const silos = slotApps[berg];
    if (!silos) {
        throw new TypeError(`no silos ride berg ${berg}`);
    }

    return silos;
}

/** Return the silo riding a berg in a locked scene. */
function siloOn(scene: (typeof todayScenes)[number], berg: number): string {
    const card = scene.lower[berg];
    if (!card) {
        throw new TypeError(`no silo rides berg ${berg}`);
    }

    return card.id;
}

/** Return the layer a silo keeps under water at a depth. */
function lockedLayer(id: string, depth: number): Entity {
    const layer = locked[id]?.[depth];
    if (!layer) {
        throw new TypeError(`silo ${id} keeps no layer at depth ${depth}`);
    }

    return layer;
}

/** Pick the water palette for the page theme. */
function palette(): WaterPalette {
    return isDarkPage() ? nightWater : paperWater;
}

/** Return the mask that grows a shared band out of the three plates sunk in its row: three windows over the plates that widen until they meet. */
function growOutOfPlates(reveal: string): JSX.CSSProperties {
    // widen each window from its plate's span to its third of the board, overlapping a little so no seam shows
    const third = boardCells / 3;
    const windows = columnLefts.map((left, index) => {
        const end = index * third - 0.2;
        return {
            left: `calc(${tokens.cell} * (${left} + (${end - left}) * ${reveal}))`,
            width: `calc(${tokens.cell} * (${columnWidth} + (${third + 0.4 - columnWidth}) * ${reveal}))`,
        };
    });
    const layer = "linear-gradient(#000, #000)";

    return {
        opacity: `clamp(0, ${reveal} * 2.5 - 0.25, 1)`,
        "mask-image": windows.map(() => layer).join(", "),
        "mask-position": windows.map((window) => `${window.left} 0`).join(", "),
        "mask-repeat": "no-repeat",
        "mask-size": windows.map((window) => `${window.width} 100%`).join(", "),
    };
}

/** Return the four layers a silo keeps under water, each labelled with who holds it, the last with the cloud it runs on when known. */
function sunk(
    role: string,
    [services, storage, source, cloud]: readonly [string, string, string, string],
    host?: string,
): Entity[] {
    return [
        { label: services, icon: "services", role },
        { label: storage, icon: "storage", role },
        { label: source, icon: "source", role },
        host === undefined
            ? { label: cloud, icon: "cloud", role }
            : { label: cloud, icon: "cloud", role, logos: [host] },
    ];
}

/** Return the four layers a homemade silo rents under water, each showing the logos of the products behind it. */
function rented(
    services: readonly string[],
    data: readonly string[],
    source: readonly string[],
    hosting: readonly string[],
): Entity[] {
    return [
        { label: "Services", icon: "services", role: "Rented", logos: services },
        { label: "Data", icon: "storage", role: "Rented", logos: data },
        { label: "Source", icon: "source", role: "Rented", logos: source },
        { label: "Hosting", icon: "cloud", role: "Rented", logos: hosting },
    ];
}

/** Crossfade a row's text from today to Destack as the water leaves the row. */
function Swap(properties: {
    row: number;
    isOpen: boolean;
    today: string;
    destack: string;
    isPlated?: boolean;
    style?: stylex.Styles;
}) {
    // tell whether the row stays dry, and read how far the water has left it
    const isDry = properties.row < dryRows;

    // set a thing as one dashed plate today, and as one plate per standard once open
    const plated = (text: string, isOpen: boolean) =>
        properties.isPlated === true ? (
            <span {...stylex.attrs(styles.plates)}>
                {(isOpen ? text.split(", ") : [text]).map((part) => (
                    <span {...stylex.attrs(plate.plate, !isOpen && plate.closed)}>{part}</span>
                ))}
            </span>
        ) : (
            text
        );
    const reveal = `var(--reveal-${properties.row})`;

    return (
        <span {...stylex.attrs(styles.swap, properties.style)}>
            <span
                style={{
                    ...(isDry ? {} : { opacity: `clamp(0, 1 - ${reveal} * 2, 1)` }),
                    "pointer-events": properties.isOpen ? "none" : "auto",
                }}
                {...stylex.attrs(
                    styles.swapText,
                    isDry && (properties.isOpen ? styles.swapOut : styles.swapReturn),
                )}
            >
                {plated(properties.today, false)}
            </span>
            <span
                style={{
                    ...(isDry ? {} : { opacity: `clamp(0, ${reveal} * 2 - 1, 1)` }),
                    "pointer-events": properties.isOpen ? "auto" : "none",
                }}
                {...stylex.attrs(
                    styles.swapText,
                    isDry && (properties.isOpen ? styles.swapIn : styles.swapLeave),
                )}
            >
                {plated(properties.destack, true)}
            </span>
        </span>
    );
}

/** Return the label taped across a gap between two icebergs, picked by the pair of silos it joins, so any swap on either side retapes it. */
function tapeLabel(gap: number, scene: (typeof todayScenes)[number]) {
    // place each silo in its iceberg's turn, and step through the labels by coprime strides
    const left = silosOf(gap).indexOf(siloOn(scene, gap));
    const right = silosOf(gap + 1).indexOf(siloOn(scene, gap + 1));
    const label = tapeLabels[(left + right * 3 + gap * 2) % tapeLabels.length];
    if (label === undefined) {
        throw new TypeError(`no tape label for gap ${gap}`);
    }

    return label;
}

/** The easing of the figure transitions. */
const easing = "cubic-bezier(0.6, 0, 0.2, 1)";

/** The figure styles. */
const styles = stylex.create({
    figure: {
        "--waterline": `calc(${tokens.stage} * ${dryRows})`,
        flexGrow: 1,
        gridTemplateRows: `repeat(6, ${tokens.stage})`,
        margin: 0,
        position: "relative",
    },
    numberLine: {
        alignItems: "baseline",
        display: "flex",
        gap: "1rem",
        justifyContent: "space-between",
    },
    claim: {
        display: "flex",
        flexDirection: "column",
        gap: "0.25rem",
        gridColumn: "9 / span 4",
        gridRow: "var(--row)",
        justifyContent: "center",
        paddingInline: tokens.inset,
        position: "relative",
        zIndex: 3,
        [narrow]: {
            alignItems: "baseline",
            alignSelf: "start",
            columnGap: "1rem",
            flexDirection: "row",
            gridColumn: "1 / -1",
            paddingTop: "0.625rem",
        },
        [mobile]: { columnGap: "0.625rem", paddingInline: "0.75rem" },
    },
    claimLine: {
        alignItems: "baseline",
        columnGap: "1rem",
        display: "flex",
        flexWrap: "wrap",
        justifyContent: "space-between",
        rowGap: "0.375rem",
        [narrow]: { flexGrow: 1 },
    },
    number: {
        color: color.mutedForeground,
        fontFamily: tokens.monoFont,
        fontSize: "0.6875rem",
        letterSpacing: "0.1em",
        lineHeight: "1rem",
        textTransform: "uppercase",
        transition: `color 300ms ${easing}`,
        whiteSpace: "nowrap",
    },
    numberLit: {
        color: color.foreground,
    },
    swap: {
        display: "grid",
    },
    swapText: {
        gridArea: "1 / 1",
    },
    swapIn: {
        transition: `opacity 300ms ${easing} 250ms`,
        [still]: { transition: "none" },
    },
    swapOut: {
        opacity: 0,
        transition: `opacity 250ms ${easing}`,
        [still]: { transition: "none" },
    },
    swapReturn: {
        transition: `opacity 300ms ${easing} 1900ms`,
        [still]: { transition: "none" },
    },
    swapLeave: {
        opacity: 0,
        transition: `opacity 250ms ${easing} 1700ms`,
        [still]: { transition: "none" },
    },
    claimText: {
        fontSize: "1rem",
        fontWeight: 600,
        lineHeight: "1.375rem",
        whiteSpace: "nowrap",
    },
    lit: {
        backgroundColor: "rgb(255 121 46 / 8%)",
        gridColumn: "1 / -1",
        pointerEvents: "none",
        transition: `opacity 300ms ${easing}`,
        zIndex: 3,
        [narrow]: { display: "none" },
    },
    litGone: {
        opacity: 0,
    },
    frameRules: {
        borderColor: tokens.rule,
        borderStyle: "solid",
        borderWidth: `${tokens.hairline} ${tokens.hairline} ${tokens.hairline}`,
        inset: `0 calc(-1 * ${tokens.hairline}) calc(-1 * ${tokens.hairline})`,
        pointerEvents: "none",
        position: "absolute",
        zIndex: 6,
        [mobile]: { borderInlineWidth: 0 },
    },
    columnRule: {
        borderRightColor: tokens.rule,
        borderRightStyle: "solid",
        borderRightWidth: tokens.hairline,
        gridColumn: "1 / span 8",
        gridRow: "1 / span 6",
        pointerEvents: "none",
        position: "relative",
        zIndex: 6,
        [narrow]: { display: "none" },
    },
    drawing: {
        display: "grid",
        gridColumn: "1 / span 8",
        gridRow: "1 / span 6",
        gridTemplateColumns: "repeat(3, minmax(0, 1fr))",
        gridTemplateRows: "repeat(6, minmax(0, 1fr))",
        minWidth: 0,
        position: "relative",
        [narrow]: {
            borderRightWidth: 0,
            gridColumn: "1 / -1",
        },
    },
    fill: {
        width: "100%",
    },
    sunkCard: {
        gridArea: "1 / 1",
        transition: `translate 900ms ${easing} 400ms`,
        width: "100%",
        [still]: { transition: "none" },
    },
    sunkSlot: {
        overflow: "clip",
    },
    sunkAway: {
        transitionDelay: "0ms",
        translate: "0 110%",
    },
    band: {
        alignItems: "center",
        display: "grid",
        gridColumn: "1 / -1",
        gridRow: "var(--row)",
        paddingInline: `calc(${tokens.cell} * 3)`,
        position: "relative",
        zIndex: 1,
    },

    column: {
        display: "grid",
        position: "absolute",
        transform: "translateY(-50%)",
        zIndex: 1,
    },
    tape: {
        top: `calc(${tokens.cellRow} * 13.5)`,
        [mobile]: { display: "none" },
    },
    ice: {
        height: "100%",
        inset: 0,
        pointerEvents: "none",
        position: "absolute",
        transition: `opacity 400ms ${easing}`,
        width: "100%",
        zIndex: 0,
        [still]: { transition: "none" },
    },
    plates: {
        display: "flex",
        gap: "0.375rem",
        justifyContent: "flex-end",
    },
    itemText: {
        fontFamily: tokens.monoFont,
        fontSize: "0.75rem",
        letterSpacing: "0.02em",
        lineHeight: "1.125rem",
        opacity: 0.8,
        [narrow]: { display: "none" },
    },
    detailText: {
        marginLeft: "auto",
        color: color.foreground,
        fontFamily: tokens.monoFont,
        fontSize: "0.8125rem",
        fontWeight: 600,
        letterSpacing: "0.02em",
        lineHeight: "1.375rem",
        translate: "0 0.125rem",
        [mobile]: { display: "none" },
    },
    pool: {
        backgroundColor: "var(--site-water)",
        backgroundImage: "linear-gradient(transparent, rgb(0 0 0 / 45%))",
        borderTopColor: "rgb(255 255 255 / 85%)",
        borderTopStyle: "solid",
        borderTopWidth: "2px",
        bottom: 0,
        left: 0,
        opacity: 0.9,
        pointerEvents: "none",
        position: "absolute",
        right: 0,
        top: "calc(var(--waterline) - 1px)",
        transition: `opacity 400ms ${easing}`,
        zIndex: 2,
    },
    poolGone: {
        opacity: 0,
    },
    unpainted: {
        opacity: 0,
    },
    tapeGone: {
        opacity: 0,
        transition: `opacity 400ms ${easing} 100ms`,
    },
    tapeBack: {
        transition: `opacity 400ms ${easing} 2100ms`,
    },
    sparks: {
        height: "100%",
        inset: 0,
        pointerEvents: "none",
        position: "absolute",
        width: "100%",
        zIndex: 3,
    },
    water: {
        pointerEvents: "none",
        position: "absolute",
        top: 0,
        transition: `opacity 400ms ${easing}`,
        zIndex: 2,
    },
});
