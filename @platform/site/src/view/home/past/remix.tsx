import { frame } from "../../layout/frame.stylex";
import { color, motion } from "@destack/theme/tokens.stylex";
import { present } from "@destack/schema";
import * as style from "@destack/style";
import { createMemo, createSignal, For, onSettled } from "@destack/view";

import {
    boardCells,
    boardInset,
    columnLefts,
    columnWidth,
    quarterCentres,
    rowCells,
} from "./board";
import { Card, type Entity } from "./card";
import { listedOf } from "../figure/price";
import { palette } from "../../palette.stylex";
import { isMotionReduced } from "@destack/view/motion";
import { duration, stagger } from "../../layout/motion";
import { movement } from "../../layout/motion.stylex";
import { makeEventListener } from "@destack/view/primitives/event-listener";
import { makeIntersectionObserver } from "@destack/view/primitives/intersection-observer";
import { makeResizeObserver } from "@destack/view/primitives/resize-observer";

/** The milliseconds each open scene holds before the next one begins. */
const sceneTime = 5200;
/** The milliseconds from the water fully draining to the first scene change. */
const firstSceneTime = 1200;
/** The milliseconds after the stack opens before its apps enter, as the water drains. */
const openDelay = 280;
/** The milliseconds after the stack closes before its silos enter, as the ice reforms. */
const closeDelay = 520;

/** The humans, agents, silos, and apps the scenes arrange. */
export const entities: Record<string, Entity> = {
    // people
    you: { label: "You", icon: "user", role: "Human", tint: "#2f7d8c" },
    cofounder: { label: "Cofounder", icon: "user", role: "Human", tint: "#a0485f" },
    designer: { label: "Designer", icon: "user", role: "Human", tint: "#5b7f2e" },
    client: { label: "Client", icon: "user", role: "Guest", tint: "#8a6d3b" },
    candidate: { label: "Candidate", icon: "user", role: "Guest", tint: "#7a6aa8" },
    investor: { label: "Investor", icon: "user", role: "Guest", tint: "#3f6f73" },
    partner: { label: "Partner", icon: "user", role: "Family", tint: "#b0567f" },
    roommate: { label: "Roommate", icon: "user", role: "Friend", tint: "#6d7f3a" },

    // agents
    agent: { label: "Agent", icon: "agent", role: "Agent", tint: "#6b5ca5" },
    chatgpt: { label: "ChatGPT", icon: "openai", role: "Agent", tint: "#10a37f" },
    claude: { label: "Claude", icon: "claude", role: "Agent", tint: "#d97757" },

    // silos, at the list price of the plan a team buys
    notion: {
        label: "Notion",
        icon: "notion",
        role: `${listedOf("Notion")} + credits`,
        tint: "#ffffff",
        glyph: "#191919",
    },
    airtable: {
        label: "Airtable",
        icon: "airtable",
        role: listedOf("Airtable"),
        tint: "#18bfff",
    },
    typeform: { label: "Typeform", icon: "typeform", role: listedOf("Typeform"), tint: "#262627" },
    dropbox: { label: "Dropbox", icon: "dropbox", role: listedOf("Dropbox"), tint: "#0061ff" },
    figma: {
        label: "Figma",
        icon: "figma",
        role: `${listedOf("Figma")} + credits`,
        tint: "#f24e1e",
    },
    slack: { label: "Slack", icon: "slack", role: listedOf("Slack"), tint: "#4a154b" },
    granola: {
        label: "Granola",
        icon: "file",
        role: listedOf("Granola"),
        tint: "#9cbb3c",
        logos: ["granola.png"],
    },
    gdocs: {
        label: "Google Docs",
        icon: "googledocs",
        role: listedOf("Google Workspace"),
        tint: "#4285f4",
    },
    calendly: { label: "Calendly", icon: "calendly", role: listedOf("Calendly"), tint: "#006bff" },
    linear: {
        label: "Linear",
        icon: "linear",
        role: listedOf("Linear"),
        tint: "#5e6ad2",
    },
    github: { label: "GitHub", icon: "github", role: listedOf("GitHub"), tint: "#24292f" },
    // homemade apps, each built with an app builder on rented services
    tracker: {
        label: "Launch tracker",
        icon: "tasks",
        role: "Built with Lovable",
        tint: "#b8862b",
    },
    portal: { label: "Client portal", icon: "pages", role: "Built with v0", tint: "#3d6fb0" },
    pipeline: { label: "Hiring board", icon: "user", role: "Built with Replit", tint: "#5b7f2e" },
    budget: { label: "Family budget", icon: "table", role: "Built with Bolt", tint: "#a0485f" },

    // the homemade apps rebuilt on Destack
    ownTracker: {
        label: "Launch tracker",
        icon: "tasks",
        role: "Built on Destack",
        isRemix: true,
    },
    ownPortal: { label: "Client portal", icon: "pages", role: "Built on Destack", isRemix: true },
    ownPipeline: {
        label: "Hiring board",
        icon: "accounts",
        role: "Built on Destack",
        isRemix: true,
    },
    ownBudget: { label: "Family budget", icon: "table", role: "Built on Destack", isRemix: true },

    // apps
    pages: { label: "Pages", icon: "pages", role: "App" },
    notes: { label: "Notes", icon: "file", role: "App" },
    chat: { label: "Chat", icon: "chat", role: "App" },
    tasks: { label: "Tasks", icon: "tasks", role: "App" },
    source: { label: "Source", icon: "source", role: "App" },
    forms: { label: "Forms", icon: "template", role: "App" },
    calendar: { label: "Calendar", icon: "calendar", role: "App" },
    sheets: { label: "Sheets", icon: "table", role: "App" },
    mail: { label: "Mail", icon: "mail", role: "App" },
    files: { label: "Files", icon: "bucket", role: "App" },
    canvas: { label: "Canvas", icon: "canvas", role: "App" },

    // remixes, each named after the apps it joins
    room: { label: "Launch room", icon: "chat", role: "Pages + Chat", isRemix: true },
    roadmap: { label: "Roadmap", icon: "tasks", role: "Pages + Tasks", isRemix: true },
    issues: { label: "Issues", icon: "notify", role: "Source + Tasks", isRemix: true },
    review: { label: "Design review", icon: "canvas", role: "Canvas + Notes", isRemix: true },
    research: { label: "User research", icon: "template", role: "Forms + Notes", isRemix: true },
    interviews: {
        label: "Interviews",
        icon: "calendar",
        role: "Forms + Calendar",
        isRemix: true,
    },
    bookings: { label: "Bookings", icon: "calendar", role: "Sheets + Calendar", isRemix: true },
    report: { label: "Report", icon: "table", role: "Sheets + Pages", isRemix: true },
    handbook: { label: "Handbook", icon: "pages", role: "Files + Pages", isRemix: true },
    hub: { label: "Family hub", icon: "storage", role: "Files + Chat", isRemix: true },
};

/** The apps each remix joins, in the order its role names them. */
const remixes: Readonly<Record<string, readonly string[]>> = {
    room: ["pages", "chat"],
    roadmap: ["pages", "tasks"],
    issues: ["source", "tasks"],
    review: ["canvas", "notes"],
    research: ["forms", "notes"],
    interviews: ["forms", "calendar"],
    bookings: ["sheets", "calendar"],
    report: ["sheets", "pages"],
    handbook: ["files", "pages"],
    hub: ["files", "chat"],
};

/** The Destack app that replaces each silo, in the silo's place. */
const replacements: Readonly<Record<string, string>> = {
    notion: "pages",
    figma: "canvas",
    typeform: "forms",
    airtable: "sheets",
    dropbox: "files",
    github: "source",
    slack: "chat",
    granola: "notes",
    calendly: "calendar",
    gdocs: "pages",
    linear: "tasks",
    tracker: "ownTracker",
    portal: "ownPortal",
    pipeline: "ownPipeline",
    budget: "ownBudget",
};

/** The silos each iceberg carries in turn, from the left berg to the right. */
export const slotApps: readonly (readonly string[])[] = [
    ["notion", "figma", "typeform", "airtable", "dropbox", "github"],
    ["slack", "granola", "calendly", "gdocs", "linear"],
    ["tracker", "portal", "pipeline", "budget"],
];
/** The iceberg each silo rides. */
const slots = new Map(
    slotApps.flatMap((apps, slot) => apps.map((id): [string, number] => [id, slot])),
);
/** The silos, which ride the icebergs today. */
const vendors = new Set(slots.keys());

/** One arrangement of the top two layers. */
type Scene = {
    /** The cards on the upper row, left to right. */
    upper: readonly string[];
    /** The cards on the lower row, left to right, and how many of the three columns each spans. */
    lower: readonly { id: string; span: number }[];
    /** Which upper card works with which lower card. */
    links: readonly [string, string][];
};

/** The locked stack today, one change per step. */
export const todayScenes: readonly Scene[] = [
    locked(
        ["you", "cofounder", "designer", "agent"],
        ["notion", "slack", "tracker"],
        ["notion", "slack", "tracker", "tracker"],
    ),
    locked(
        ["you", "cofounder", "designer", "agent"],
        ["notion", "linear", "tracker"],
        ["notion", "linear", "linear", "tracker"],
    ),
    locked(
        ["you", "cofounder", "designer", "agent"],
        ["github", "linear", "tracker"],
        ["github", "linear", "tracker", "github"],
    ),
    locked(
        ["you", "cofounder", "designer", "claude"],
        ["figma", "granola", "tracker"],
        ["figma", "granola", "figma", "tracker"],
    ),
    locked(
        ["you", "cofounder", "client", "claude"],
        ["figma", "granola", "portal"],
        ["granola", "portal", "portal", "figma"],
    ),
    locked(
        ["you", "cofounder", "client", "claude"],
        ["typeform", "granola", "portal"],
        ["portal", "granola", "typeform", "portal"],
    ),
    locked(
        ["you", "cofounder", "candidate", "claude"],
        ["typeform", "calendly", "pipeline"],
        ["pipeline", "calendly", "typeform", "pipeline"],
    ),
    locked(
        ["you", "cofounder", "investor", "chatgpt"],
        ["airtable", "calendly", "portal"],
        ["airtable", "portal", "portal", "airtable"],
    ),
    locked(
        ["you", "cofounder", "investor", "chatgpt"],
        ["airtable", "gdocs", "portal"],
        ["portal", "gdocs", "gdocs", "airtable"],
    ),
    locked(
        ["you", "partner", "roommate", "chatgpt"],
        ["airtable", "gdocs", "budget"],
        ["airtable", "gdocs", "budget", "budget"],
    ),
    locked(
        ["you", "partner", "roommate", "chatgpt"],
        ["dropbox", "gdocs", "budget"],
        ["dropbox", "gdocs", "budget", "budget"],
    ),
    locked(
        ["you", "partner", "roommate", "agent"],
        ["dropbox", "slack", "budget"],
        ["slack", "dropbox", "budget", "budget"],
    ),
];

/** The open loop. */
export const scenes: readonly Scene[] = todayScenes.flatMap((scene) => {
    // replace each silo in its place, and point each person at the replacement
    const swap = (id: string) => replacements[id] ?? id;
    const lower = scene.lower.map((card) => swap(card.id));
    const [first, second, third] = lower;
    if (first === undefined || second === undefined || third === undefined) {
        throw new TypeError("a locked scene holds three silos");
    }
    const replaced: Scene = {
        upper: scene.upper,
        lower: lower.map((id) => ({ id, span: 1 })),
        links: scene.links.map(([person, target]): [string, string] => [person, swap(target)]),
    };

    // join the first two replacements into their remix beside the third
    const remix = remixOf(first, second);
    const joined = (id: string) => (id === first || id === second ? remix : id);

    return [
        replaced,
        {
            upper: scene.upper,
            lower: [
                { id: remix, span: 2 },
                { id: third, span: 1 },
            ],
            links: replaced.links.map(([person, target]): [string, string] => [
                person,
                joined(target),
            ]),
        },
    ];
});

/** Every card the scenes show, locked or open. */
const ids = [
    ...new Set(
        [...todayScenes, ...scenes].flatMap((scene) => [
            ...scene.upper,
            ...scene.lower.map((card) => card.id),
        ]),
    ),
];

/** The services each open app calls, each with the store it keeps the app's state in. */
const appUses: Readonly<Record<string, readonly (readonly [string, string])[]>> = withRemixes({
    pages: [
        ["Access", "DB"],
        ["Search", "Bucket"],
    ],
    notes: [
        ["AI", "Vault"],
        ["Search", "DB"],
    ],
    chat: [
        ["Access", "DB"],
        ["AI", "Vault"],
    ],
    tasks: [
        ["Access", "DB"],
        ["Settings", "DB"],
    ],
    source: [
        ["Access", "Audit"],
        ["Search", "Bucket"],
    ],
    forms: [
        ["Access", "DB"],
        ["Settings", "DB"],
    ],
    calendar: [
        ["Access", "DB"],
        ["Settings", "DB"],
    ],
    sheets: [
        ["Access", "DB"],
        ["AI", "Vault"],
    ],
    files: [
        ["Access", "Bucket"],
        ["Search", "Bucket"],
    ],
    canvas: [
        ["Access", "Bucket"],
        ["AI", "Vault"],
    ],
    ownTracker: [
        ["Access", "DB"],
        ["Settings", "Audit"],
    ],
    ownPortal: [
        ["Access", "DB"],
        ["Search", "Bucket"],
    ],
    ownPipeline: [
        ["Access", "DB"],
        ["AI", "Vault"],
    ],
    ownBudget: [
        ["Access", "Audit"],
        ["Settings", "DB"],
    ],
});

/** The source step each open scene shows at work, by its label. */
export const sceneSources = scenes.map((scene, index) => {
    // build a remix, or install from each source in turn, two scenes each
    const source = scene.lower.some((card) => card.id in remixes)
        ? "Build"
        : ["Registry", "Repository", "Templates"][Math.floor(index / 2) % 3];
    if (source === undefined) {
        throw new TypeError(`scene ${index} has no source step`);
    }

    return source;
});

/** How far the user row sits below its figure row's middle, in CSS pixels, to clear the switch. */
const userDrop = 8;

/** The hole row the hosts' tops sit on, counted from the layer's top. */
const hostTop = 47;
/** The hole row the cables from the build run across to the hosts. */
const hostBus = 44.5;

/** The board's height in cells across the three figure rows the layer covers. */
const layerRows = rowCells * 3;

/** The placement of every card in each scene of the locked stack. */
const todayPlacements = todayScenes.map(arrange);
/** The placement of every card in each open scene. */
const scenePlacements = scenes.map(arrange);

/** How a card changes into its placement: kept, shown, hidden, moved, or at rest. */
type Step = "stay" | "enter" | "leave" | "move" | "park";

/** One placed card: its row, its span in whole board cells, and how it gets there. */
type Placement = {
    /** The row, upper or lower. */
    row: 0 | 1;
    /** The span's left edge in cells from the board's edge. */
    left: number;
    /** The span's width in cells. */
    width: number;
    /** Whether the card shows on the board. */
    isShown: boolean;
    /** How the card moves there. */
    step: Step;
    /** The milliseconds the card waits before it changes. */
    delay: number;
};

/** One kind of cable, which sets its look. */
type Kind = "locked" | "link" | "chain" | "drop";

/** A point in layer pixels. */
type Point = { x: number; y: number };

/** A right-angled cable run between two ends. */
type Run = { from: Point; to: Point; bend: number };

/** A card's edges and centre in layer pixels. */
type Box = { left: number; right: number; top: number; bottom: number; centre: number };

/** One cable between two cards, and how visible it is. */
type Cable = {
    /** The drawn cable. */
    path: SVGPathElement;
    /** The plugs at both ends. */
    ends: SVGPathElement;
    /** The dash of light that runs down the cable when the scene changes. */
    pulse: SVGPathElement;
    /** The visibility from 0 to 1. */
    alpha: number;
    /** The time the cable fades back in after the last change. */
    showsAt: number;
    /** How far the cable has settled onto the grid, from 0 on its cards to 1 in the holes. */
    settle: number;
};

/** Show the top two layers, locked today or freely rearranging with Destack, all draggable. */
export function Remix(properties: {
    isOpen: boolean;
    today: number;
    isLive: boolean;
    revealOf: (row: number) => number;
    onScene: (scene: number) => void;
}) {
    // hold the scene, the dragged card, and the elements to measure
    const [scene, setScene] = createSignal(0);
    const [drag, setDrag] = createSignal<{ id: string; x: number; y: number }>();
    let layer: HTMLDivElement | undefined;
    let wires: SVGGElement | undefined;
    const cards = new Map<string, HTMLDivElement>();
    const last = new Map<string, Placement>();

    // pick the scene on show: the locked stack today, the loop once open
    const shown = () =>
        properties.isOpen
            ? present(scenes[scene()], "scene")
            : present(todayScenes[properties.today], "scene");

    // place every card of the scene: cards leave and enter where they stand, row by row
    let wasLaidOpen = properties.isOpen;
    let change = 0;
    const layout = createMemo(() => {
        // pick the placements and note whether the stack just opened or closed
        const current = properties.isOpen
            ? present(scenePlacements[scene()], "scene")
            : present(todayPlacements[properties.today], "scene");
        const isToggle = properties.isOpen !== wasLaidOpen;
        wasLaidOpen = properties.isOpen;
        change += 1;

        // enter after the leaving cards are gone, or after the water drains or the ice reforms on a toggle
        const toggleDelay = properties.isOpen ? openDelay : closeDelay;
        const enterAt = Math.max(duration.short, isToggle ? toggleDelay : 0);

        // step each card toward its placement
        for (const id of ids) {
            const placement = current.get(id);
            const previous = last.get(id);

            // keep a card in its place, move it by fading out and in, or show it after the leaving cards
            if (placement) {
                const isShown = previous?.isShown === true;
                const step: Step =
                    previous === undefined || (isShown && isSamePlace(previous, placement))
                        ? "stay"
                        : isShown
                          ? "move"
                          : "enter";
                const start = step === "enter" ? enterAt : 0;
                last.set(id, { ...placement, step, delay: start + placement.row * stagger });
            }
            // hide a card that showed where it stands
            else if (previous?.isShown === true) {
                last.set(id, {
                    ...previous,
                    isShown: false,
                    step: "leave",
                    delay: previous.row * stagger,
                });
            }
            // rest a hidden card, first at the place where it next appears
            else {
                const place =
                    previous ?? (vendors.has(id) ? lockedPlacement(id) : upcoming(id, scene()));
                last.set(id, { ...place, isShown: false, step: "park", delay: 0 });
            }
        }

        return { change, places: new Map(last) };
    });

    // follow a dragged card with the pointer, then let it spring home
    const grab = (id: string, event: PointerEvent) => {
        // hold the pointer start and pick the card up
        const startX = event.clientX;
        const startY = event.clientY;
        event.preventDefault();
        setDrag({ id, x: 0, y: 0 });

        // follow the pointer until it lifts, then let go of the card and stop following it
        const drop = () => {
            setDrag(undefined);
            stops.forEach((stop) => stop());
        };
        const stops = [
            makeEventListener(window, "pointermove", (moving) =>
                setDrag({ id, x: moving.clientX - startX, y: moving.clientY - startY }),
            ),
            makeEventListener(window, "pointerup", drop),
            makeEventListener(window, "pointercancel", drop),
        ];
    };

    // wake the cables after the layout changes or the layer comes into view, and rest them while off screen
    const wake = { until: 0, isVisible: true, resume: () => {} };
    const resizes = makeResizeObserver(() => {
        wake.until = performance.now() + 300;
    });
    const sight = makeIntersectionObserver([], (entries) => {
        for (const entry of entries) {
            wake.isVisible = entry.isIntersecting;
            wake.until = performance.now() + 300;
            if (entry.isIntersecting) {
                wake.resume();
            }
        }
    });

    // trace the cables every frame once the cards are in the page
    onSettled(() => {
        // require the rendered layer and wires
        if (!layer || !wires) {
            throw new TypeError("the remix rendered without its layer and wires");
        }

        // hold the cables, the animation frame, the scene timing, and the motion state
        const isStill = isMotionReduced(layer);
        const cables = new Map<string, Cable>();
        let animationFrame: number | undefined;
        let nextScene: number | undefined;
        let wasOpen = properties.isOpen;
        let wasScene = scene();
        let wasToday = properties.today;
        let changedAt = -Infinity;
        let delay = 0;
        let isStirring = false;
        const narrowScreen = window.matchMedia("(max-width: 1099px)");

        // measure the boxes of the given cards within the layer, all before any cable is drawn
        const measure = (measured: Iterable<string>) => {
            // measure each card against the layer bounds
            const bounds = layer.getBoundingClientRect();
            const boxes = new Map<string, Box>();
            for (const id of measured) {
                const card = cards.get(id)?.firstElementChild?.firstElementChild;
                if (card) {
                    const rect = card.getBoundingClientRect();
                    boxes.set(id, {
                        left: rect.left - bounds.left,
                        right: rect.right - bounds.left,
                        top: rect.top - bounds.top,
                        bottom: rect.bottom - bounds.top,
                        centre: (rect.left + rect.right) / 2 - bounds.left,
                    });
                }
            }

            return { bounds, boxes };
        };

        // start a cable the first time it is needed
        const cable = (key: string, kind: Kind) => {
            let found = cables.get(key);
            if (!found) {
                const path = document.createElementNS("http://www.w3.org/2000/svg", "path");
                const ends = document.createElementNS("http://www.w3.org/2000/svg", "path");
                const pulse = document.createElementNS("http://www.w3.org/2000/svg", "path");
                path.setAttribute(
                    "class",
                    present(style.attrs(styles.cable, cableKinds[kind]).class, "class names"),
                );
                ends.setAttribute(
                    "class",
                    present(style.attrs(plugKinds[kind]).class, "class names"),
                );
                pulse.setAttribute(
                    "class",
                    present(style.attrs(styles.pulse).class, "class names"),
                );
                pulse.setAttribute("pathLength", "1");
                wires.append(path, pulse, ends);
                found = {
                    path,
                    ends,
                    pulse,
                    alpha: 0,
                    showsAt: 0,
                    settle: 0,
                };
                cables.set(key, found);
            }

            return found;
        };

        // fade a cable out at a change, and back in once the cards have settled
        const show = (found: Cable, fade: number, now: number) => {
            // hide the cable from the last change until its moment, then fade it in
            if (found.showsAt < changedAt) {
                found.showsAt = changedAt + delay;
            }
            found.alpha =
                now >= found.showsAt
                    ? Math.min(1, found.alpha + 0.06)
                    : Math.max(0, found.alpha - 0.12);
            const opacity = String(found.alpha * fade);
            found.path.style.opacity = opacity;
            found.ends.style.opacity = opacity;
            found.pulse.style.opacity = opacity;
            isStirring ||= found.alpha < 1;
        };

        // hang a locked cable between two cards as a taut curve
        const hang = (key: string, from: Point, to: Point, now: number) => {
            // draw the plugs and the curve
            const found = cable(key, "locked");
            show(found, 1, now);
            found.ends.setAttribute("d", `${plug(from)}${plug(to)}`);
            const half = (to.y - from.y) / 2;
            found.path.setAttribute(
                "d",
                `M${from.x} ${from.y}C${from.x} ${from.y + half} ${to.x} ${to.y - half} ${to.x} ${to.y}`,
            );
        };

        // run an open cable at right angles, following its cards while they move and easing into the holes once they rest
        const run = (
            key: string,
            kind: Kind,
            free: Run,
            grid: Run,
            isResting: boolean,
            isAcross: boolean,
            fade: number,
            now: number,
        ) => {
            // start the cable and fade it in
            const found = cable(key, kind);
            show(found, fade, now);

            // blend the free run toward the grid run as the cable settles
            const target = isResting ? 1 : 0;
            found.settle += (target - found.settle) * 0.14;
            if (Math.abs(target - found.settle) < 0.002) {
                found.settle = target;
            }
            isStirring ||= found.settle !== target;
            const settle = found.settle;
            const mix = (start: number, end: number) => start + (end - start) * settle;
            const from = { x: mix(free.from.x, grid.from.x), y: mix(free.from.y, grid.from.y) };
            const to = { x: mix(free.to.x, grid.to.x), y: mix(free.to.y, grid.to.y) };
            const bend = mix(free.bend, grid.bend);

            // draw the plugs, the right-angled path, and its pulse along the same line
            found.ends.setAttribute("d", `${plug(from)}${plug(to)}`);
            const line = isAcross
                ? `M${from.x} ${from.y}H${bend}V${to.y}H${to.x}`
                : `M${from.x} ${from.y}V${bend}H${to.x}V${to.y}`;
            found.path.setAttribute("d", line);
            found.pulse.setAttribute("d", line);
        };

        // trace every cable from the cards' measured positions
        const trace = (now: number) => {
            // measure the cards the scene links and reset the motion flag
            const current = shown();
            const isOpen = properties.isOpen;
            const needed = new Set([
                ...current.links.flat(),
                ...current.lower.map((card) => card.id),
            ]);
            const { bounds, boxes } = measure(needed);
            const kept = new Set<string>();
            const places = layout().places;
            const cell = bounds.width / boardCells;
            const cellRow = bounds.height / layerRows;
            isStirring = false;

            // snap a position across to the middle of the hole column it falls in
            const hole = (x: number) => (Math.floor(x / cell) + 0.5) * cell;

            // tell whether a card rests in its place, neither held nor travelling
            const rests = (id: string, box: Box) => {
                const place = places.get(id);
                const top = cellRow * (place?.row === 0 ? 2 : 11);

                return (
                    place !== undefined &&
                    drag()?.id !== id &&
                    Math.abs(box.centre - centre(place, cell)) < 1.5 &&
                    Math.abs(box.top - top) < 1.5
                );
            };

            // run every cable across a gap along one tidy bus: its middle, or above the row captions on narrow screens
            const share = narrowScreen.matches ? captionedBus : 0.5;
            const middle = (top: number, bottom: number) => top + (bottom - top) * share;

            // link the upper cards to the lower cards they work with: once open, each person runs straight down, across on their own row, and down into the app
            const kind = isOpen ? "link" : "locked";
            current.links.forEach(([upper, lower]) => {
                // find both cards of the link, skipping links whose cards are missing
                const from = boxes.get(upper);
                const to = boxes.get(lower);
                if (!from || !to) {
                    return;
                }
                const key = `${kind}:${upper}:${lower}`;
                kept.add(key);
                const start = { x: from.centre, y: from.bottom };
                const end = { x: to.centre, y: to.top };
                if (!isOpen) {
                    hang(key, start, end, now);
                    return;
                }
                const bend = middle(from.bottom, to.top);
                run(
                    key,
                    kind,
                    { from: start, to: end, bend },
                    {
                        from: { x: hole(from.centre), y: from.bottom },
                        to: {
                            x: hole(to.centre),
                            y: to.top,
                        },
                        bend,
                    },
                    rests(upper, from) && rests(lower, to),
                    false,
                    1,
                    now,
                );
            });

            // once open, chain the lower cards along a hole row, drop each into the service it calls, and link each service to its store
            if (isOpen) {
                const reveal = properties.revealOf(2);
                const stored = properties.revealOf(3);
                const item = (label: string) => {
                    const found = layer.parentElement
                        ?.querySelector(`[data-item="${label}"]`)
                        ?.getBoundingClientRect();

                    return found
                        ? {
                              left: found.left - bounds.left,
                              width: found.width,
                              centre: (found.left + found.right) / 2 - bounds.left,
                              top: found.top - bounds.top,
                              bottom: found.bottom - bounds.top,
                          }
                        : undefined;
                };

                // list the calls into the services and the links down to the stores
                const calls = current.lower.flatMap((card) =>
                    [...new Set(usesOf(card.id).map(([service]) => service))].map((service) => ({
                        id: card.id,
                        service,
                    })),
                );
                const links = [
                    ...new Map(
                        current.lower
                            .flatMap((card) => usesOf(card.id))
                            .map(([service, store]) => [`${service}:${store}`, { service, store }]),
                    ).values(),
                ];
                const lower = current.lower.flatMap((card) => {
                    const found = boxes.get(card.id);
                    return found
                        ? [{ id: card.id, isResting: rests(card.id, found), ...found }]
                        : [];
                });
                lower.forEach((card, index) => {
                    // chain this card to the next one along the hole row
                    const next = lower[index + 1];
                    if (next) {
                        const key = `chain:${card.id}:${next.id}`;
                        kept.add(key);
                        const start = { x: card.right, y: (card.top + card.bottom) / 2 };
                        const end = { x: next.left, y: (next.top + next.bottom) / 2 };
                        const y = cellRow * 13.5;
                        run(
                            key,
                            "chain",
                            { from: start, to: end, bend: (start.x + end.x) / 2 },
                            {
                                from: { x: start.x, y },
                                to: { x: end.x, y },
                                bend: (start.x + end.x) / 2,
                            },
                            card.isResting && next.isResting,
                            true,
                            reveal,
                            now,
                        );
                    }

                    // drop this card into every service it calls, spread along both edges, turning lower for later services
                    const own = calls.filter((call) => call.id === card.id);
                    own.forEach((call) => {
                        // find the service cell, or skip a service its band does not show
                        const service = item(call.service);
                        if (!service) {
                            return;
                        }
                        const key = `drop:${card.id}:${call.service}`;
                        kept.add(key);
                        const place = places.get(card.id);
                        const anchor = {
                            x: hole(service.centre),
                            y: service.top,
                        };
                        const x = hole(place ? centre(place, cell) : card.centre);
                        const bend = middle(card.bottom, anchor.y);
                        run(
                            key,
                            "drop",
                            { from: { x: card.centre, y: card.bottom }, to: anchor, bend },
                            { from: { x, y: card.bottom }, to: anchor, bend },
                            card.isResting,
                            false,
                            reveal,
                            now,
                        );
                    });
                });

                // link each service the scene calls down to each store it keeps the scene's state in
                for (const link of links) {
                    const service = item(link.service);
                    const store = item(link.store);
                    if (!service || !store) {
                        continue;
                    }
                    const key = `store:${link.service}:${link.store}`;
                    kept.add(key);
                    const from = { x: hole(service.centre), y: service.bottom };
                    const to = {
                        x: hole(store.centre),
                        y: store.top,
                    };
                    const bend = middle(service.bottom, store.top);
                    const path = { from, to, bend };
                    run(key, "drop", path, path, true, false, stored, now);
                }

                // feed every host from the build
                const build = item("Build");
                if (build) {
                    quarterCentres.forEach((column, index) => {
                        // run down from the build, across the bus, and down into this host
                        const key = `host:${index}`;
                        kept.add(key);
                        const path = {
                            from: { x: build.centre, y: build.bottom },
                            to: { x: hole(cell * column), y: cellRow * hostTop },
                            bend: cellRow * hostBus,
                        };
                        run(key, "drop", path, path, true, false, properties.revealOf(5), now);
                    });
                }
            }

            // fade out cables the scene no longer uses, and remove them once gone
            for (const [key, found] of cables) {
                if (kept.has(key)) {
                    continue;
                }
                isStirring = true;
                found.alpha = Math.max(0, found.alpha - 0.12);
                found.path.style.opacity = String(found.alpha);
                found.ends.style.opacity = String(found.alpha);
                found.pulse.style.opacity = String(found.alpha);
                if (found.alpha === 0) {
                    found.path.remove();
                    found.ends.remove();
                    found.pulse.remove();
                    cables.delete(key);
                }
            }
        };

        // time the cables to the choreography, and step the scenes once the water has drained
        const loop = (now: number) => {
            // restart the cable timing when the stack opens, closes, or changes scene
            if (properties.isOpen !== wasOpen) {
                changedAt = now;
                delay = (properties.isOpen ? openDelay : closeDelay) + duration.medium + stagger;
                wasOpen = properties.isOpen;
            } else if (scene() !== wasScene || properties.today !== wasToday) {
                changedAt = now;
                delay = duration.short + duration.medium + stagger;

                // run a dash of light down every open cable once the new scene's cables are in
                if (properties.isOpen && !isStill) {
                    for (const found of cables.values()) {
                        found.pulse.animate(
                            [{ strokeDashoffset: 1 }, { strokeDashoffset: -pulseLength }],
                            {
                                delay: delay + pulseDelay,
                                duration: pulseTime,
                                easing: "ease-in-out",
                            },
                        );
                    }
                }
            }
            wasScene = scene();
            wasToday = properties.today;

            // hold the open scene on the locked one while locked, and step it while open and live
            if (!properties.isOpen) {
                nextScene = undefined;
                if (scene() !== properties.today * 2) {
                    setScene(properties.today * 2);
                    properties.onScene(properties.today * 2);
                }
            } else if (properties.isLive && !isStill) {
                nextScene ??= now + firstSceneTime;
                if (drag()) {
                    nextScene = Math.max(nextScene, now + 1200);
                } else if (now >= nextScene) {
                    const next = (scene() + 1) % scenes.length;
                    setScene(next);
                    properties.onScene(next);
                    nextScene = now + sceneTime;
                }
            }

            // trace only while something moves: travelling cards, a drag, or a settling cable
            const isBusy =
                drag() !== undefined || now - changedAt < 1500 || now < wake.until || isStirring;
            if (wake.isVisible && isBusy) {
                trace(now);
            }
            if (drag()) {
                wake.until = now + 1500;
            }

            // keep looping while on screen, and rest until the layer comes back into view
            animationFrame = wake.isVisible ? requestAnimationFrame(loop) : undefined;
        };
        animationFrame = requestAnimationFrame(loop);
        wake.resume = () => {
            animationFrame ??= requestAnimationFrame(loop);
        };

        // retrace after the layout changes or the layer comes into view, and rest while off screen
        resizes.observe(layer);
        sight.add(layer);

        return () => {
            if (animationFrame !== undefined) {
                cancelAnimationFrame(animationFrame);
            }
        };
    });

    return (
        <div
            ref={layer}
            style={{
                height: `calc(${frame.cellRow} * ${layerRows})`,
                width: `calc(${frame.cell} * ${boardCells})`,
            }}
            {...style.attrs(styles.layer)}
        >
            <svg aria-hidden="true" {...style.attrs(styles.lines)}>
                <g ref={wires} />
            </svg>

            {/* place each card on its row; cards fade between places and can be dragged */}
            <For each={ids}>
                {(id) => {
                    // read the card's placement, drag offset, and kind
                    const placement = () => {
                        const found = layout().places.get(id);
                        if (!found) {
                            throw new TypeError(`card ${id} has no placement`);
                        }

                        return found;
                    };
                    const held = () => (drag()?.id === id ? drag() : undefined);
                    const slot = slots.get(id);
                    const isVendor = slot !== undefined;

                    return (
                        <div
                            ref={(element) => cards.set(id, element)}
                            data-card
                            onPointerDown={(event) => grab(id, event)}
                            {...style.attributes(
                                [styles.card, held() !== undefined && styles.held],
                                {
                                    ...span(placement()),
                                    ...motionOf(placement(), layout().change, held()),
                                },
                            )}
                        >
                            <div class={style.attrs(styles.fill).class}>
                                <Card
                                    entity={entityOf(id)}
                                    kind={isVendor ? "vendor" : "plain"}
                                    xstyle={styles.fill}
                                />
                            </div>
                        </div>
                    );
                }}
            </For>
        </div>
    );
}

/** How far down a gap the cable bus runs on narrow screens, as a share of the gap. */
const captionedBus = 0.3;
/** The length of the dash of light that runs down a cable, as a share of the cable. */
const pulseLength = 0.12;
/** The milliseconds the dash of light takes to run down a cable. */
const pulseTime = 400;
/** The milliseconds after a cable appears before its dash of light starts. */
const pulseDelay = 90;
/** A card fading out over a short change and back in at its new place over a medium one. */
const moveFadeEven = style.keyframes({
    "0%": { animationTimingFunction: motion.easingStandard, opacity: 1, translate: "0 0" },
    "33%": {
        animationTimingFunction: motion.easingEmphasised,
        opacity: 0,
        translate: `0 ${movement.travel}`,
    },
    "100%": { opacity: 1, translate: "0 0" },
});

/** The same fade under a second name, so each change of scene runs it afresh. */
const moveFadeOdd = style.keyframes({
    "0%": { animationTimingFunction: motion.easingStandard, opacity: 1, translate: "0 0" },
    "34%": {
        animationTimingFunction: motion.easingEmphasised,
        opacity: 0,
        translate: `0 ${movement.travel}`,
    },
    "100%": { opacity: 1, translate: "0 0" },
});

/** Return a card's place, as offsets a transform moves it by, and its width within the layer. */
function span(place: Placement) {
    return {
        "--x": `calc(${frame.cell} * ${place.left})`,
        "--y":
            place.row === 0
                ? `calc(${frame.cellRow} * ${layerRows} / 6 + ${userDrop}px)`
                : `calc(${frame.cellRow} * ${layerRows} / 2)`,
        width: `calc(${frame.cell} * ${place.width})`,
    };
}

/** Return the centre of a card's span in pixels, for a given cell width. */
function centre(place: Placement, cell: number) {
    return (place.left + place.width / 2) * cell;
}

/** Return a card's inline motion for a change and any drag. */
function motionOf(
    place: Placement,
    change: number,
    held: { x: number; y: number } | undefined,
): Record<string, string> {
    // move the card to its place and by any drag in one transform, so the compositor carries it
    const offset = held ? `translate(${held.x}px, ${held.y}px)` : "translate(0px, 0px)";
    const transform = `translate(var(--x), var(--y)) ${offset} translateY(-50%)`;

    // jump the box while the card is faded out, follow a drag at once, and ease home after one
    const jumpAt = place.step === "move" ? place.delay + duration.short : 0;
    const box = held
        ? "transform 0ms, width 0ms"
        : place.step === "stay"
          ? `transform ${motion.durationMedium} ${motion.easingStandard}, width 0ms`
          : `transform 0ms linear ${jumpAt}ms, width 0ms linear ${jumpAt}ms`;

    // show a card, rising into its place after the leaving cards are gone
    if (place.step === "enter") {
        const fade = `${motion.durationMedium} ${motion.easingEmphasised} ${place.delay}ms`;
        return {
            opacity: "1",
            transform,
            translate: "0 0",
            transition: `opacity ${fade}, translate ${fade}, ${box}`,
        };
    }
    // hide a card, dropping a little as it fades
    else if (place.step === "leave") {
        const fade = `${motion.durationShort} ${motion.easingStandard} ${place.delay}ms`;
        return {
            opacity: "0",
            "pointer-events": "none",
            transform,
            translate: `0 ${movement.travel}`,
            transition: `opacity ${fade}, translate ${fade}, ${box}`,
        };
    }
    // move a card by fading it out, jumping its box, and fading it back in
    else if (place.step === "move") {
        const fades = change % 2 === 0 ? moveFadeEven : moveFadeOdd;
        return {
            animation: `${fades} calc(${motion.durationShort} + ${motion.durationMedium}) ${place.delay}ms both`,
            opacity: "1",
            transform,
            translate: "0 0",
            transition: box,
        };
    }
    // keep a shown card, or rest a hidden one
    else {
        return {
            opacity: place.isShown ? "1" : "0",
            transform,
            translate: place.isShown ? "0 0" : `0 ${movement.travel}`,
            transition: box,
            ...(place.isShown ? {} : { "pointer-events": "none" }),
        };
    }
}

/** Return a small square plug at a cable end, as an SVG path. */
function plug(point: Point) {
    return `M${point.x - 2} ${point.y - 2}h4v4h-4Z`;
}

/** Place every card a scene shows in whole board cells. */
function arrange(scene: Scene): Map<string, Placement> {
    // share the upper row evenly between its cards
    const placed = new Map<string, Placement>();
    const gap = 3;
    const width =
        (boardCells - boardInset * 2 - gap * (scene.upper.length - 1)) / scene.upper.length;
    scene.upper.forEach((id, index) => {
        const left = boardInset + index * (width + gap);
        placed.set(id, {
            row: 0,
            left,
            width,
            isShown: true,
            step: "stay",
            delay: 0,
        });
    });

    // lay the lower row's cards across the columns they span
    let column = 0;
    for (const card of scene.lower) {
        const left = columnLefts[column];
        const last = columnLefts[column + card.span - 1];
        if (left === undefined || last === undefined) {
            throw new TypeError(`card ${card.id} spans past the board columns`);
        }
        placed.set(card.id, {
            row: 1,
            left,
            width: last + columnWidth - left,
            isShown: true,
            step: "stay",
            delay: 0,
        });
        column += card.span;
    }

    return placed;
}

/** Return a silo's place on its iceberg, from the first scene of the locked stack that shows it. */
function lockedPlacement(id: string): Placement {
    // find the first locked scene with the silo
    const placement = todayPlacements.find((placements) => placements.has(id))?.get(id);
    if (!placement) {
        throw new Error(`silo ${id} never rides an iceberg`);
    }

    return placement;
}

/** Return whether two placements take the same place: the same row, edge, and width. */
function isSamePlace(first: Placement, second: Placement) {
    return (
        first.row === second.row &&
        Math.abs(first.left - second.left) < 0.5 &&
        Math.abs(first.width - second.width) < 0.5
    );
}

/** Return where a card next appears after an open scene, or in the locked stack. */
function upcoming(id: string, from: number): Placement {
    // search the scenes that follow in order
    for (let step = 1; step <= scenes.length; step++) {
        const placement = present(scenePlacements[(from + step) % scenes.length], "scene").get(id);
        if (placement) {
            return placement;
        }
    }

    // find the card in the locked stack
    const today = present(todayPlacements[0], "scene").get(id);
    if (today) {
        return today;
    }

    // fail when no scene shows the card
    throw new Error(`card ${id} never appears`);
}

/** Build a locked scene. */
function locked(
    upper: readonly string[],
    lower: readonly [string, string, string],
    targets: readonly string[],
): Scene {
    return {
        upper,
        lower: lower.map((id) => ({ id, span: 1 })),
        links: linksOf(upper, targets),
    };
}

/** Return the remix that joins two apps, or throw when none does. */
function remixOf(first: string, second: string): string {
    const found = Object.entries(remixes).find(
        ([, parts]) => parts.includes(first) && parts.includes(second),
    );
    if (!found) {
        throw new TypeError(`no remix joins ${first} and ${second}`);
    }

    return found[0];
}

/** Pair each person with the target at the same index, or throw when a person has none. */
function linksOf(upper: readonly string[], targets: readonly string[]): [string, string][] {
    return upper.map((id, index): [string, string] => {
        const target = targets[index];
        if (target === undefined) {
            throw new TypeError(`person ${id} has no target`);
        }

        return [id, target];
    });
}

/** Return the card with an id. */
function entityOf(id: string): Entity {
    const entity = entities[id];
    if (!entity) {
        throw new TypeError(`unknown card ${id}`);
    }

    return entity;
}

/** Return the services an open app calls, each with its store. */
export function usesOf(id: string): readonly (readonly [string, string])[] {
    const uses = appUses[id];
    if (!uses) {
        throw new TypeError(`app ${id} calls no services`);
    }

    return uses;
}

/** Add each remix's services and stores, the union of the apps it joins. */
function withRemixes(
    uses: Record<string, readonly (readonly [string, string])[]>,
): Record<string, readonly (readonly [string, string])[]> {
    // join the pairs of each remix's apps, once each
    const joined = Object.entries(remixes).map(
        ([remix, parts]): [string, readonly (readonly [string, string])[]] => {
            const pairs = parts.flatMap((part) => {
                // reject a remix of an app without services
                const used = uses[part];
                if (!used) {
                    throw new TypeError(`remix ${remix} joins unknown app ${part}`);
                }

                return used;
            });
            const unique = [...new Map(pairs.map((pair) => [pair.join(":"), pair])).values()];

            return [remix, unique];
        },
    );

    return { ...uses, ...Object.fromEntries(joined) };
}

/** The cable strokes for each cable kind. */
const cableKinds = style.create({
    locked: { stroke: color.mutedForeground },
    link: { stroke: `color-mix(in srgb, ${color.foreground} 22%, transparent)` },
    chain: { stroke: `color-mix(in srgb, ${color.foreground} 22%, transparent)` },
    drop: { stroke: `color-mix(in srgb, ${color.foreground} 22%, transparent)` },
});

/** The plug fills for each cable kind. */
const plugKinds = style.create({
    locked: { fill: color.mutedForeground },
    link: { fill: `color-mix(in srgb, ${color.foreground} 30%, transparent)` },
    chain: { fill: `color-mix(in srgb, ${color.foreground} 30%, transparent)` },
    drop: { fill: `color-mix(in srgb, ${color.foreground} 30%, transparent)` },
});

/** The remix layer styles. */
const styles = style.create({
    layer: {
        clipPath: "inset(-100vh 0)",
        left: 0,
        pointerEvents: "none",
        position: "absolute",
        top: 0,
        zIndex: 1,
    },
    lines: {
        height: "100%",
        inset: 0,
        overflow: "visible",
        pointerEvents: "none",
        position: "absolute",
        width: "100%",
    },
    cable: {
        fill: "none",
        strokeWidth: 1.25,
    },
    pulse: {
        fill: "none",
        stroke: palette.cream,
        strokeDasharray: `${pulseLength} 2`,
        strokeDashoffset: 1,
        strokeLinecap: "round",
        strokeWidth: 2.5,
    },
    card: {
        cursor: "grab",
        display: "flex",
        justifyContent: "center",
        left: 0,
        pointerEvents: "auto",
        position: "absolute",
        top: 0,
        touchAction: "none",
        userSelect: "none",
    },
    held: {
        cursor: "grabbing",
        zIndex: 2,
    },
    fill: {
        width: "100%",
    },
});
