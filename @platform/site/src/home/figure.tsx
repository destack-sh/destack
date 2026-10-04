import { color } from "@destack/theme/tokens.stylex";
import { present } from "@destack/schema";
import * as stylex from "@destack/style";
import { createMemo, createSignal, For, type JSX, onSettled } from "@destack/view";

import { Debris } from "../effect/debris";
import { isDarkPage, isWeakGraphics } from "../effect/gl";
import { charge } from "../effect/goo";
import { bergStagger, type Bob, crackTime, Ice } from "../effect/ice";
import { type Flight, sound } from "../effect/sound";
import { Sparks } from "../effect/sparks";
import {
    drainAt,
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
import { tokens } from "../style/tokens.stylex";
import { Band, Card, DuctTape, type Entity, type Reveal } from "./card";
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
import { orbitOf } from "../site/mark";
import { Remix, sceneSources, scenes, slotApps, todayScenes, usesOf } from "./remix";
import { telemetry } from "@destack/telemetry";
import { log } from "../site/telemetry.ts";

/** The pitch each berg cracks at, from the left to the right, so the three breaks sound apart. */
const bergPitches = [1.12, 1, 0.9];
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

/** How long a shown layer stays lit, in milliseconds. */
const litTime = 2600;

/** The six layers, from the users of the stack down to where it runs. */
const layers: readonly Layer[] = [
    {
        name: "Users",
        claim: { today: "Bargain for entry", destack: "Bring everyone" },
        detail: { today: "Their accounts", destack: "One account, Every agent" },
        item: {
            today: ({ people, vendors }) =>
                `${people} people × ${vendors} logins = ${people * vendors} logins`,
            destack: ({ people }) => `${people} people × 1 account = ${people} accounts`,
        },
    },
    {
        name: "Apps",
        claim: { today: "Duct-tape silos", destack: "Remix software" },
        detail: { today: "Closed apps", destack: "TS, HTML, CSS" },
        item: {
            today: ({ people, vendors }) =>
                `${vendors} vendors × ${people} seats = ${people * vendors} licences`,
            destack: () => "0 licences",
        },
    },
    {
        name: "Services",
        claim: { today: "Await roadmaps", destack: "Standardise logic" },
        detail: { today: "Private APIs", destack: "HTTP, OpenAPI" },
        item: {
            today: ({ vendors }) => `${vendors} separate rate-limited APIs`,
            destack: () => "1 API for every app",
        },
    },
    {
        name: "Data",
        claim: { today: "Rent your data", destack: "Own your data" },
        detail: { today: "Vendor formats", destack: "SQL, JSON, MD, S3" },
        item: {
            today: ({ vendors }) => `${vendors} separate data silos`,
            destack: () => "1 data plane",
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
        claim: { today: "Pay double markup", destack: "Run everywhere" },
        detail: { today: "Their cloud", destack: "Node, Docker, Workers" },
        item: {
            today: ({ vendors }) => `${vendors} extra compute planes`,
            destack: () => "1 compute plane",
        },
    },
];

/** The layers each vendor keeps under water, one per submerged row. */
const locked: Readonly<Record<string, readonly Entity[]>> = {
    notion: sunk("Theirs", ["API: 10 req/s", "Export: zip", "Closed source", "Their cloud"]),
    figma: sunk("Theirs", ["Plugin sandbox", "Files: .fig only", "Closed source", "Their cloud"]),
    typeform: sunk("Theirs", [
        "Paid webhooks",
        "Responses: theirs",
        "Closed source",
        "Their cloud",
    ]),
    airtable: sunk("Theirs", ["API: 5 req/s", "Export: CSV", "Closed source", "Their cloud"]),
    dropbox: sunk("Theirs", ["App review", "Links: theirs", "Closed source", "Their cloud"]),
    slack: sunk("Theirs", ["API throttled", "Export: owners", "Closed source", "Their cloud"]),
    loom: sunk("Theirs", ["No open API", "Video: theirs", "Closed source", "Their cloud"]),
    calendly: sunk("Theirs", ["Paid webhooks", "Invitees: theirs", "Closed source", "Their cloud"]),
    gdocs: sunk("Theirs", ["API quotas", "No Vault", "Closed source", "Google cloud"]),
    linear: sunk("Theirs", ["API: 2.5k/h", "Export: CSV", "Closed source", "Their cloud"]),
    github: sunk("Theirs", ["API: 5k/h", "Repos only", "Closed platform", "Their cloud"]),
    homemade: sunk("Rented", ["Supabase edge", "Supabase DB", "Private repo", "Vercel only"]),
};

/** The milliseconds each silo rides its iceberg before the next swap. */
const swapTime = 6500;

/** The layers every Destack app shares, one per band row, with the packages each holds and what each shows inside. */
const shared: readonly {
    entity: Entity;
    items: readonly (Entity & { reveal: Reveal })[];
}[] = [
    {
        entity: { label: "Services", icon: "services", role: "Shared logic", tint: "#6b5ca5" },
        items: [
            {
                label: "Access",
                tint: "#a0485f",
                icon: "auth",
                role: "Roles",
                reveal: {
                    kind: "fields",
                    rows: [
                        ["client", "forms"],
                        ["claude", "incidents"],
                        ["investor", "update"],
                    ],
                },
            },
            {
                label: "Settings",
                tint: "#6d7f86",
                icon: "settings",
                role: "JSON",
                reveal: {
                    kind: "fields",
                    rows: [
                        ["week", "monday"],
                        ["theme", "night"],
                        ["digest", "8:00"],
                    ],
                },
            },
            {
                label: "Search",
                tint: "#3d6fb0",
                icon: "search",
                role: "Full text",
                reveal: {
                    kind: "fields",
                    rows: [
                        ["plan.md", "2 hits"],
                        ["t_0931", "1 hit"],
                        ["#team", "4 hits"],
                    ],
                },
            },
            {
                label: "AI",
                tint: "#6b5ca5",
                icon: "ai",
                role: "Your own",
                reveal: {
                    kind: "fields",
                    rows: [
                        ["model", "yours"],
                        ["key", "yours"],
                        ["credits", "none"],
                    ],
                },
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
                reveal: {
                    kind: "fields",
                    rows: [
                        ["tasks", "1,204 rows"],
                        ["candidates", "86 rows"],
                        ["invoices", "42 rows"],
                    ],
                },
            },
            {
                label: "Bucket",
                tint: "#b8862b",
                icon: "bucket",
                role: "S3",
                reveal: {
                    kind: "fields",
                    rows: [
                        ["plan.md", "2 KB"],
                        ["cover.png", "48 KB"],
                        ["notes.md", "1 KB"],
                    ],
                },
            },
            {
                label: "Vault",
                tint: "#12313c",
                icon: "vault",
                role: "Secrets",
                reveal: {
                    kind: "fields",
                    rows: [
                        ["MAIL_TOKEN", "\u2022\u2022\u2022\u2022"],
                        ["MODEL_KEY", "\u2022\u2022\u2022\u2022"],
                        ["STRIPE_KEY", "\u2022\u2022\u2022\u2022"],
                    ],
                },
            },
            {
                label: "Audit",
                tint: "#5b7f2e",
                icon: "audit",
                role: "Change log",
                reveal: {
                    kind: "fields",
                    rows: [
                        ["09:41", "you"],
                        ["09:42", "claude"],
                        ["09:44", "client"],
                    ],
                },
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
                reveal: {
                    kind: "fields",
                    rows: [
                        ["3f9a2c", "pipeline view"],
                        ["e02d4f", "remix crm"],
                        ["9d44b1", "book interviews"],
                    ],
                },
            },
            {
                label: "Registry",
                tint: "#a0485f",
                icon: "registry",
                role: "npm",
                reveal: {
                    kind: "fields",
                    rows: [
                        ["@you/hiring", "1.2"],
                        ["@you/invoices", "0.4"],
                        ["@roommate/household", "2.0"],
                    ],
                },
            },
            {
                label: "Build",
                tint: "#4f8a5b",
                icon: "build",
                role: "Node",
                reveal: {
                    kind: "fields",
                    rows: [
                        ["bundle", "38 KB"],
                        ["types", "ok"],
                        ["done", "0.4s"],
                    ],
                },
            },
            {
                label: "Templates",
                tint: "#3d6fb0",
                icon: "template",
                role: "Starters",
                reveal: {
                    kind: "fields",
                    rows: [
                        ["standup", "incidents"],
                        ["hiring", "feedback"],
                        ["invoices", "household"],
                    ],
                },
            },
        ],
    },
];

/** The hosts a space can run on. */
const hosts: readonly { entity: Entity; reveal: Reveal }[] = [
    {
        entity: {
            label: "Your laptop",
            icon: "computer",
            role: "You run, we tunnel",
            tint: "#3d6fb0",
        },
        reveal: {
            kind: "code",
            name: "terminal",
            lines: ["$ destack dev", "ready on :3000", "tunnel  you.destack.sh"],
        },
    },
    {
        entity: {
            label: "Your server",
            icon: "hosts",
            role: "You run, we tunnel",
            tint: "#4f8a5b",
        },
        reveal: {
            kind: "code",
            name: "destack new",
            lines: ["$ docker compose up", "destack  running", "backups  nightly"],
        },
    },
    {
        entity: {
            label: "Your cloud",
            icon: "cloud",
            role: "You run everything",
            tint: "#6b5ca5",
        },
        reveal: {
            kind: "code",
            name: "terminal",
            lines: ["$ destack deploy --to aws", "region   eu-central-1", "billing  yours"],
        },
    },
    {
        entity: {
            label: "Our cloud",
            icon: "cloud",
            role: "We run everything",
            tint: "#c64a17",
        },
        reveal: {
            kind: "code",
            name: "terminal",
            lines: ["$ destack deploy", "regions  3", "backups  hourly"],
        },
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
const tapeLabels = ["APIs", "MCPs", "Webhooks", "CSV", "Glue code", "Cron", "Scripts", "Plugins"];
/** The gaps between the three icebergs that duct tape spans. */
const tapeGaps = [0, 1];

/** How many shards break off each berg and rise into the planet's ring. */
const shardsPerBerg = 16;

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
    const [litLayer, setLitLayer] = createSignal<number | undefined>(undefined);
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
    let lensElement: HTMLDivElement | undefined;
    let water: Water | undefined;
    let sparks: Sparks | undefined;
    let sparkCanvasElement: HTMLCanvasElement | undefined;
    let swirling: ReturnType<typeof setInterval> | undefined;
    let ice: Ice | undefined;
    let debris: Debris | undefined;
    let debrisCanvasElement: HTMLCanvasElement | undefined;
    let settle: ReturnType<typeof setTimeout> | undefined;
    let rising: ReturnType<typeof setTimeout> | undefined;
    let calm: ReturnType<typeof setTimeout> | undefined;

    // read the rendered elements or throw when the figure lacks them
    const elements = () => {
        if (
            !figureElement ||
            !drawingElement ||
            !canvasElement ||
            !iceCanvasElement ||
            !lensElement ||
            !sparkCanvasElement ||
            !debrisCanvasElement
        ) {
            throw new TypeError("the stack figure rendered without its drawing and canvases");
        }

        return {
            figure: figureElement,
            drawing: drawingElement,
            canvas: canvasElement,
            iceCanvas: iceCanvasElement,
            lens: lensElement,
            sparkCanvas: sparkCanvasElement,
            debrisCanvas: debrisCanvasElement,
        };
    };

    // set things adrift after a while on still water
    const drift = () => {
        clearTimeout(calm);
        setIsAdrift(false);
        calm = setTimeout(() => setIsAdrift(true), adriftDelay);
    };
    const reveals = layers.map(() => 0);
    const light = { target: { x: 0, y: 0 }, x: 0, y: 0, radius: 0, isOn: false, waterline: 0 };

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
        light.waterline = level;
        const { offset, height } = frame;
        const row = height / layers.length;
        const { style } = elements().figure;
        for (const [index, previous] of reveals.entries()) {
            // leave the dry rows above the waterline alone
            if (index < dryRows) {
                continue;
            }
            const passed = (level - offset - row * index) / row;
            const reveal = Math.max(0, Math.min(1, passed));
            style.setProperty(`--reveal-${index}`, reveal.toFixed(3));

            // pluck a note as each band comes into view, climbing the chord
            if (previous < 0.5 && reveal >= 0.5) {
                sound.pluck(index - dryRows);
            }
            reveals[index] = reveal;
        }
    };

    // aim the searchlight at the pointer
    let stirredAt: number | undefined;
    const aim = (event: PointerEvent) => {
        // aim the light at the pointer, and turn it off over controls
        const bounds = elements().figure.getBoundingClientRect();
        light.target = { x: event.clientX - bounds.left, y: event.clientY - bounds.top };
        const origin = event.target;
        const isControl = origin instanceof Element && origin.closest("button, a") !== null;
        light.isOn = event.pointerType === "mouse" && event.buttons === 0 && !isControl;

        // stir the water when skimming it, harder the closer the pointer
        const distance = Math.abs(light.target.y - light.waterline);
        if (!isOpen() && distance < stirRange) {
            if (stirredAt === undefined || Math.abs(light.target.x - stirredAt) > stirStep) {
                stir(light.target.x + waterSpill, 7 * (1 - distance / stirRange));
                sound.stir(1 - distance / stirRange, event.clientX / window.innerWidth);
                stirredAt = light.target.x;
            }
        } else {
            stirredAt = undefined;
        }
    };

    // put the searchlight out
    const leave = () => {
        light.isOn = false;
    };

    // pick points across the ice above and just below the waterline, in page pixels, where shards break off
    const shardOrigins = () => {
        // read the drawing and the bergs' size
        const bounds = elements().drawing.getBoundingClientRect();
        const third = frame.width / 3;
        const waterline = (frame.height * dryRows) / layers.length;
        const peak = Math.min(third * 0.42, waterline * 0.62);

        // scatter shards over each berg's ridge, a few from just under the water
        return columnCentres.flatMap((centre) =>
            Array.from({ length: shardsPerBerg }, () => {
                const isSunk = Math.random() < 0.25;
                const depth = isSunk ? -Math.random() * 50 : Math.random() * peak * 0.85;

                return {
                    x:
                        bounds.left +
                        window.scrollX +
                        (frame.width / boardCells) * centre +
                        (Math.random() - 0.5) * third * 0.7,
                    y: bounds.top + window.scrollY + waterline - depth,
                };
            }),
        );
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

        // spray sparks where the ice breaks, and swirl motes down into the drain as the water goes
        clearInterval(swirling);
        if (next === "destack" && sparks) {
            const bounds = elements().drawing.getBoundingClientRect();
            const origin = elements().sparkCanvas.getBoundingClientRect();
            const y = frame.offset + (frame.height * dryRows) / layers.length;
            for (const centre of columnCentres) {
                sparks.burst(
                    bounds.left - origin.left + (bounds.width * centre) / boardCells,
                    y,
                    14,
                );
            }
            const began = performance.now();
            const motes = sparks;
            swirling = setInterval(() => {
                if (performance.now() - began > travel) {
                    clearInterval(swirling);
                }
                motes.swirl(
                    elements().canvas.clientWidth * 0.3,
                    elements().canvas.clientWidth - waterSpill * 2,
                    light.waterline + 10,
                    2,
                );
            }, 120);
        }

        // sound the change, and switch the background from sea to music or back
        sound.play(next === "destack" ? "destack" : "restack");
        sound.follow(next);

        // crack each berg in turn, then send its shards up into the planet's ring once it bursts
        clearTimeout(rising);
        if (next === "destack") {
            for (const [berg, pitch] of bergPitches.entries()) {
                const crackIn = (berg * bergStagger) / 1000;
                sound.breakIce(crackIn, crackIn + crackTime / 1000, pitch);
            }
            rising = setTimeout(() => {
                if (debris) {
                    debris.rise(shardOrigins());
                    soundFlights(debris, (flights) => sound.shatter(flights));
                }
            }, crackTime);
        }
        // bring the shards home to the reforming ice
        else if (debris) {
            debris.fall();
            soundFlights(debris, (flights) => sound.gather(flights));
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

    // start the ice, water, and searchlight once the figure is in the page
    onSettled(() => {
        // require the rendered drawing and canvases
        const { figure, drawing, canvas, iceCanvas, lens, sparkCanvas, debrisCanvas } = elements();

        // flip the stack whenever the page's switch asks for it
        const flip = () => select(isOpen() ? "today" : "destack");
        document.addEventListener(commandEvents.switchStack, flip);

        // show a layer when the page asks: destack, bring the figure into view, and light the layer's row a while
        let unlight: ReturnType<typeof setTimeout> | undefined;
        const show = (event: Event) => {
            // refuse a show request that names no layer
            if (!(event instanceof CustomEvent) || typeof event.detail !== "number") {
                throw new TypeError("show layer requires a layer index");
            }

            // destack first, then bring the figure and the layer's row into view
            if (!isOpen()) {
                select("destack");
            }
            figure.scrollIntoView({ behavior: "smooth", block: "center" });
            setLitLayer(event.detail);
            clearTimeout(unlight);
            unlight = setTimeout(() => setLitLayer(undefined), litTime);
        };
        document.addEventListener(commandEvents.showLayer, show);

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
                debris = new Debris(debrisCanvas, orbitOf);
                sparks = new Sparks(sparkCanvas);
                sparks.drain = {
                    x: canvas.clientWidth * drainAt - waterSpill,
                    y: canvas.clientHeight + 30,
                };
            }
            water.shader.request();
        } catch (error) {
            log.error("water.render.failed", telemetry.exceptionAttributes(error));
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

        // swing the searchlight after the pointer and show inside the boxes it falls on
        let beam: number | undefined;
        const shine = () => {
            // schedule the next frame, and rest while the light is out
            beam = requestAnimationFrame(shine);
            if (!light.isOn && light.radius === 0) {
                return;
            }

            // measure the boxes with an inside, and light up only under water or over a visible box
            const bounds = figure.getBoundingClientRect();
            const boxes = [...figure.querySelectorAll<HTMLElement>("[data-inside]")].map((box) => {
                const outside = box.parentElement;
                if (!outside) {
                    throw new TypeError("a box's inside has no box around it");
                }

                return { box, outside, place: outside.getBoundingClientRect() };
            });
            const pointer = { x: light.target.x + bounds.left, y: light.target.y + bounds.top };
            const isOverBox = boxes.some(
                ({ outside, place }) =>
                    pointer.x >= place.left &&
                    pointer.x <= place.right &&
                    pointer.y >= place.top &&
                    pointer.y <= place.bottom &&
                    outside.checkVisibility({ opacityProperty: true }),
            );
            const isUnderWater = !isOpen() && light.target.y > light.waterline;
            const goal = light.isOn && (isOverBox || isUnderWater) ? (isOpen() ? 72 : 96) : 0;
            if (light.radius < 0.5 && light.isOn) {
                light.x = light.target.x;
                light.y = light.target.y;
            }
            light.radius += (goal - light.radius) * 0.16;
            light.x += (light.target.x - light.x) * 0.28;
            light.y += (light.target.y - light.y) * 0.28;
            const radius = light.radius < 0.5 ? 0 : light.radius;
            light.radius = radius;

            // move and size the lens, and light each box under it
            lens.style.opacity = radius > 0 ? "1" : "0";
            lens.style.translate = `${light.x - radius}px ${light.y - radius}px`;
            lens.style.width = `${radius * 2}px`;
            lens.style.height = `${radius * 2}px`;
            water?.shine(light.x + waterSpill, light.y, radius);
            boxes.forEach(({ box, place }) => {
                // show only the insides the lens touches, and leave the rest out of the page's painting
                const x = light.x + bounds.left;
                const y = light.y + bounds.top;
                const gap = Math.hypot(
                    Math.max(place.left - x, 0, x - place.right),
                    Math.max(place.top - y, 0, y - place.bottom),
                );
                const isLit = radius > 0 && gap < radius;
                if (isLit !== box.hasAttribute("data-lit")) {
                    box.toggleAttribute("data-lit", isLit);
                }
                if (isLit) {
                    box.style.setProperty("--lens-x", `${x - place.left}px`);
                    box.style.setProperty("--lens-y", `${y - place.top}px`);
                    box.style.setProperty("--lens-radius", `${radius}px`);
                }
            });
        };

        // pause the ice, water, and searchlight while the figure is off screen
        const sight = new IntersectionObserver((entries) => {
            for (const entry of entries) {
                // show or hide the effects, and restart the searchlight on screen
                ice?.shader.show(entry.isIntersecting);
                water?.shader.show(entry.isIntersecting);
                if (beam !== undefined) {
                    cancelAnimationFrame(beam);
                    beam = undefined;
                }
                if (entry.isIntersecting) {
                    beam = requestAnimationFrame(shine);
                }
            }
        });
        sight.observe(figure);

        return () => {
            // stop listening, and stop the observers, timers, and frames
            document.removeEventListener(commandEvents.switchStack, flip);
            document.removeEventListener(commandEvents.showLayer, show);
            clearTimeout(unlight);
            sight.disconnect();
            themes.disconnect();
            resize.disconnect();
            scheme.removeEventListener("change", repaint);
            clearTimeout(settle);
            clearTimeout(calm);
            clearInterval(swirling);
            clearInterval(swapping);
            sparks?.stop();
            debris?.stop();
            if (beam !== undefined) {
                cancelAnimationFrame(beam);
            }
            ice?.shader.dispose();
            water?.shader.dispose();
        };
    });

    return (
        <figure
            ref={figureElement}
            aria-label="Apps today compared with Destack"
            onPointerMove={aim}
            onPointerLeave={leave}
            style={{
                "--reveal-2": "0",
                "--reveal-3": "0",
                "--reveal-4": "0",
                "--reveal-5": "0",
                "--card-shadow": isOpen() ? "#ff792e" : "#12313c",
                "--card-marks": isOpen() ? "1" : "0",
            }}
            {...stylex.attrs(lattice.frame, lattice.ruleBottom, styles.figure)}
        >
            {/* say what each layer lets you do, verb first */}
            <For each={layers}>
                {(layer, index) => (
                    <div
                        data-universe
                        style={{
                            "--row": String(index() + 1),
                            ...(index() < dryRows
                                ? {}
                                : { opacity: `calc(0.85 + 0.15 * var(--reveal-${index()}))` }),
                        }}
                        {...stylex.attrs(styles.claim, litLayer() === index() && styles.claimLit)}
                    >
                        <span
                            style={numberOnWater(index())}
                            {...stylex.attrs(
                                styles.number,
                                index() < dryRows && isOpen() && styles.numberLit,
                            )}
                        >
                            0{index() + 1} {layer.name}
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

                        {/* itemise what the layer costs */}
                        <Swap
                            row={index()}
                            isOpen={isOpen()}
                            today={layer.item.today(count())}
                            destack={layer.item.destack(count())}
                            style={styles.itemText}
                        />
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
                                        reveal={{ kind: "cipher" }}
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
                        <Card
                            entity={host.entity}
                            kind="plain"
                            reveal={host.reveal}
                            style={styles.fill}
                        />
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
                    height: `calc(100% + ${waterSpill}px)`,
                    left: `-${waterSpill}px`,
                    width: `calc(100% + ${waterSpill * 2}px)`,
                }}
                {...stylex.attrs(styles.water, !isPainted() && styles.unpainted)}
            />

            {/* carry shards of ice between the bergs and the planet's ring, over the whole page */}
            <canvas ref={debrisCanvasElement} aria-hidden="true" {...stylex.attrs(styles.debris)} />

            {/* glow sparks and motes over the water */}
            <canvas ref={sparkCanvasElement} aria-hidden="true" {...stylex.attrs(styles.sparks)} />

            {/* ring the searchlight that follows the pointer */}
            <div ref={lensElement} aria-hidden="true" {...stylex.attrs(styles.lens)}>
                <svg viewBox="0 0 100 100" {...stylex.attrs(styles.lensRing)}>
                    <circle cx="50" cy="50" r="49" />
                    <path d="M50 -6V6M50 94V106M-6 50H6M94 50H106" />
                </svg>
            </div>
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

/** Sound each shard's flight, breaking off and chiming into the ring, or falling home to the freezing ice. */
function soundFlights(shattered: Debris, play: (flights: Flight[]) => void): void {
    const now = performance.now();
    play(
        shattered.shards.map((shard) => ({
            leaveIn: (shard.at - now) / 1000,
            reachIn: (shard.at + shard.duration - now) / 1000,
            position: (shard.home.x - window.scrollX) / window.innerWidth,
        })),
    );
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

/** Return the four layers a silo keeps under water, each labelled with who holds it. */
function sunk(
    role: string,
    [services, storage, source, cloud]: readonly [string, string, string, string],
): Entity[] {
    return [
        { label: services, icon: "services", role },
        { label: storage, icon: "storage", role },
        { label: source, icon: "source", role },
        { label: cloud, icon: "cloud", role },
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
                    <span {...stylex.attrs(styles.plate, !isOpen && styles.plateClosed)}>
                        {part}
                    </span>
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

/** Return a layer number colour that lights up orange as the water leaves its row. */
function numberOnWater(row: number): JSX.CSSProperties {
    if (row < dryRows) {
        return {};
    }

    return {
        color: `color-mix(in srgb, var(--destack-color-primary) calc(var(--reveal-${row}) * 100%), var(--destack-color-foreground))`,
    };
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
    claimLit: {
        backgroundColor: "rgb(255 121 46 / 16%)",
        boxShadow: `inset 3px 0 0 ${tokens.signal}`,
    },
    claim: {
        transitionDuration: "400ms",
        transitionProperty: "background-color, box-shadow",
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
        color: color.primary,
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
    plate: {
        backgroundColor: tokens.cream,
        borderColor: tokens.signalInk,
        borderStyle: "solid",
        borderWidth: "1.5px",
        boxShadow: `2px 2px 0 var(--card-shadow, ${tokens.signalInk})`,
        color: tokens.signalInk,
        fontSize: "0.75rem",
        lineHeight: 1,
        paddingBlock: "0.3125rem",
        paddingInline: "0.4375rem",
        whiteSpace: "nowrap",
    },
    plateClosed: {
        backgroundColor: "transparent",
        cursor: "not-allowed",
        borderColor: "currentColor",
        borderStyle: "dashed",
        boxShadow: "none",
        color: "inherit",
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
    lens: {
        height: 0,
        opacity: 0,
        left: 0,
        pointerEvents: "none",
        position: "absolute",
        top: 0,
        width: 0,
        zIndex: 4,
    },
    lensRing: {
        fill: "none",
        height: "100%",
        overflow: "visible",
        stroke: tokens.signal,
        strokeWidth: 1.5,
        vectorEffect: "non-scaling-stroke",
        width: "100%",
    },
    tapeGone: {
        opacity: 0,
        transition: `opacity 400ms ${easing} 100ms`,
    },
    tapeBack: {
        transition: `opacity 400ms ${easing} 2100ms`,
    },
    debris: {
        height: "100%",
        inset: 0,
        pointerEvents: "none",
        position: "fixed",
        width: "100%",
        zIndex: 5,
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
