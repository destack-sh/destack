import { frame } from "../../layout/frame.stylex";
import { color } from "@destack/theme/tokens.stylex";
import { present } from "@destack/schema";
import * as style from "@destack/style";
import { createMemo, createSignal, For, onSettled } from "@destack/view";

import { cardVariables } from "./card.stylex";
import {
    boardCells,
    boardInset,
    columnLefts,
    columnWidth,
    quarterCentres,
    rowCells,
} from "./board";
import { Card, type Entity } from "./card";
import { palette } from "../../palette.stylex";

/** The milliseconds each open scene holds before the next one begins. */
const sceneTime = 5200;
/** The milliseconds from the water fully draining to the first scene change. */
const firstSceneTime = 1200;
/** The milliseconds between neighbouring cards turning over, left to right. */
const flipStagger = 60;
/** The milliseconds after the stack opens before its apps turn into view, once the silos have sunk. */
const openFlipAt = 500;
/** The milliseconds a card's ink takes to fade out before it hands over its box, or to fade in after it takes one. */
const inkFade = 320;
/** The milliseconds a card takes to turn over and show the card replacing it. */
const flipTime = 600;
/** The milliseconds cards take to travel between places. */
const moveTime = 600;

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

    // silos, at the list price of their recommended plan, billed yearly
    notion: {
        label: "Notion",
        icon: "notion",
        role: "$20/seat/mo + credits",
        tint: "#ffffff",
        glyph: "#191919",
    },
    airtable: {
        label: "Airtable",
        icon: "airtable",
        role: "$45/seat/mo + credits",
        tint: "#18bfff",
    },
    typeform: { label: "Typeform", icon: "typeform", role: "$91/mo + credits", tint: "#262627" },
    dropbox: { label: "Dropbox", icon: "dropbox", role: "$18/seat/mo", tint: "#0061ff" },
    figma: { label: "Figma", icon: "figma", role: "$55/seat/mo + credits", tint: "#f24e1e" },
    slack: { label: "Slack", icon: "slack", role: "$15/seat/mo", tint: "#4a154b" },
    granola: {
        label: "Granola",
        icon: "file",
        role: "$14/seat/mo",
        tint: "#9cbb3c",
        logos: ["granola.png"],
    },
    gdocs: { label: "Google Docs", icon: "googledocs", role: "$14/seat/mo", tint: "#4285f4" },
    calendly: { label: "Calendly", icon: "calendly", role: "$16/seat/mo", tint: "#006bff" },
    linear: { label: "Linear", icon: "linear", role: "$16/seat/mo + credits", tint: "#5e6ad2" },
    github: { label: "GitHub", icon: "github", role: "$21/seat/mo + credits", tint: "#24292f" },
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
        tint: "#b8862b",
    },
    ownPortal: { label: "Client portal", icon: "pages", role: "Built on Destack", tint: "#3d6fb0" },
    ownPipeline: { label: "Hiring board", icon: "user", role: "Built on Destack", tint: "#5b7f2e" },
    ownBudget: { label: "Family budget", icon: "table", role: "Built on Destack", tint: "#a0485f" },

    // apps
    pages: { label: "Pages", icon: "pages", role: "App", tint: "#3d6fb0" },
    notes: { label: "Notes", icon: "file", role: "App", tint: "#b8862b" },
    chat: { label: "Chat", icon: "chat", role: "App", tint: "#4f8a5b" },
    tasks: { label: "Tasks", icon: "tasks", role: "App", tint: "#c64a17" },
    source: { label: "Source", icon: "source", role: "App", tint: "#24292f" },
    forms: { label: "Forms", icon: "template", role: "App", tint: "#6b5ca5" },
    calendar: { label: "Calendar", icon: "calendar", role: "App", tint: "#c64a17" },
    sheets: { label: "Sheets", icon: "table", role: "App", tint: "#4f8a5b" },
    mail: { label: "Mail", icon: "mail", role: "App", tint: "#3d6fb0" },
    files: { label: "Files", icon: "bucket", role: "App", tint: "#2f7d8c" },
    canvas: { label: "Canvas", icon: "vector", role: "App", tint: "#c64a17" },

    // remixes, each named after the apps it joins
    room: { label: "Launch room", icon: "chat", role: "Pages + Chat", tint: "#4f8a5b" },
    roadmap: { label: "Roadmap", icon: "tasks", role: "Pages + Tasks", tint: "#c64a17" },
    issues: { label: "Issues", icon: "notify", role: "Source + Tasks", tint: "#24292f" },
    review: { label: "Design review", icon: "vector", role: "Canvas + Notes", tint: "#c64a17" },
    research: { label: "User research", icon: "template", role: "Forms + Notes", tint: "#6b5ca5" },
    interviews: {
        label: "Interviews",
        icon: "calendar",
        role: "Forms + Calendar",
        tint: "#a0485f",
    },
    bookings: { label: "Bookings", icon: "calendar", role: "Sheets + Calendar", tint: "#3f6f73" },
    report: { label: "Report", icon: "table", role: "Sheets + Pages", tint: "#4f8a5b" },
    handbook: { label: "Handbook", icon: "pages", role: "Files + Pages", tint: "#3d6fb0" },
    hub: { label: "Family hub", icon: "storage", role: "Files + Chat", tint: "#2f7d8c" },
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
/** The milliseconds a silo takes to rise after the one it replaces starts to sink. */
const swapDelay = 400;
/** The milliseconds a silo waits to rise again after the water returns. */
const landingDelay = 900;

/** One arrangement of the top two layers. */
type Scene = {
    /** The cards on the upper row, left to right. */
    upper: readonly string[];
    /** The cards on the lower row, left to right, and how many of the three columns each spans. */
    lower: readonly { id: string; span: number }[];
    /** Which upper card works with which lower card. */
    links: readonly [string, string][];
};

/** The locked stack today, one change per step: a silo swaps on its iceberg or someone new signs in, and the logins reshuffle. */
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

/** The open loop: each locked scene with every silo replaced in its place, then its two replaced apps joined into a remix. */
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

/** The services each open app calls, each with the store that service keeps the app's state in, by label; remixes use their apps' own. */
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

/** The source step each open scene shows at work, by its label: installing a set of apps, then building its remix. */
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

/** How far the user row sits below the middle of its figure row, in CSS pixels, to leave room for the switch on the seam above it. */
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

/** How a card moves into its placement. */
type Step = "stay" | "enter" | "leave" | "park" | "fuse" | "flip";

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
    /** How the card moves there: stays or travels on the board, enters or leaves across an edge, fuses into the cards replacing it, or parks out of sight. */
    step: Step;
    /** The milliseconds the card waits before it moves. */
    delay: number;
    /** Whether the card shows or hides at the end of its move rather than at its start. */
    isLate: boolean;
    /** How far the card is turned about its horizontal axis, in degrees: flipped away while it swaps places with another card in its slot. */
    turn?: number;
};

/** How a card's ink fades, as CSS values for the card's variables. */
type Ink = {
    /** The ink's opacity, one when shown. */
    opacity: string;
    /** The fade's duration. */
    time: string;
    /** The delay before the fade. */
    delay: string;
};

/** One kind of cable, which sets its look. */
type Kind = "locked" | "link" | "chain" | "drop";

/** A point in layer pixels. */
type Point = { x: number; y: number };

/**
 * A right-angled cable run between two ends.
 *
 * The bend is where its middle segment crosses, down then across then down, or across then down then across.
 */
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
    /** The time the cable starts to fade in. */
    showsAt: number;
    /** How far the cable has settled onto the grid, from 0 following its cards to 1 resting in the holes. */
    settle: number;
};

/** Show the top two layers, locked today or freely rearranging with Destack, every card draggable. */
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

    // place every card of the scene: cards enter and leave across the nearest board edge, vendor apps sink in place
    let wasLaidOpen = properties.isOpen;
    const layout = createMemo(() => {
        // pick the placements and note whether the stack just opened or closed
        const current = properties.isOpen
            ? present(scenePlacements[scene()], "scene")
            : present(todayPlacements[properties.today], "scene");
        const isToggle = properties.isOpen !== wasLaidOpen;
        wasLaidOpen = properties.isOpen;

        // step each card toward its placement, collecting the cards that enter and leave across an edge, and those that morph
        const before = new Map(last);
        const entering: Placement[] = [];
        const leaving: Placement[] = [];
        const morphAt = isToggle ? 300 : 0;
        for (const id of ids) {
            const placement = current.get(id);
            const previous = last.get(id);
            const isVendor = vendors.has(id);

            // keep or move a card that shows, growing it out of the cards it replaces, and taking over only at the end of a merge
            if (placement) {
                const isEntering = previous !== undefined && !previous.isShown && !isVendor;
                const isGrowing = isEntering && isOnBoard(previous);
                const isFlipping = isGrowing && previous.turn !== undefined;
                const next: Placement = {
                    ...placement,
                    step: isFlipping ? "flip" : isEntering ? "enter" : "stay",
                    delay: isVendor
                        ? vendorDelay(previous, isToggle)
                        : isFlipping
                          ? (isToggle ? openFlipAt : 0) + flipStagger * slotOf(placement)
                          : isGrowing
                            ? morphAt
                            : 0,
                    isLate: isGrowing && sourcesOf(placement, before, current) > 1,
                };
                last.set(id, next);
                if (isEntering && !isGrowing) {
                    entering.push(next);
                }
            }
            // sink a vendor app where it stands
            else if (isVendor) {
                last.set(id, {
                    ...(previous ?? lockedPlacement(id)),
                    isShown: false,
                    step: "stay",
                    delay: 0,
                    isLate: false,
                });
            }
            // fuse a card that showed into the cards replacing it, or send it off across its nearest edge
            else if (previous?.isShown === true) {
                const successors = [...current]
                    .filter(
                        ([other, place]) =>
                            before.get(other)?.isShown !== true && overlaps(place, previous),
                    )
                    .map(([, place]) => place);
                // flap every open card away on its hinge as the stack closes, before the water comes back
                if (isToggle && !properties.isOpen) {
                    last.set(id, {
                        ...previous,
                        isShown: false,
                        step: "flip",
                        delay: flipStagger * slotOf(previous) + previous.row * flipStagger * 2,
                        isLate: false,
                        turn: 90,
                    });
                    continue;
                }

                // turn a card over to show the one card that takes its slot
                const [successor] = successors;
                if (
                    successors.length === 1 &&
                    successor !== undefined &&
                    sourcesOf(successor, before, current) === 1 &&
                    sameSlot(successor, previous)
                ) {
                    last.set(id, {
                        ...previous,
                        isShown: false,
                        step: "flip",
                        delay: flipStagger * slotOf(previous),
                        isLate: false,
                        turn: 90,
                    });
                } else if (successor !== undefined) {
                    const isMerging =
                        successors.length === 1 && sourcesOf(successor, before, current) > 1;
                    last.set(id, {
                        ...cover(successors),
                        step: "fuse",
                        delay: morphAt,
                        isLate: isMerging,
                    });
                } else {
                    const next: Placement = { ...beyond(previous), step: "leave", isLate: true };
                    last.set(id, next);
                    leaving.push(next);
                }
            }
            // leave a card that is turning away alone until it is gone
            else if (previous?.step === "flip" && previous.turn !== undefined) {
                continue;
            }
            // park a hidden card on the cards it replaces when it next appears, or beyond the edge nearest where it appears
            else {
                const next = upcoming(id, scene());
                const following = properties.isOpen
                    ? present(scenePlacements[(scene() + 1) % scenes.length], "scene")
                    : present(scenePlacements[properties.today * 2], "scene");
                const origins = following.has(id)
                    ? [...current]
                          .filter(
                              ([other, place]) => !following.has(other) && overlaps(place, next),
                          )
                          .map(([, place]) => place)
                    : [];
                const [origin] = origins;
                const isSwapping =
                    origins.length === 1 && origin !== undefined && sameSlot(origin, next);
                last.set(id, {
                    ...(isSwapping ? next : origins.length ? cover(origins) : beyond(next)),
                    isShown: false,
                    step: "park",
                    isLate: false,
                    ...(isSwapping && { turn: -90 }),
                });
            }
        }

        // start entering cards after the leaving ones clear, farthest from its edge first; send leaving cards nearest first
        const enterAt = isToggle ? 300 : moveTime * 0.45;
        const leaveAt = isToggle ? 400 : 0;
        stagger(entering, enterAt, (place) => -edgeDistance(place));
        stagger(leaving, leaveAt, edgeDistance);

        return new Map(last);
    });

    // follow a dragged card with the pointer, then let it spring home
    const grab = (id: string, event: PointerEvent) => {
        // hold the pointer start and pick the card up
        const startX = event.clientX;
        const startY = event.clientY;
        event.preventDefault();
        setDrag({ id, x: 0, y: 0 });

        // follow the pointer until it lifts, then let go of the card
        const move = (moving: PointerEvent) =>
            setDrag({ id, x: moving.clientX - startX, y: moving.clientY - startY });
        const drop = () => {
            // let go of the card and stop following the pointer
            setDrag(undefined);
            window.removeEventListener("pointermove", move);
            window.removeEventListener("pointerup", drop);
            window.removeEventListener("pointercancel", drop);
        };
        window.addEventListener("pointermove", move);
        window.addEventListener("pointerup", drop);
        window.addEventListener("pointercancel", drop);
    };

    // trace the cables every frame once the cards are in the page
    onSettled(() => {
        // require the rendered layer and wires
        if (!layer || !wires) {
            throw new TypeError("the remix rendered without its layer and wires");
        }

        // hold the cables, the animation frame, the scene timing, and the motion state
        const isStill = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
        const cables = new Map<string, Cable>();
        let animationFrame: number | undefined;
        let nextScene: number | undefined;
        let wasOpen = properties.isOpen;
        let wasScene = scene();
        let wasToday = properties.today;
        let changedAt = -Infinity;
        let delay = 0;
        let wakeUntil = 0;
        let isStirring = false;
        let isVisible = true;
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

        // fade a cable in once its moment in the choreography comes
        const show = (found: Cable, fade: number, now: number) => {
            // fade the cable in after the delay since the last change
            if (found.alpha === 0 && found.showsAt < changedAt) {
                found.showsAt = changedAt + delay;
            }
            if (now >= found.showsAt) {
                found.alpha = Math.min(1, found.alpha + 0.06);
            }
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
            const places = layout();
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

            // link the upper cards to the lower cards they work with: once open, each person runs straight down, across on their own lane, and down into the app
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
                delay = properties.isOpen ? 500 : 800;
                wasOpen = properties.isOpen;
            } else if (scene() !== wasScene || properties.today !== wasToday) {
                changedAt = now;
                delay = moveTime * 0.8;

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
                drag() !== undefined || now - changedAt < 1500 || now < wakeUntil || isStirring;
            if (isVisible && isBusy) {
                trace(now);
            }
            if (drag()) {
                wakeUntil = now + 1500;
            }
            animationFrame = requestAnimationFrame(loop);
        };
        animationFrame = requestAnimationFrame(loop);

        // retrace after the layout changes or the layer comes into view, and rest while off screen
        const sizes = new ResizeObserver(() => {
            wakeUntil = performance.now() + 300;
        });
        sizes.observe(layer);
        const sight = new IntersectionObserver((entries) => {
            for (const entry of entries) {
                isVisible = entry.isIntersecting;
                wakeUntil = performance.now() + 300;
            }
        });
        sight.observe(layer);

        return () => {
            sizes.disconnect();
            sight.disconnect();
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

            {/* place each card on its row; cards travel between rows and can be dragged */}
            <For each={ids}>
                {(id) => {
                    // read the card's placement, drag offset, and kind
                    const placement = () => {
                        const found = layout().get(id);
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
                                [
                                    styles.card,
                                    held() !== undefined && styles.held,
                                    styles.ink(inkOf(isVendor, placement())),
                                ],
                                {
                                    ...span(placement()),
                                    top:
                                        placement().row === 0
                                            ? `calc(100% / 6 + ${userDrop}px)`
                                            : "50%",
                                    ...follow(motion(isVendor, placement()), held()),
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

/** How far down a gap the cable bus runs on narrow screens, as a share of the gap, clear of the row captions below it. */
const captionedBus = 0.3;
/** The easing of a card travelling between places. */
const easing = "cubic-bezier(0.6, 0, 0.2, 1)";
/** The length of the dash of light that runs down a cable, as a share of the cable. */
const pulseLength = 0.12;
/** The milliseconds the dash of light takes to run down a cable. */
const pulseTime = 600;
/** The milliseconds after a cable appears before its dash of light starts. */
const pulseDelay = 150;
/** A merged card settling into place from slightly smaller. */
const land = style.keyframes({
    from: { opacity: 0, scale: "0.98" },
});
/** How far a silo sinks before it is gone, deep enough for the water to hide it. */
const sinkDepth = "8rem";
/** The easing of a vendor app sinking under the water. */
const sink = "cubic-bezier(0.5, 0, 0.9, 0.6)";

/** Return the left edge and width of a card's span, as CSS lengths within the layer. */
function span(place: Placement) {
    return {
        left: `calc(${frame.cell} * ${place.left})`,
        width: `calc(${frame.cell} * ${place.width})`,
    };
}

/** Return the centre of a card's span in pixels, for a given cell width. */
function centre(place: Placement, cell: number) {
    return (place.left + place.width / 2) * cell;
}

/** Return a card's inline motion: how it enters and leaves. */
function motion(isVendor: boolean, place: Placement): Record<string, string> {
    // read the card's visibility
    const isShown = place.isShown;

    // sink a silo under the water and fade it as it goes deep, and raise the next one out of it onto its berg
    if (isVendor) {
        return isShown
            ? {
                  opacity: "1",
                  translate: "0 0",
                  rotate: "0deg",
                  scale: "1",
                  transition: `opacity 300ms ease ${place.delay}ms, translate ${moveTime}ms ${easing} ${place.delay}ms, rotate ${moveTime}ms ${easing} ${place.delay}ms, scale 500ms ${easing} ${place.delay}ms`,
              }
            : {
                  opacity: "0",
                  translate: `0 ${sinkDepth}`,
                  rotate: "5deg",
                  scale: "1",
                  "pointer-events": "none",
                  transition: `translate 600ms ${sink}, rotate 600ms ${sink}, opacity 400ms ease 200ms`,
              };
    }

    // turn a card over in its slot: the old one turns away, and the new one turns into view from behind it
    if (place.step === "flip" || place.turn !== undefined) {
        const half = flipTime / 2;
        const at = place.delay + (isShown ? half : 0);
        return {
            opacity: isShown ? "1" : "0",
            rotate: `x ${isShown ? 0 : (place.turn ?? 90)}deg`,
            "transform-origin": "50% 0",
            translate: "0 0",
            "z-index": isShown ? "2" : "1",
            transition:
                place.step === "park"
                    ? "none"
                    : `rotate ${half}ms ${isShown ? "cubic-bezier(0.2, 0.7, 0.3, 1.2)" : "cubic-bezier(0.6, 0, 0.9, 0.5)"} ${at}ms, opacity 0ms linear ${place.delay + half}ms`,
            ...(isShown ? {} : { "pointer-events": "none" }),
        };
    }

    // travel cards whole or morph them into each other: a card going away fades its ink out, then hands its box over to the card
    //  taking its place, whose ink fades in, so the boxes never blink
    const swap = swapOf(place);
    const move = `left ${moveTime}ms ${easing} ${place.delay}ms, top ${moveTime}ms ${easing} ${place.delay}ms, width ${moveTime}ms ${easing} ${place.delay}ms, opacity 0ms linear ${place.step === "stay" ? 0 : swap}ms`;
    // fade in a card that grows out of a merge as it takes over
    const landing: Record<string, string> =
        isShown && place.isLate && place.step !== "park"
            ? { animation: `${land} 320ms ${easing} ${swap}ms both` }
            : {};

    return {
        ...landing,
        rotate: "x 0deg",
        opacity: isShown ? "1" : "0",
        "z-index": isShown ? "2" : "1",
        translate: "0 0",
        transition: place.step === "park" ? "none" : move,
        ...(place.isShown ? {} : { "pointer-events": "none" }),
    };
}

/** Return when a moving card hands its box over to the card taking its place, in milliseconds. */
function swapOf(place: Placement) {
    return place.isLate ? place.delay + moveTime : place.delay + inkFade;
}

/** Return how a card's ink fades: its opacity, fade time and delay. */
function inkOf(isVendor: boolean, place: Placement): Ink {
    // show the ink of a silo and of a turning card at once
    if (isVendor || place.step === "flip" || place.turn !== undefined) {
        return { opacity: "1", time: "0ms", delay: "0ms" };
    }

    // fade a travelling card's ink out before its box hands over, and the arriving card's ink in after
    const swap = swapOf(place);
    const delay = place.step === "stay" ? 0 : place.isLate ? swap : place.delay + moveTime * 0.3;

    return place.isShown
        ? { opacity: "1", time: `${inkFade}ms`, delay: `${delay}ms` }
        : { opacity: "0", time: `${inkFade}ms`, delay: `${swap - inkFade}ms` };
}

/** Add the drag offset to a card's motion: the card follows the pointer while held and springs home when let go. */
function follow(
    entrance: Record<string, string>,
    held: { x: number; y: number } | undefined,
): Record<string, string> {
    // spring the offset home on its own, apart from the entrance and exit delays
    const offset = held ? `translate(${held.x}px, ${held.y}px)` : "translate(0px, 0px)";
    const release = held ? "transform 0ms" : `transform 400ms ${easing}`;

    return {
        ...entrance,
        transform: `${offset} translateY(-50%)`,
        transition:
            entrance["transition"] === "none" ? release : `${entrance["transition"]}, ${release}`,
    };
}

/** Return a small square plug at a cable end, as an SVG path. */
function plug(point: Point) {
    return `M${point.x - 2} ${point.y - 2}h4v4h-4Z`;
}

/**
 * Place every card a scene shows in whole board cells.
 *
 * The upper row shares the board evenly with four-cell gaps; the lower row snaps to the three board columns.
 */
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
            isLate: false,
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
            isLate: false,
        });
        column += card.span;
    }

    return placed;
}

/** Return how long a silo waits to show: landing on the reformed ice after the water returns, bobbing up in a swap, or staying put. */
function vendorDelay(previous: Placement | undefined, isToggle: boolean) {
    // land after the ice reforms
    if (isToggle) {
        return landingDelay;
    }
    // bob up after the silo it replaces starts to sink
    else if (previous?.isShown !== true) {
        return swapDelay;
    }
    // stay where it floats
    else {
        return 0;
    }
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

/** Return how many cards that showed before and are gone now a placement replaces. */
function sourcesOf(
    place: Placement,
    before: ReadonlyMap<string, Placement>,
    current: ReadonlyMap<string, Placement>,
) {
    return [...before].filter(
        ([id, previous]) => previous.isShown && !current.has(id) && overlaps(previous, place),
    ).length;
}

/** Return which third of the board a placement starts in, from 0 at the left to 3 at the right. */
function slotOf(place: Placement) {
    return Math.round((place.left / boardCells) * 4);
}

/** Return whether two placements take the same slot: the same row, edge, and width. */
function sameSlot(first: Placement, second: Placement) {
    return (
        first.row === second.row &&
        Math.abs(first.left - second.left) < 0.5 &&
        Math.abs(first.width - second.width) < 0.5
    );
}

/** Return whether two placements on the same row overlap. */
function overlaps(first: Placement, second: Placement) {
    return (
        first.row === second.row &&
        first.left < second.left + second.width &&
        second.left < first.left + first.width
    );
}

/** Return a hidden placement covering all the given placements on their row. */
function cover(places: Placement[]): Placement {
    // reject an empty cover, which has no row
    const [first] = places;
    if (!first) {
        throw new TypeError("cover takes at least one placement");
    }
    const left = Math.min(...places.map((place) => place.left));
    const right = Math.max(...places.map((place) => place.left + place.width));

    return {
        row: first.row,
        left,
        width: right - left,
        isShown: false,
        step: "stay",
        delay: 0,
        isLate: false,
    };
}

/** Return whether a placement lies on the board rather than beyond an edge. */
function isOnBoard(place: Placement) {
    return place.left >= 0 && place.left + place.width <= boardCells;
}

/** Return how far a placement sits from the board edge it enters and leaves by, in cells. */
function edgeDistance(place: Placement) {
    const middle = place.left + place.width / 2;
    return middle <= boardCells / 2 ? place.left : boardCells - place.left - place.width;
}

/** Return a placement moved just beyond the board edge nearest to it, hidden. */
function beyond(place: Placement): Placement {
    const middle = place.left + place.width / 2;
    const left = middle <= boardCells / 2 ? -place.width - 2 : boardCells + 2;

    return { ...place, left, isShown: false, delay: 0 };
}

/** Delay each card by its turn: the lowest key goes first, then one every 70 milliseconds after a start. */
function stagger(places: Placement[], start: number, key: (place: Placement) => number) {
    const order = places.toSorted((first, second) => key(first) - key(second));
    order.forEach((place, turn) => {
        place.delay = start + turn * 70;
    });
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

/** Build a locked scene: the people above, one silo per iceberg below, and the silo each person signs in to. */
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
        perspective: "900px",
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
        pointerEvents: "auto",
        position: "absolute",
        touchAction: "none",
        userSelect: "none",
    },
    ink: (ink: Ink) => ({
        [cardVariables.ink]: ink.opacity,
        [cardVariables.inkTime]: ink.time,
        [cardVariables.inkDelay]: ink.delay,
    }),
    held: {
        cursor: "grabbing",
        zIndex: 2,
    },
    fill: {
        width: "100%",
    },
});
