import { color } from "@destack/theme/tokens.stylex";
import * as stylex from "@destack/style";
import { createMemo, createSignal, For, onSettled } from "@destack/view";

import { sound } from "../effect/sound";
import { tokens } from "../style/tokens.stylex";
import { boardCells, boardInset, rowCells } from "./board";
import { Card, type Entity, type Reveal } from "./card";

/** The milliseconds each open scene holds before the next one begins. */
const sceneTime = 5200;
/** The milliseconds from the water fully draining to the first scene change. */
const firstSceneTime = 2400;
/** The milliseconds cards take to travel between places. */
const moveTime = 1400;

/** The humans, agents, and apps the scenes arrange. */
const entities: Record<string, Entity> = {
    you: { label: "You", icon: "user", role: "Human" },
    colleague: { label: "Colleague", icon: "user", role: "Human" },
    friend: { label: "Friend", icon: "user", role: "Human" },
    agent: { label: "Agent", icon: "agent", role: "Agent" },
    notion: { label: "Notion", icon: "notion", role: "$10/seat/mo" },
    slack: { label: "Slack", icon: "slack", role: "$8.75/seat/mo" },
    github: { label: "GitHub", icon: "github", role: "$4/seat/mo" },
    pages: { label: "Pages", icon: "pages", role: "App" },
    chat: { label: "Chat", icon: "chat", role: "App" },
    tasks: { label: "Tasks", icon: "tasks", role: "App" },
    planner: { label: "Planner", icon: "tasks", role: "Fork of Tasks" },
};

/** What each card shows under the searchlight today: every separate access a person needs, or ciphertext. */
const todayReveals: { [id: string]: Reveal | undefined } = {
    you: {
        kind: "fields",
        rows: [
            ["Notion", "login + 2FA"],
            ["Slack", "magic link"],
            ["GitHub", "SSO + 2FA"],
            ["billing", "3 plans"],
        ],
    },
    colleague: {
        kind: "fields",
        rows: [
            ["Notion", "seat pending"],
            ["Slack", "guest, 1 channel"],
            ["GitHub", "no seat"],
        ],
    },
    friend: {
        kind: "fields",
        rows: [
            ["Notion", "public link"],
            ["Slack", "no access"],
            ["GitHub", "no access"],
        ],
    },
    agent: {
        kind: "fields",
        rows: [
            ["BLOCKED", "Notion API"],
            ["BLOCKED", "Slack history"],
            ["ALLOWED", "GitHub MCP"],
        ],
    },
    notion: { kind: "cipher" },
    slack: { kind: "cipher" },
    github: { kind: "cipher" },
};

/** What each card shows under the searchlight with Destack: one identity and its grants, or the app's source. */
const openReveals: { [id: string]: Reveal | undefined } = {
    you: {
        kind: "fields",
        rows: [
            ["passkey", "one"],
            ["every app", "owner"],
        ],
    },
    colleague: {
        kind: "fields",
        rows: [
            ["passkey", "one"],
            ["pages", "edit"],
            ["chat", "post"],
        ],
    },
    friend: {
        kind: "fields",
        rows: [
            ["account", "none needed"],
            ["one page", "comment"],
        ],
    },
    agent: {
        kind: "fields",
        rows: [
            ["ALLOWED", "tasks: edit"],
            ["ALLOWED", "pages: read"],
            ["ASKS", "to send"],
        ],
    },
    pages: {
        kind: "code",
        name: "pages.tsx",
        lines: [
            "function Pages() {",
            "  const all = usePages();",
            "  return (",
            "    <For each={all()}>",
            "      {PageRow}",
            "    </For>",
            "  );",
            "}",
        ],
    },
    chat: {
        kind: "code",
        name: "chat.tsx",
        lines: [
            "function Chat() {",
            "  const said = useChat();",
            "  return (",
            "    <For each={said()}>",
            "      {Message}",
            "    </For>",
            "  );",
            "}",
        ],
    },

    tasks: {
        kind: "code",
        name: "tasks.ts",
        lines: [
            "async function add() {",
            "  await tasks.create({",
            '    title: "Ship v2",',
            "    due: nextWeek(),",
            "  });",
            "}",
        ],
    },
    planner: {
        kind: "code",
        name: "planner.tsx",
        lines: [
            "function Planner() {",
            "  return (",
            "    <Week>",
            "      <Tasks />",
            "    </Week>",
            "  );",
            "}",
        ],
    },
};

/** Every card the scenes can show. */
const ids = Object.keys(entities);
/** The vendor apps, in the order of the icebergs they ride. */
const vendors = ["notion", "slack", "github"];
/** The open apps, which anyone can fork. */
const apps = ["pages", "chat", "tasks", "planner"];

/** One arrangement of the top two layers. */
type Scene = {
    /** The cards on the upper row, left to right. */
    upper: readonly string[];
    /** The cards on the lower row, left to right, and whether each is reshaped wide. */
    lower: readonly { id: string; isWide?: boolean }[];
    /** Which upper card works with which lower card. */
    links: readonly [string, string][];
};

/** The locked stack today: everyone signs in to separate vendor apps. */
const today: Scene = {
    upper: ["you", "colleague", "friend", "agent"],
    lower: [{ id: "notion" }, { id: "slack" }, { id: "github" }],
    links: [
        ["you", "notion"],
        ["colleague", "slack"],
        ["friend", "notion"],
        ["agent", "github"],
    ],
};

/** The open loop: everyone shares the apps, an app is remixed, then an agent moves into the apps. */
const scenes: readonly Scene[] = [
    {
        upper: ["you", "colleague", "friend", "agent"],
        lower: [{ id: "pages" }, { id: "chat" }, { id: "tasks" }],
        links: [
            ["you", "pages"],
            ["you", "chat"],
            ["colleague", "chat"],
            ["colleague", "tasks"],
            ["friend", "pages"],
            ["agent", "tasks"],
            ["agent", "pages"],
        ],
    },
    {
        upper: ["you", "colleague", "friend", "agent"],
        lower: [{ id: "pages" }, { id: "chat" }, { id: "planner", isWide: true }],
        links: [
            ["you", "pages"],
            ["colleague", "planner"],
            ["friend", "pages"],
            ["friend", "chat"],
            ["agent", "planner"],
        ],
    },
    {
        upper: ["you", "colleague", "chat", "friend"],
        lower: [{ id: "pages" }, { id: "agent" }, { id: "planner", isWide: true }],
        links: [
            ["you", "pages"],
            ["colleague", "agent"],
            ["chat", "agent"],
            ["friend", "planner"],
        ],
    },
];

/** The board's height in cells across the three figure rows the layer covers. */
const layerRows = rowCells * 3;

/** The placement of every card in the locked stack, and in each open scene. */
const todayPlacements = arrange(today);
/** The placement of every card in each open scene. */
const scenePlacements = scenes.map(arrange);

/** How a card moves into its placement. */
type Step = "stay" | "enter" | "leave" | "park";

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
    /** How the card moves there: stays or travels on the board, enters or leaves across an edge, or parks off the board. */
    step: Step;
    /** The milliseconds the card waits before it moves. */
    delay: number;
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

/** One cable between two cards, its springy middle, and how visible it is. */
type Cable = {
    /** The drawn cable. */
    path: SVGPathElement;
    /** The plugs at both ends. */
    ends: SVGPathElement;
    /** The straight-line middle on the previous frame, to feel how fast the ends move. */
    middle: Point | undefined;
    /** How far the middle swings off the straight line. */
    offset: Point;
    /** How fast the middle swings. */
    velocity: Point;
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
    isLive: boolean;
    revealOf: (row: number) => number;
    onScene: (scene: number) => void;
}) {
    // hold the scene, the dragged card, and the elements to measure
    const [scene, setScene] = createSignal(0);
    const [drag, setDrag] = createSignal<{ id: string; x: number; y: number }>();
    let layer!: HTMLDivElement;
    let wires!: SVGGElement;
    const cards = new Map<string, HTMLDivElement>();
    const last = new Map<string, Placement>();

    // pick the scene on show: the locked stack today, the loop once open
    const shown = () => (properties.isOpen ? scenes[scene()] : today);

    // place every card of the scene: cards enter and leave across the nearest board edge, vendor apps sink in place
    let wasLaidOpen = properties.isOpen;
    const layout = createMemo(() => {
        // pick the placements and note whether the stack just opened or closed
        const current = properties.isOpen ? scenePlacements[scene()] : todayPlacements;
        const isToggle = properties.isOpen !== wasLaidOpen;
        wasLaidOpen = properties.isOpen;

        // step each card toward its placement, collecting the cards that enter and leave
        const entering: Placement[] = [];
        const leaving: Placement[] = [];
        for (const id of ids) {
            const placement = current.get(id);
            const previous = last.get(id);
            const isVendor = vendors.includes(id);

            // keep or move a card that shows
            if (placement) {
                const isEntering = previous !== undefined && !previous.isShown && !isVendor;
                const next: Placement = { ...placement, step: isEntering ? "enter" : "stay" };
                last.set(id, next);
                if (isEntering) {
                    entering.push(next);
                }
            }
            // sink a vendor app where it stands
            else if (isVendor) {
                last.set(id, {
                    ...(previous ?? todayPlacements.get(id)!),
                    isShown: false,
                    step: "stay",
                    delay: 0,
                });
            }
            // send a card that showed off across its nearest edge
            else if (previous?.isShown) {
                const next: Placement = { ...beyond(previous), step: "leave" };
                last.set(id, next);
                leaving.push(next);
            }
            // park a hidden card beyond the edge nearest where it next appears
            else {
                last.set(id, { ...beyond(upcoming(id, scene())), step: "park" });
            }
        }

        // start entering cards after the leaving ones clear, farthest from its edge first; send leaving cards nearest first
        const enterAt = isToggle ? 700 : moveTime * 0.45;
        const leaveAt = isToggle ? 1200 : 0;
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
        sound.play("click");
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
        // hold the cables, the frame, the scene timing, and the motion state
        const isStill = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
        const cables = new Map<string, Cable>();
        let frame: number | undefined;
        let nextScene: number | undefined;
        let wasOpen = properties.isOpen;
        let wasScene = scene();
        let changedAt = -Infinity;
        let delay = 0;
        let wakeUntil = 0;
        let isStirring = false;
        let isVisible = true;

        // measure the boxes of the given cards within the layer, all before any cable is drawn
        const measure = (ids: Iterable<string>) => {
            // measure each card against the layer bounds
            const bounds = layer.getBoundingClientRect();
            const boxes = new Map<string, Box>();
            for (const id of ids) {
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
                path.setAttribute("class", stylex.attrs(styles.cable, cableKinds[kind]).class!);
                ends.setAttribute("class", stylex.attrs(plugKinds[kind]).class!);
                wires.append(path, ends);
                found = {
                    path,
                    ends,
                    middle: undefined,
                    offset: { x: 0, y: 0 },
                    velocity: { x: 0, y: 0 },
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
            isStirring ||= found.alpha < 1;
        };

        // push a cable's middle against the motion of its ends, spring it back, and return how far it swings
        const swing = (found: Cable, middle: Point) => {
            // swing the middle against its ends' motion and damp it back
            const previous = found.middle ?? middle;
            found.middle = middle;
            const velocity = found.velocity;
            velocity.x =
                (velocity.x - (middle.x - previous.x) * 0.9 - found.offset.x * 0.09) * 0.84;
            velocity.y =
                (velocity.y - (middle.y - previous.y) * 0.9 - found.offset.y * 0.09) * 0.84;
            found.offset.x += velocity.x;
            found.offset.y += velocity.y;
            isStirring ||= Math.abs(velocity.x) + Math.abs(velocity.y) > 0.02;

            return found.offset;
        };

        // hang a locked cable between two cards as a taut curve that swings as they bob
        const hang = (key: string, from: Point, to: Point, now: number) => {
            // draw the plugs and a curve bent by the swing
            const found = cable(key, "locked");
            show(found, 1, now);
            found.ends.setAttribute("d", `${plug(from)}${plug(to)}`);
            const offset = swing(found, { x: (from.x + to.x) / 2, y: (from.y + to.y) / 2 });
            const half = (to.y - from.y) / 2;
            const bend = { x: offset.x * 1.33, y: offset.y * 1.33 };
            found.path.setAttribute(
                "d",
                `M${from.x} ${from.y}C${from.x + bend.x} ${from.y + half + bend.y} ${to.x + bend.x} ${to.y - half + bend.y} ${to.x} ${to.y}`,
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

            // swing the middle segment across its length while the cable is loose
            const offset = swing(found, { x: (from.x + to.x) / 2, y: (from.y + to.y) / 2 });
            const bend =
                mix(free.bend, grid.bend) + (isAcross ? offset.x : offset.y) * 1.33 * (1 - settle);

            // draw the plugs and the right-angled path
            found.ends.setAttribute("d", `${plug(from)}${plug(to)}`);
            found.path.setAttribute(
                "d",
                isAcross
                    ? `M${from.x} ${from.y}H${bend}V${to.y}H${to.x}`
                    : `M${from.x} ${from.y}V${bend}H${to.x}V${to.y}`,
            );
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

            // link the upper cards to the lower cards they work with, along the two hole rows between them once open
            const kind = isOpen ? "link" : "locked";
            current.links.forEach(([upper, lower], index) => {
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
                run(
                    key,
                    kind,
                    { from: start, to: end, bend: (start.y + end.y) / 2 },
                    {
                        from: { x: hole(from.centre), y: from.bottom },
                        to: { x: hole(to.centre), y: to.top },
                        bend: cellRow * (8.5 + (index % 2)),
                    },
                    rests(upper, from) && rests(lower, to),
                    false,
                    1,
                    now,
                );
            });

            // once open, chain the lower cards along a hole row and drop each into the services below its place
            if (isOpen) {
                const reveal = properties.revealOf(2);
                const floor = cellRow * 20;
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

                    // drop this card into the services below it
                    const key = `drop:${card.id}`;
                    kept.add(key);
                    const place = places.get(card.id);
                    const anchor = { x: hole(place ? centre(place, cell) : card.centre), y: floor };
                    const start = { x: card.centre, y: card.bottom };
                    run(
                        key,
                        "drop",
                        { from: start, to: anchor, bend: (start.y + anchor.y) / 2 },
                        {
                            from: { x: anchor.x, y: card.bottom },
                            to: anchor,
                            bend: (card.bottom + anchor.y) / 2,
                        },
                        card.isResting,
                        false,
                        reveal,
                        now,
                    );
                });
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
                if (found.alpha === 0) {
                    found.path.remove();
                    found.ends.remove();
                    cables.delete(key);
                }
            }
        };

        // time the cables to the choreography, and step the scenes once the water has drained
        const loop = (now: number) => {
            // restart the cable timing when the stack opens, closes, or changes scene
            if (properties.isOpen !== wasOpen) {
                changedAt = now;
                delay = properties.isOpen ? 1100 : 3000;
                wasOpen = properties.isOpen;
            } else if (scene() !== wasScene) {
                changedAt = now;
                delay = moveTime * 0.8;
            }
            wasScene = scene();

            // reset the scenes while locked, and step them while open and live
            if (!properties.isOpen) {
                nextScene = undefined;
                if (scene() !== 0) {
                    setScene(0);
                    properties.onScene(0);
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

            // trace only while something moves: bobbing ice, dancing users, travelling cards, a drag, or a swinging cable
            const isBobbing = !properties.isOpen && !isStill;
            const isBusy =
                isBobbing || drag() || now - changedAt < 4500 || now < wakeUntil || isStirring;
            if (isVisible && isBusy) {
                trace(now);
            }
            if (drag()) {
                wakeUntil = now + 1500;
            }
            frame = requestAnimationFrame(loop);
        };
        frame = requestAnimationFrame(loop);

        // retrace after the layout changes, and rest while off screen
        const sizes = new ResizeObserver(() => {
            wakeUntil = performance.now() + 300;
        });
        sizes.observe(layer);
        const sight = new IntersectionObserver(([entry]) => {
            isVisible = entry.isIntersecting;
        });
        sight.observe(layer);

        return () => {
            sizes.disconnect();
            sight.disconnect();
            if (frame !== undefined) {
                cancelAnimationFrame(frame);
            }
        };
    });

    return (
        <div
            ref={layer}
            style={{
                height: `calc(${tokens.cellRow} * ${layerRows})`,
                width: `calc(${tokens.cell} * ${boardCells})`,
            }}
            {...stylex.attrs(styles.layer)}
        >
            <svg aria-hidden="true" {...stylex.attrs(styles.lines)}>
                <g ref={wires} />
            </svg>

            {/* place each card on its row; cards travel between rows and can be dragged */}
            <For each={ids}>
                {(id) => {
                    // read the card's placement, drag offset, and kind
                    const placement = () => layout().get(id)!;
                    const held = () => (drag()?.id === id ? drag() : undefined);
                    const vendor = vendors.indexOf(id);
                    const isPerson = vendor < 0 && !apps.includes(id);

                    return (
                        <div
                            ref={(element) => cards.set(id, element)}
                            onPointerDown={(event) => grab(id, event)}
                            style={{
                                ...span(placement()),
                                top: placement().row === 0 ? "calc(100% / 6)" : "50%",
                                ...motion(vendor >= 0, placement(), held()),
                            }}
                            class={stylex.attrs(styles.card, held() && styles.held).class}
                        >
                            <div
                                style={{
                                    "--toward": id === "you" || id === "friend" ? "1" : "-1",
                                    "animation-delay": `${-ids.indexOf(id) * 0.9}s`,
                                    transform:
                                        vendor >= 0
                                            ? "translateY(var(--lift, 0px)) rotate(var(--tilt, 0deg))"
                                            : undefined,
                                }}
                                data-bob={vendor >= 0 ? String(vendor) : undefined}
                                class={
                                    stylex.attrs(
                                        styles.fill,
                                        isPerson && styles.dance,
                                        isPerson && properties.isOpen && styles.still,
                                    ).class
                                }
                            >
                                <Card
                                    entity={entities[id]}
                                    kind={vendor >= 0 ? "vendor" : "plain"}
                                    reveal={properties.isOpen ? openReveals[id] : todayReveals[id]}
                                    style={styles.fill}
                                />
                            </div>
                        </div>
                    );
                }}
            </For>
        </div>
    );
}

/** The easing of a card travelling between places. */
const easing = "cubic-bezier(0.6, 0, 0.2, 1)";
/** The easing of a card springing back from a drag. */
const spring = "cubic-bezier(0.3, 1.45, 0.5, 1)";
/** The easing of a vendor app sinking with the ice. */
const sink = "cubic-bezier(0.5, 0, 0.9, 0.6)";

/** Return the left edge and width of a card's span, as CSS lengths within the layer. */
function span(place: Placement) {
    return {
        left: `calc(${tokens.cell} * ${place.left})`,
        width: `calc(${tokens.cell} * ${place.width})`,
    };
}

/** Return the centre of a card's span in pixels, for a given cell width. */
function centre(place: Placement, cell: number) {
    return (place.left + place.width / 2) * cell;
}

/** Return a card's inline motion: how it enters, leaves, and springs back from a drag. */
function motion(
    isVendor: boolean,
    place: Placement,
    held: { x: number; y: number } | undefined,
): Record<string, string> {
    // read the card's visibility and drag offset
    const isShown = place.isShown;
    const offset = held ? `${held.x}px ${held.y}px` : "0 0";

    // sink vendor apps with the shattering ice, then land them on the reformed ice
    if (isVendor) {
        return isShown
            ? {
                  opacity: "1",
                  translate: offset,
                  rotate: "0deg",
                  scale: "1",
                  transition: held
                      ? "none"
                      : `opacity 400ms ${easing} 2700ms, scale 450ms ${spring} 2700ms, translate 700ms ${spring}, rotate 0ms linear 2700ms`,
              }
            : {
                  opacity: "0",
                  translate: "0 2.5rem",
                  rotate: "8deg",
                  scale: "1.12",
                  "pointer-events": "none",
                  transition: `opacity 550ms ${sink} 150ms, translate 800ms ${sink} 150ms, rotate 800ms ${sink} 150ms, scale 0ms linear 1000ms`,
              };
    }

    // travel cards across and off the board whole, and spring them back after a drag
    const move = `left ${moveTime}ms ${easing} ${place.delay}ms, top ${moveTime}ms ${easing} ${place.delay}ms, width ${moveTime}ms ${easing}`;

    return {
        translate: offset,
        transition:
            place.step === "park" ? "none" : held ? move : `${move}, translate 700ms ${spring}`,
        ...(place.isShown ? {} : { "pointer-events": "none" }),
    };
}

/** Return a small square plug at a cable end, as an SVG path. */
function plug(point: Point) {
    return `M${point.x - 2} ${point.y - 2}h4v4h-4Z`;
}

/**
 * Place every card a scene shows in whole board cells.
 *
 * Rows of four get four-cell gaps, rows of three get five, and reshaped apps grow half as wide again.
 */
function arrange(scene: Scene): Map<string, Placement> {
    // lay out both rows left to right
    const placed = new Map<string, Placement>();
    const rows = [scene.upper.map((id) => ({ id, isWide: false })), scene.lower];
    rows.forEach((cards, row) => {
        // share the free cells among the row's cards by weight
        const gap = cards.length >= 4 ? 4 : 5;
        const free = boardCells - boardInset * 2 - gap * (cards.length - 1);
        const weights = cards.map((card) => (card.isWide ? 1.5 : 1));
        const total = weights.reduce((sum, weight) => sum + weight, 0);
        let left = boardInset;
        let used = 0;
        cards.forEach((card, index) => {
            // give the last card the cells left over
            const isLast = index === cards.length - 1;
            const width = isLast ? free - used : Math.round((free * weights[index]) / total);
            placed.set(card.id, {
                row: row === 0 ? 0 : 1,
                left,
                width,
                isShown: true,
                step: "stay",
                delay: 0,
            });
            left += width + gap;
            used += width;
        });
    });

    return placed;
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

/** Delay each card by its turn: the lowest key goes first, then one every 180 milliseconds after a start. */
function stagger(places: Placement[], start: number, key: (place: Placement) => number) {
    const order = [...places].sort((first, second) => key(first) - key(second));
    order.forEach((place, turn) => {
        place.delay = start + turn * 180;
    });
}

/** Return where a card next appears after an open scene, or in the locked stack. */
function upcoming(id: string, from: number): Placement {
    // search the scenes that follow in order
    for (let step = 1; step <= scenes.length; step++) {
        const placement = scenePlacements[(from + step) % scenes.length].get(id);
        if (placement) {
            return placement;
        }
    }

    // find the card in the locked stack
    const locked = todayPlacements.get(id);
    if (locked) {
        return locked;
    }

    // fail when no scene shows the card
    throw new Error(`card ${id} never appears`);
}

/** The sway of the people cards dancing while locked. */
const dance = stylex.keyframes({
    "0%, 100%": { transform: "translate(0, 0) rotate(0deg)" },
    "34%": {
        transform:
            "translate(calc(var(--toward) * 7px * var(--sway)), calc(-2px * var(--sway))) rotate(calc(var(--toward) * 2deg * var(--sway)))",
    },
    "40%": {
        transform:
            "translate(calc(var(--toward) * 4px * var(--sway)), calc(1px * var(--sway))) rotate(calc(var(--toward) * -3deg * var(--sway)))",
    },
    "48%": {
        transform:
            "translate(calc(var(--toward) * 6px * var(--sway)), 0) rotate(calc(var(--toward) * 1deg * var(--sway)))",
    },
    "72%": {
        transform:
            "translate(calc(var(--toward) * -3px * var(--sway)), calc(-1px * var(--sway))) rotate(calc(var(--toward) * -1.5deg * var(--sway)))",
    },
});

/** The cable strokes for each cable kind. */
const cableKinds = stylex.create({
    locked: { stroke: color.mutedForeground },
    link: { stroke: color.primary, strokeDasharray: "3 4" },
    chain: { stroke: color.primary },
    drop: { stroke: color.primary },
});

/** The plug fills for each cable kind. */
const plugKinds = stylex.create({
    locked: { fill: color.mutedForeground },
    link: { fill: color.primary },
    chain: { fill: color.primary },
    drop: { fill: color.primary },
});

/** The remix layer styles. */
const styles = stylex.create({
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
    card: {
        cursor: "grab",
        display: "flex",
        justifyContent: "center",
        pointerEvents: "auto",
        position: "absolute",
        touchAction: "none",
        transform: "translateY(-50%)",
        userSelect: "none",
    },
    held: {
        cursor: "grabbing",
        zIndex: 2,
    },
    dance: {
        animationDuration: "3.6s",
        animationIterationCount: "infinite",
        animationName: dance,
        animationTimingFunction: "ease-in-out",
        transition: `--sway 900ms ${easing}`,
        "@media (prefers-reduced-motion: reduce)": { animationName: "none" },
    },
    still: {
        "--sway": "0",
        animationPlayState: "paused",
    },
    fill: {
        width: "100%",
    },
});
