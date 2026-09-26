import { color, fontFamily } from "@destack/theme/tokens.stylex";
import * as stylex from "@destack/style";
import { createMemo, createSignal, For, type JSX, onSettled } from "@destack/view";

import { Debris } from "../effect/debris";
import { isDarkPage } from "../effect/gl";
import { charge } from "../effect/goo";
import { type Bob, Ice } from "../effect/ice";
import { sound } from "../effect/sound";
import { Sparks } from "../effect/sparks";
import {
    drainAt,
    nightWater,
    paperWater,
    stir,
    travel,
    Water,
    waterSpill,
    waveAt,
} from "../effect/water";
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
import { orbitOf } from "./plate";
import { appUses, Remix, sceneSources, scenes, slotApps, todayScenes } from "./remix";

/** The media query for screens narrower than the desktop frame, where the drawing spans the whole frame. */
const narrow = "@media (max-width: 1099px)";
/** The media query for phone-width screens. */
const mobile = "@media (max-width: 767px)";
/** The height of the track the switch sits in above the drawing on narrow screens. */
const switchTrack = "4rem";
/** The media query for readers who prefer reduced motion. */
const still = "@media (prefers-reduced-motion: reduce)";

/** The switch's name for the stack today, in scare quotes. */
const stackName = "\u201cStack\u201d";
/** How far each letter of the stack's name sits off the line, in pixels. */
const jumble = [0.5, -0.5, 1, -0.25, 0.5, -1, 0.25];

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
};

/** The six layers, from the users of the stack down to where it runs. */
const layers: readonly Layer[] = [
    {
        name: "Users",
        claim: { today: "Beg for entry", destack: "Bring everyone" },
        detail: { today: "Their accounts", destack: "One account, Every agent" },
    },
    {
        name: "Apps",
        claim: { today: "Duct-tape silos", destack: "Remix software" },
        detail: { today: "Closed apps", destack: "TS, HTML, CSS" },
    },
    {
        name: "Services",
        claim: { today: "Await roadmaps", destack: "Standardise logic" },
        detail: { today: "Private APIs", destack: "HTTP, OpenAPI" },
    },
    {
        name: "Data",
        claim: { today: "Rent your data", destack: "Own your data" },
        detail: { today: "Vendor formats", destack: "SQL, JSON, MD, S3" },
    },
    {
        name: "Source",
        claim: { today: "Trust blindly", destack: "Fork the code" },
        detail: { today: "Closed source", destack: "Git, npm" },
    },
    {
        name: "Hosts",
        claim: { today: "Pay double markup", destack: "Run everywhere" },
        detail: { today: "Their cloud", destack: "Node, Docker, Workers" },
    },
];

/** The icons of the layers each silo keeps under water, from services down to hosts. */
const sunkIcons = ["services", "storage", "source", "cloud"];

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
const tapeDrops = [
    [-5, 3],
    [4, -4],
];

/** The connectors taped between the vendor apps; eleven, a prime, so every swap beside a strip gives it a new one. */
const tapeLabels = [
    "APIs",
    "MCPs",
    "Zapier",
    "Webhooks",
    "CSV",
    "Make",
    "Embeds",
    "n8n",
    "iCal",
    "Scripts",
    "Exports",
];
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
    const isOpen = createMemo(() => stack() === "destack");
    let figure!: HTMLElement;
    let drawing!: HTMLDivElement;
    let canvas!: HTMLCanvasElement;
    let iceCanvas!: HTMLCanvasElement;
    let lens!: HTMLDivElement;
    let water: Water | undefined;
    let sparks: Sparks | undefined;
    let sparkCanvas!: HTMLCanvasElement;
    let swirling: ReturnType<typeof setInterval> | undefined;
    let ice: Ice | undefined;
    let debris: Debris | undefined;
    let toggle!: HTMLButtonElement;
    let pointer!: SVGPathElement;
    let stackWord!: HTMLSpanElement;
    let destackWord!: HTMLSpanElement;
    let debrisCanvas!: HTMLCanvasElement;
    let settle: ReturnType<typeof setTimeout> | undefined;
    let calm: ReturnType<typeof setTimeout> | undefined;

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
        const bounds = drawing.getBoundingClientRect();
        const origin = canvas.getBoundingClientRect();
        frame.offset = bounds.top - origin.top;
        frame.shift = bounds.left - origin.left;
        frame.width = bounds.width;
        frame.height = bounds.height;
        frame.depth = canvas.clientHeight;
    };

    // charge the goo while the switch promises Destack
    const prime = (isPrimed: boolean) => charge(isPrimed && !isOpen());

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
        for (let index = dryRows; index < layers.length; index++) {
            const passed = (level - offset - row * index) / row;
            const reveal = Math.max(0, Math.min(1, passed));
            figure.style.setProperty(`--reveal-${index}`, reveal.toFixed(3));

            // pluck a note as each band comes into view, climbing the chord
            if (reveals[index] < 0.5 && reveal >= 0.5) {
                sound.pluck(index - dryRows);
            }
            reveals[index] = reveal;
        }
    };

    // aim the searchlight at the pointer
    let stirredAt: number | undefined;
    const aim = (event: PointerEvent) => {
        // aim the light at the pointer, and turn it off over controls
        const bounds = figure.getBoundingClientRect();
        light.target = { x: event.clientX - bounds.left, y: event.clientY - bounds.top };
        const isControl = (event.target as Element).closest("button, a") !== null;
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
        const bounds = drawing.getBoundingClientRect();
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
        prime(false);

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
            const bounds = drawing.getBoundingClientRect();
            const origin = sparkCanvas.getBoundingClientRect();
            const y = frame.offset + (frame.height * dryRows) / layers.length;
            for (const centre of columnCentres) {
                sparks.burst(
                    bounds.left - origin.left + (bounds.width * centre) / boardCells,
                    y,
                    14,
                );
            }
            const began = performance.now();
            swirling = setInterval(() => {
                if (performance.now() - began > travel) {
                    clearInterval(swirling);
                }
                sparks!.swirl(
                    canvas.clientWidth * 0.3,
                    canvas.clientWidth - waterSpill * 2,
                    light.waterline + 10,
                    2,
                );
            }, 120);
        }

        // sound the change, and switch the background from sea to music or back
        sound.play(next === "destack" ? "destack" : "restack");
        sound.follow(next);

        // send shards off the ice up into the planet's ring, or bring them home to the reforming ice
        if (next === "destack") {
            debris?.rise(shardOrigins());
        } else {
            debris?.fall();
        }

        // sound each shard's flight, breaking off and chiming into the ring, or falling home to the freezing ice
        if (debris) {
            const now = performance.now();
            const flights = debris.shards.map((shard) => ({
                leaveIn: (shard.at - now) / 1000,
                reachIn: (shard.at + shard.duration - now) / 1000,
                position: (shard.home.x - window.scrollX) / window.innerWidth,
            }));
            if (next === "destack") {
                sound.shatter(flights);
            } else {
                sound.gather(flights);
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

    // start the ice, water, and searchlight once the figure is in the page
    onSettled(() => {
        // fit the switch's bar to each word once the poster face has loaded
        const fitBar = () => {
            toggle.style.setProperty("--stack-width", `${stackWord.offsetWidth}px`);
            toggle.style.setProperty("--destack-width", `${destackWord.offsetWidth}px`);
        };
        fitBar();

        // draw the arrow from the promise's verb down to the switch, wherever the layout puts them
        const aimArrow = () => {
            // find the verb, or leave the arrow out on pages without it
            const word = document.querySelector<HTMLElement>('[data-word="unify"]');
            if (!word) {
                return;
            }

            // run from under the word, bend down, and come in level with the switch
            const origin = figure.getBoundingClientRect();
            const from = word.getBoundingClientRect();
            const to = toggle.getBoundingClientRect();
            const start = {
                x: (from.left + from.right) / 2 - origin.left,
                y: from.bottom + 6 - origin.top,
            };
            const end = { x: to.left - 12 - origin.left, y: (to.top + to.bottom) / 2 - origin.top };
            pointer.setAttribute(
                "d",
                `M${start.x} ${start.y} C${start.x} ${end.y} ${start.x + (end.x - start.x) * 0.4} ${end.y} ${end.x} ${end.y} M${end.x - 9} ${end.y - 6} L${end.x} ${end.y} L${end.x - 9} ${end.y + 6}`,
            );
        };
        void document.fonts.ready.then(() => {
            fitBar();
            aimArrow();
        });
        window.addEventListener("resize", aimArrow);

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

        // pick the water palette for the page theme
        const palette = () => (isDarkPage() ? nightWater : paperWater);

        // start the ice and water, or leave the figure dry when the browser has no WebGL
        const waterline = () => (frame.height * dryRows) / layers.length;
        const masses = columnCentres.map(() => ({ lift: 0, sway: 0, tilt: 0, at: 0 }));
        let riders:
            | { element: HTMLElement; berg: number; lift: number; rest: number; height: number }[]
            | undefined;
        let sunk: { element: HTMLElement; berg: number; depth: number }[] | undefined;

        // float a heavy berg on the waves: heave and lean a little with the water under it, and slide slowly to and fro
        const bobOf = (berg: number, seconds: number): Bob => {
            // read the waves under the berg's centre
            const x = frame.shift + (frame.width / boardCells) * columnCentres[berg];
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
            const state = masses[berg];
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
            for (let node: HTMLElement | null = element; node && node !== drawing;) {
                bottom += node.offsetTop;
                node = node.offsetParent as HTMLElement | null;
            }

            return bottom;
        };
        let strips: SVGElement[] | undefined;
        const tapeCentres: Point[] = columnCentres.map(() => ({ x: 0, y: 0 }));
        const tapeTilts: number[] = columnCentres.map(() => 0);

        // stick a tape to where its cards actually are, then span, turn, and stretch it
        const stick = (tape: SVGElement, gap: number, centres: Point[], tilts: number[]) => {
            // find each tape end on its tilted card
            const cell = frame.width / boardCells;
            const middle = (frame.height / (rowCells * layers.length)) * rowCells * 1.5;
            const half = (cell * columnWidth) / 2 - tapeGrip;
            const anchor = (berg: number, side: number, drop: number) => {
                const angle = (tilts[berg] * Math.PI) / 180;
                const x = side * half;

                return {
                    x: centres[berg].x + x * Math.cos(angle) - drop * Math.sin(angle),
                    y: centres[berg].y + x * Math.sin(angle) + drop * Math.cos(angle),
                };
            };
            const from = anchor(gap, 1, tapeDrops[gap][0]);
            const to = anchor(gap + 1, -1, tapeDrops[gap][1]);
            const rest = cell * (columnCentres[gap] + columnCentres[gap + 1]) * 0.5;
            const length = Math.hypot(to.x - from.x, to.y - from.y);
            const turn = (Math.atan2(to.y - from.y, to.x - from.x) * 180) / Math.PI;
            tape.style.translate = `${((from.x + to.x) / 2 - rest).toFixed(2)}px ${((from.y + to.y) / 2 - middle).toFixed(2)}px`;
            tape.style.rotate = `${turn.toFixed(2)}deg`;
            tape.style.scale = `${(length / tapeLength).toFixed(4)} 1`;
        };
        measure();
        try {
            const centres = columnCentres.map((centre) => centre / boardCells);
            ice = new Ice(iceCanvas, centres, !isStill, bobOf, () => {
                // collect the cards riding the bergs and the cards sunk inside them, once
                riders ??= [...figure.querySelectorAll<HTMLElement>("[data-bob]")].map(
                    (element) => ({
                        element,
                        berg: Number(element.dataset.bob),
                        lift: 0,
                        rest: restingBottom(element),
                        height: element.offsetHeight,
                    }),
                );
                sunk ??= [...figure.querySelectorAll<HTMLElement>("[data-sunk]")].map(
                    (element) => ({
                        element,
                        berg: Number(element.dataset.sunk),
                        depth: Number(element.dataset.depth),
                    }),
                );
                const bobs = ice!.bobs;
                const cell = frame.width / boardCells;

                // read where each showing silo is dragged or springing home to, less its centring, before any writes
                const drags = riders.map((rider) => {
                    // skip hidden silos
                    const card = rider.element.parentElement!;
                    if (card.style.opacity === "0") {
                        return undefined;
                    }

                    // read the rendered offset
                    const offset = new DOMMatrixReadOnly(getComputedStyle(card).transform);

                    return { x: offset.m41, y: offset.m42 + rider.height / 2 };
                });

                // float each vendor card low above its berg, moving with it
                for (const [index, rider] of riders.entries()) {
                    // move the rider with its berg
                    const bob = bobs[rider.berg];
                    const sink = waterline() - rider.rest + rider.height * cardDraft;
                    rider.lift = sink + bob.lift;
                    tapeTilts[rider.berg] = bob.tilt;
                    rider.element.style.setProperty("--lift", `${rider.lift.toFixed(2)}px`);
                    rider.element.style.setProperty("--sway", `${bob.sway.toFixed(2)}px`);
                    rider.element.style.setProperty("--tilt", `${bob.tilt.toFixed(2)}deg`);

                    // hold the showing silo's tapes where it floats, dragged or not
                    const drag = drags[index];
                    if (drag) {
                        tapeCentres[rider.berg] = {
                            x: cell * columnCentres[rider.berg] + bob.sway + drag.x,
                            y: rider.rest - rider.height + rider.lift + drag.y,
                        };
                    }
                }
                strips ??= [...figure.querySelectorAll<SVGElement>("[data-tape]")];
                for (const tape of strips) {
                    stick(tape, Number(tape.dataset.tape), tapeCentres, tapeTilts);
                }

                // swing each sunk card around its berg's pivot on the waterline
                const row = frame.height / layers.length;
                for (const card of sunk) {
                    const bob = bobs[card.berg];
                    const angle = (bob.tilt * Math.PI) / 180;
                    const depth = row * (card.depth + 0.5);
                    const x = bob.sway - depth * Math.sin(angle);
                    const y = bob.lift + depth * Math.cos(angle) - depth;
                    card.element.style.translate = `${x.toFixed(2)}px ${y.toFixed(2)}px`;
                    card.element.style.rotate = `${bob.tilt.toFixed(2)}deg`;
                }
            });
            ice.place(waterline(), frame.height - waterline(), frame.shift);
            water = new Water(canvas, palette(), waterlineOf(stack()), !isStill, follow);
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
            console.error("water rendering failed", error);
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
            const boxes = [...figure.querySelectorAll<HTMLElement>("[data-inside]")];
            const places = boxes.map((box) => box.parentElement!.getBoundingClientRect());
            const pointer = { x: light.target.x + bounds.left, y: light.target.y + bounds.top };
            const isOverBox = places.some(
                (place, index) =>
                    pointer.x >= place.left &&
                    pointer.x <= place.right &&
                    pointer.y >= place.top &&
                    pointer.y <= place.bottom &&
                    boxes[index].parentElement!.checkVisibility({ opacityProperty: true }),
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
            boxes.forEach((box, index) => {
                // show only the insides the lens touches, and leave the rest out of the page's painting
                const place = places[index];
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
        const sight = new IntersectionObserver(([entry]) => {
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
        });
        sight.observe(figure);

        return () => {
            // stop the observers, timers, and frames
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
            ref={figure}
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
                            "--track": String(index() + 2),
                            ...(index() < dryRows
                                ? {}
                                : { opacity: `calc(0.75 + 0.25 * var(--reveal-${index()}))` }),
                        }}
                        {...stylex.attrs(styles.claim)}
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
                    </div>
                )}
            </For>

            {/* drift flotsam along the waterline, in front of the cards and under the water, once it has been calm a while */}
            <Flotsam isAdrift={isAdrift()} surfacedAt={surfacedAt()} waterline="var(--waterline)" />

            {/* draw both configurations in the six middle columns */}
            <div ref={drawing} {...stylex.attrs(lattice.ruleRight, styles.drawing)}>
                <canvas
                    ref={iceCanvas}
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
                            {slotApps[berg].map((id) => (
                                <div
                                    class={
                                        stylex.attrs(
                                            styles.sunkCard,
                                            todayScenes[today()].lower[berg].id !== id &&
                                                styles.sunkAway,
                                        ).class
                                    }
                                >
                                    <Card
                                        entity={locked[id][index]}
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
                        label={tapeLabel(index, todayScenes[today()])}
                        gap={index}
                        left={`calc(${tokens.cell} * ${(columnCentres[index] + columnCentres[index + 1]) / 2})`}
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
                            active={isLive() ? activeOf(scene())[index] : []}
                        />
                    </div>
                ))}
                {/* move users, agents, and apps around freely over the layers below */}
                <Remix
                    isOpen={isOpen()}
                    today={today()}
                    isLive={isLive()}
                    revealOf={(row) => reveals[row]}
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

            {/* point from the promise's verb to the switch */}
            <svg aria-hidden="true" {...stylex.attrs(styles.arrow)}>
                <path ref={pointer} {...stylex.attrs(styles.arrowLine)} />
            </svg>

            {/* switch between today and Destack on a key hung across the seam above the drawing */}
            <button
                data-universe
                type="button"
                role="switch"
                aria-label="Destack"
                aria-checked={isOpen() ? "true" : "false"}
                onClick={() => select(isOpen() ? "today" : "destack")}
                onPointerEnter={() => prime(true)}
                onPointerLeave={() => prime(false)}
                ref={toggle}
                {...stylex.attrs(styles.switch)}
            >
                {/* slide a bar under the chosen word, nudging toward Destack until it gets there */}
                <span
                    aria-hidden="true"
                    {...stylex.attrs(styles.knob, isOpen() ? styles.knobOn : styles.knobNudge)}
                >
                    {/* draw a crooked line under the stack, and a straight one under Destack */}
                    <svg
                        viewBox="0 0 100 8"
                        preserveAspectRatio="none"
                        {...stylex.attrs(styles.scrawl, isOpen() && styles.scrawlGone)}
                    >
                        <path d="M1 4.5 C 20 2.5, 35 6, 55 4.5 S 85 3, 99 5" />
                    </svg>
                    <span {...stylex.attrs(styles.rule, isOpen() && styles.ruleShown)} />
                </span>
                <span
                    ref={stackWord}
                    aria-hidden="true"
                    {...stylex.attrs(styles.key, styles.keyStack, !isOpen() && styles.keyOn)}
                >
                    {[...stackName].map((letter, index) => (
                        <span
                            style={{ translate: `0 ${jumble[index % jumble.length]}px` }}
                            {...stylex.attrs(styles.jumbled)}
                        >
                            {letter}
                        </span>
                    ))}
                </span>
                <span
                    ref={destackWord}
                    aria-hidden="true"
                    {...stylex.attrs(styles.key, styles.keyDestack, isOpen() && styles.keyOn)}
                >
                    Destack
                </span>
            </button>

            {/* stand in for the water until the shader paints its first frame */}
            <div
                aria-hidden="true"
                {...stylex.attrs(styles.pool, (isPainted() || isOpen()) && styles.poolGone)}
            />
            <canvas
                ref={canvas}
                aria-hidden="true"
                style={{
                    height: `calc(100% + ${waterSpill}px)`,
                    left: `-${waterSpill}px`,
                    width: `calc(100% + ${waterSpill * 2}px)`,
                }}
                {...stylex.attrs(styles.water, !isPainted() && styles.unpainted)}
            />

            {/* carry shards of ice between the bergs and the planet's ring, over the whole page */}
            <canvas ref={debrisCanvas} aria-hidden="true" {...stylex.attrs(styles.debris)} />

            {/* glow sparks and motes over the water */}
            <canvas ref={sparkCanvas} aria-hidden="true" {...stylex.attrs(styles.sparks)} />

            {/* ring the searchlight that follows the pointer */}
            <div ref={lens} aria-hidden="true" {...stylex.attrs(styles.lens)}>
                <svg viewBox="0 0 100 100" {...stylex.attrs(styles.lensRing)}>
                    <circle cx="50" cy="50" r="49" />
                    <path d="M50 -6V6M50 94V106M-6 50H6M94 50H106" />
                </svg>
            </div>
        </figure>
    );
}

/** Return the items each shared layer lights up in an open scene: the services its apps call, their stores, and its source step. */
function activeOf(scene: number): readonly (readonly string[])[] {
    // follow each app of the scene to its service and on to its store
    const uses = scenes[scene].lower.flatMap((card) => appUses[card.id]);
    const services = uses.map(([service]) => service);
    const stores = uses.map(([, store]) => store);

    return [services, stores, [sceneSources[scene], "Build"]];
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
function sunk(role: string, labels: readonly string[]): Entity[] {
    return labels.map((label, index) => ({ label, icon: sunkIcons[index], role }));
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
        properties.isPlated ? (
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
    const left = slotApps[gap].indexOf(scene.lower[gap].id);
    const right = slotApps[gap + 1].indexOf(scene.lower[gap + 1].id);

    return tapeLabels[(left + right * 3 + gap * 5) % tapeLabels.length];
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

/** The nudge of the switch's knob toward Destack, hinting that it wants to be switched. */
const nudge = stylex.keyframes({
    "0%, 80%, 100%": { translate: "0 0" },
    "86%": { translate: "18% 0" },
    "91%": { translate: "4% 0" },
    "95%": { translate: "9% 0" },
});

/** The figure styles. */
const styles = stylex.create({
    figure: {
        "--waterline": `calc(${tokens.stage} * ${dryRows})`,
        flexGrow: 1,
        gridTemplateRows: `repeat(6, ${tokens.stage})`,
        margin: 0,
        position: "relative",
        [narrow]: {
            "--waterline": `calc(${switchTrack} + ${tokens.stage} * ${dryRows})`,
            gridTemplateRows: `${switchTrack} repeat(6, ${tokens.stage})`,
        },
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
            gridRow: "var(--track)",
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
            gridRow: "2 / span 6",
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
    arrow: {
        height: "100%",
        inset: 0,
        overflow: "visible",
        pointerEvents: "none",
        position: "absolute",
        width: "100%",
        zIndex: 6,
        [narrow]: { display: "none" },
    },
    arrowLine: {
        fill: "none",
        stroke: tokens.signal,
        strokeLinecap: "round",
        strokeLinejoin: "round",
        strokeWidth: 3,
    },
    switch: {
        backgroundColor: "var(--destack-color-background)",
        borderColor: tokens.rule,
        borderStyle: "solid",
        borderWidth: tokens.hairline,
        color: color.foreground,
        columnGap: "1.5rem",
        cursor: "pointer",
        display: "grid",
        gridTemplateColumns: "auto auto",
        left: `calc(${tokens.column} * 4)`,
        paddingBlock: "0.5rem 0.375rem",
        paddingInline: "1.25rem",
        position: "absolute",
        top: 0,
        translate: "-50% -38%",
        zIndex: 6,
        [narrow]: {
            alignSelf: "center",
            gridColumn: "1 / -1",
            gridRow: 1,
            justifySelf: "center",
            left: "auto",
            position: "relative",
            translate: "none",
        },
    },
    key: {
        fontFamily: tokens.posterFont,
        fontSize: "0.9375rem",
        letterSpacing: "0.1em",
        lineHeight: 1,
        opacity: 0.4,
        paddingBottom: "0.4375rem",
        textTransform: "uppercase",
        transition: `opacity 300ms ${easing}`,
    },
    keyDestack: {
        fontFamily: fontFamily.default,
        fontSize: "0.9375rem",
        fontWeight: 800,
        letterSpacing: "0.08em",
    },
    keyStack: {
        fontFamily: '"Comic Sans MS", "Chalkboard SE", "Comic Neue", cursive',
        fontSize: "1rem",
        fontWeight: 700,
        letterSpacing: "0.02em",
        textTransform: "none",
    },
    jumbled: {
        display: "inline-block",
    },
    keyOn: {
        opacity: 1,
    },
    knob: {
        bottom: "0.125rem",
        color: color.foreground,
        height: "0.5rem",
        left: "1.25rem",
        position: "absolute",
        transition: `translate 600ms cubic-bezier(0.5, 0, 0.15, 1.15), width 600ms cubic-bezier(0.5, 0, 0.15, 1.15), color 400ms ${easing}`,
        width: "var(--stack-width)",
        [still]: { transition: "none" },
    },
    scrawl: {
        fill: "none",
        height: "100%",
        inset: 0,
        overflow: "visible",
        position: "absolute",
        stroke: "currentColor",
        strokeLinecap: "round",
        strokeWidth: 2,
        transition: `opacity 250ms ${easing}`,
        vectorEffect: "non-scaling-stroke",
        width: "100%",
    },
    scrawlGone: {
        opacity: 0,
    },
    rule: {
        backgroundColor: "currentColor",
        height: "2px",
        left: 0,
        opacity: 0,
        position: "absolute",
        right: 0,
        top: "calc(50% - 1px)",
        transition: `opacity 250ms ${easing} 350ms`,
    },
    ruleShown: {
        opacity: 1,
    },
    knobNudge: {
        animationDuration: "4.5s",
        animationIterationCount: "infinite",
        animationName: nudge,
        [still]: { animationName: "none" },
    },
    knobOn: {
        color: tokens.signal,
        translate: "calc(var(--stack-width) + 1.5rem) 0",
        width: "var(--destack-width)",
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
