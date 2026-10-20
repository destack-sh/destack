import { frame } from "../../layout/frame.stylex";
import { color, font, shadow, stroke } from "@destack/theme/tokens.stylex";
import { present } from "@destack/schema";
import * as style from "@destack/style";
import { Shader, type ShaderFailure } from "@destack/shader";
import { createMemo, createSignal, For, type JSX, onSettled } from "@destack/view";

import { charge } from "../../effect/goo";
import {
    type Bergs,
    TRAVEL,
    WATER_FRAGMENT,
    WATER_SPILL,
    type WaterColors,
    type WaterLayer,
    Waterline,
} from "../../effect/water";
import { commandEvents } from "../../layout/command";
import { lattice } from "../../layout/lattice.stylex";
import { Crossfade } from "../figure/crossfade";
import { Label } from "../entry/label";
import { morph } from "../figure/morph";
import { plate } from "../figure/plate.stylex";
import { browserStyles } from "../figure/browser";
import { chrome } from "../figure/chrome.stylex";
import { cardVariables } from "./card.stylex";
import { Band, Card, DuctTape, type Entity } from "./card";
import {
    boardCells,
    columnCentres,
    columnLefts,
    columnWidth,
    quarterLefts,
    quarterWidth,
} from "./board";
import { Remix, sceneSources, scenes, slotApps, todayScenes, usesOf } from "./remix";
import { palette } from "../../palette.stylex";
import { captureException } from "../../telemetry.ts";
import { media } from "@destack/style/media.stylex";
import { screen } from "../../layout/screen.stylex";
import { isMotionReduced } from "@destack/view/motion";
import { createEventListener } from "@destack/view/primitives/event-listener";
import { makeIntersectionObserver } from "@destack/view/primitives/intersection-observer";
import { makeResizeObserver } from "@destack/view/primitives/resize-observer";
import { makeTimer } from "@destack/view/primitives/timer";

/** The rows above the waterline today. */
const dryRows = 2;

/** How much faster the dry rows turn than the water travels, done within its first half row. */
const DRY_PACE = 2;

/** How far behind the row above each dry row starts turning, in rows of the water's travel. */
const DRY_STAGGER = 0.5;

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
    /** What the layer means for you, as a line under the claim. */
    note: Record<Stack, string>;
    /** What the layer costs you, as a bill line counted from the scene's people and vendors. */
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
        claim: { today: "Sign up everywhere", destack: "Sign in once" },
        detail: { today: "Their accounts", destack: "OIDC, Passkeys" },
        note: {
            today: "another account with every new app",
            destack: "one account and one set of permissions for every app",
        },
        item: {
            today: ({ people, vendors }) => `${people * vendors} logins`,
            destack: ({ people }) => `${people} logins`,
        },
    },
    {
        name: "Apps",
        claim: { today: "Settle for what they ship", destack: "Make every app yours" },
        detail: { today: "Closed apps", destack: "TS, HTML, CSS" },
        note: {
            today: "features arrive when they ship them",
            destack: "features arrive when you or your agent add them",
        },
        item: {
            today: ({ vendors }) => `${vendors} roadmaps`,
            destack: () => "your roadmap",
        },
    },
    {
        name: "Services",
        claim: { today: "Call their APIs", destack: "Build on one API" },
        detail: { today: "Private APIs", destack: "OpenAPI, JSON Schema" },
        note: {
            today: "integrations break when their API changes",
            destack: "apps build on each other without glue code",
        },
        item: {
            today: ({ vendors }) => `${vendors} APIs`,
            destack: () => "1 API",
        },
    },
    {
        name: "Data",
        claim: { today: "Rent your data", destack: "Own your data" },
        detail: { today: "Vendor formats", destack: "SQL, S3, R2" },
        note: {
            today: "exports only in the formats they allow",
            destack: "keep your context and use it everywhere",
        },
        item: {
            today: ({ vendors }) => `${vendors} silos`,
            destack: () => "1 database",
        },
    },
    {
        name: "Source",
        claim: { today: "Trust a black box", destack: "Fork any app" },
        detail: { today: "Closed source", destack: "Git, npm" },
        note: {
            today: "code you can't read or change",
            destack: "every app is a Git repository, yours to read and change",
        },
        item: {
            today: ({ vendors }) => `${vendors} closed codebases`,
            destack: () => "open source",
        },
    },
    {
        name: "Hosts",
        claim: { today: "Run where they let you", destack: "Run anywhere" },
        detail: { today: "Their cloud", destack: "Bun, Workers" },
        note: {
            today: "prices set by their margins",
            destack: "your laptop, your server or our cloud",
        },
        item: {
            today: ({ vendors }) => `${vendors} clouds`,
            destack: () => "any machine",
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

/** The milliseconds each silo holds its column before the next swap. */
const swapTime = 6500;

/** The layers every Destack app shares, one per band row, with the packages each holds. */
const shared: readonly {
    entity: Entity;
    items: readonly Entity[];
}[] = [
    {
        entity: { label: "Services", icon: "services", role: "Shared logic" },
        items: [
            {
                label: "Access",
                icon: "auth",
                role: "Roles",
            },
            {
                label: "Settings",
                icon: "settings",
                role: "JSON",
            },
            {
                label: "Search",
                icon: "search",
                role: "Full text",
            },
            {
                label: "AI",
                icon: "ai",
                role: "Your own",
            },
        ],
    },
    {
        entity: { label: "Data", icon: "storage", role: "Your data" },
        items: [
            {
                label: "DB",
                icon: "storage",
                role: "SQL",
            },
            {
                label: "Bucket",
                icon: "bucket",
                role: "S3",
            },
            {
                label: "Vault",
                icon: "vault",
                role: "Secrets",
            },
            {
                label: "Audit",
                icon: "audit",
                role: "Change log",
            },
        ],
    },
    {
        entity: { label: "Source", icon: "source", role: "Your code" },
        items: [
            {
                label: "Repository",
                icon: "source",
                role: "Git",
            },
            {
                label: "Registry",
                icon: "registry",
                role: "npm",
            },
            {
                label: "Build",
                icon: "build",
                role: "Node",
            },
            {
                label: "Templates",
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
        role: "Tunnelled",
    },
    {
        label: "Your server",
        icon: "hosts",
        role: "Tunnelled",
    },
    {
        label: "Your cloud",
        icon: "cloud",
        role: "Self-hosted",
    },
    {
        label: "Our cloud",
        icon: "cloud",
        role: "Hosted by us",
    },
];

/** The colors of the water and its ice, following the appearance. */
const WATER_COLORS: WaterColors = {
    shallow: palette.waterShallow,
    deep: palette.waterDeep,
    caustic: palette.waterLight,
    foam: palette.cream,
    ice: palette.ice,
    iceShadow: palette.iceShadow,
    ink: palette.ink,
};

/** The most device pixels the water and ice draw: the desktop figure's 1216 by 602 at 2x. */
const WATER_PIXELS = 2432 * 1204;

/** The box of the water and ice layers. */
const spillStyle = {
    "clip-path": `inset(0 ${WATER_SPILL}px ${WATER_SPILL}px)`,
    height: `calc(100% + ${WATER_SPILL}px)`,
    left: `-${WATER_SPILL}px`,
    width: `calc(100% + ${WATER_SPILL * 2}px)`,
};

/** How far each strip of duct tape is turned off level, in degrees, one per gap. */
const tapeTurns: readonly number[] = [-4, 3];

/** The connectors taped between the vendor apps. */
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
/** The gaps between the three columns that duct tape spans. */
const tapeGaps = [0, 1];

/** Compare apps today, their lower layers under water, with the open Destack stack drained. */
export function StackFigure(properties: { onChange: (isOpen: boolean) => void }) {
    // hold the chosen stack, the scene, the canvases, and the effect timers
    const [stack, setStack] = createSignal<Stack>("today");
    const [isPainted, setIsPainted] = createSignal(false);
    const [scene, setScene] = createSignal(0);
    const [isLive, setIsLive] = createSignal(false);
    const [today, setToday] = createSignal(0);
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
    let waterElement: HTMLDivElement | undefined;
    let settle: ReturnType<typeof setTimeout> | undefined;
    let isStill = false;

    // hold the waterline, the bergs under it, and a count of its moves that refreshes the shaders
    const waterline = new Waterline(0);
    const [bergs, setBergs] = createSignal<Bergs>();
    const [moves, setMoves] = createSignal(0);
    const waterValues = (layer: WaterLayer) => {
        moves();

        return waterline.values(layer, bergs(), WATER_COLORS);
    };

    // read the rendered elements or throw when the figure lacks them
    const elements = () => {
        if (!figureElement || !drawingElement || !waterElement) {
            throw new TypeError("the stack figure rendered without its drawing and water");
        }

        return { figure: figureElement, drawing: drawingElement, water: waterElement };
    };

    // hold how far the water has left each row
    const reveals = layers.map(() => 0);

    // keep the drawing's place within the water canvas, measured only when the layout changes
    const board = { offset: 0, shift: 0, width: 0, height: 0, depth: 0 };
    const measure = () => {
        // read the drawing's offset and size within the water canvas
        const bounds = elements().drawing.getBoundingClientRect();
        const origin = elements().water.getBoundingClientRect();
        board.offset = bounds.top - origin.top;
        board.shift = bounds.left - origin.left;
        board.width = bounds.width;
        board.height = bounds.height;
        board.depth = elements().water.clientHeight;

        // stand an iceberg under each column, from the dry rows down to the bottom of the drawing
        const cell = board.width / boardCells;
        setBergs({
            centres: columnCentres.map((centre) => board.shift + cell * centre),
            half: (cell * columnWidth) / 2,
            top: board.offset + (board.height * dryRows) / layers.length,
            bottom: board.offset + board.height,
            tip: (board.height / layers.length) * 1.6,
        });
    };

    // return the waterline of a configuration in canvas pixels
    const waterlineOf = (next: Stack) =>
        next === "destack"
            ? board.depth + 12
            : board.offset + (board.height * dryRows) / layers.length;

    // reveal each row by how far the waterline has passed it
    const follow = (level: number) => {
        // light the rows the waterline has passed
        setIsPainted(true);
        const { offset, height } = board;
        const row = height / layers.length;
        const figureStyle = elements().figure.style;
        const start = offset + row * dryRows;
        for (const index of reveals.keys()) {
            // reveal the dry rows as the water starts to move, then each row as the water passes it
            const passed =
                index < dryRows
                    ? ((level - start) / row) * DRY_PACE - index * DRY_STAGGER
                    : (level - offset - row * index) / row;
            const reveal = Math.max(0, Math.min(1, passed));
            figureStyle.setProperty(`--reveal-${index}`, reveal.toFixed(3));
            reveals[index] = reveal;
        }
    };

    // light the layer on the pointer's row over the drawing or the legend
    const skim = (event: PointerEvent) => {
        const bounds = elements().figure.getBoundingClientRect();
        const row = Math.floor(((event.clientY - bounds.top) / bounds.height) * layers.length);
        setLit(Math.max(0, Math.min(layers.length - 1, row)));
    };

    // select a configuration, keep the reader's choice, and move the water
    const select = (next: Stack) => {
        // store the choice and tell the page
        setStack(next);
        properties.onChange(next === "destack");
        charge(0);

        // bring the open stack to life only once the water has fully drained
        clearTimeout(settle);
        setIsLive(false);
        if (next === "destack") {
            settle = setTimeout(() => setIsLive(true), TRAVEL);
        }

        // move the water, at once when motion is reduced, revealing rows as it passes them
        if (isStill) {
            waterline.place(waterlineOf(next));
        } else {
            waterline.moveTo(waterlineOf(next));
        }
        setMoves(moves() + 1);
        trace();
    };

    // follow the waterline frame by frame until its move ends
    let tracing = 0;
    const trace = () => {
        cancelAnimationFrame(tracing);
        const step = () => {
            follow(waterline.level());
            if (waterline.progress() < 1) {
                tracing = requestAnimationFrame(step);
            }
        };
        step();
    };

    // flip the stack whenever the page's switch asks for it
    createEventListener(
        () => document,
        commandEvents.switchStack,
        () => morph(() => select(isOpen() ? "today" : "destack")),
    );

    // rest the water on its line, and again through resizes
    const rest = () => {
        // place the bergs and the waterline, and reveal the rows above it
        measure();
        waterline.place(waterlineOf(stack()));
        setMoves(moves() + 1);
        follow(waterline.level());
    };
    const resizes = makeResizeObserver(rest);

    // pause the water and every animation while the figure is off screen
    const sight = makeIntersectionObserver([], (entries) => {
        for (const entry of entries) {
            entry.target.toggleAttribute("data-asleep", !entry.isIntersecting);
        }
    });

    // swap one silo at a time while the stack is locked and the figure is on screen
    const swap = () => {
        if (!isOpen() && !elements().figure.hasAttribute("data-asleep")) {
            setToday((today() + 1) % todayScenes.length);
        }
    };

    // start the water once the figure is in the page
    onSettled(() => {
        // require the rendered drawing and water, and read the motion preference
        const { figure, drawing } = elements();
        isStill = isMotionReduced(figure);

        // swap the silos unless motion is reduced, rest the water, and watch the figure
        const stopSwapping = isStill ? undefined : makeTimer(swap, swapTime, setInterval);
        rest();
        resizes.observe(drawing);
        sight.add(figure);

        return () => {
            // stop the swapping, the settling timer and the frames
            stopSwapping?.();
            clearTimeout(settle);
            cancelAnimationFrame(tracing);
        };
    });

    return (
        <div {...style.attrs(lattice.frame, lattice.ruleBottom, styles.shell)}>
            {/* labels, each over its half of the window */}
            <div {...style.attrs(styles.labels)}>
                <span {...style.attrs(styles.drawingLabel)}>
                    <Label>Your apps, and what's under them</Label>
                </span>
                <span {...style.attrs(styles.layersLabel)}>
                    <Label>Every layer of the stack</Label>
                </span>
            </div>

            <div {...style.attrs(styles.window)}>
                {/* window strip, level with the other figures' windows */}
                <div aria-hidden="true" {...style.attrs(styles.strip)}>
                    <span {...style.attrs(browserStyles.lights)} />
                </div>

                <figure
                    ref={figureElement}
                    aria-label="Apps today compared with Destack"
                    onPointerMove={skim}
                    onPointerLeave={() => setLit(undefined)}
                    style={{
                        "--reveal-0": "0",
                        "--reveal-1": "0",
                        "--reveal-2": "0",
                        "--reveal-3": "0",
                        "--reveal-4": "0",
                        "--reveal-5": "0",
                    }}
                    {...style.attrs(styles.figure, isOpen() && styles.marked)}
                >
                    {/* say what each layer lets you do, verb first */}
                    <For each={layers}>
                        {(layer, index) => (
                            <div
                                {...style.attributes([styles.claim, styles.row(index() + 1)], {
                                    opacity: `calc(0.85 + 0.15 * var(--reveal-${index()}))`,
                                })}
                            >
                                {/* set the layer's number beside the count it costs */}
                                <span {...style.attrs(styles.numberLine)}>
                                    <span
                                        {...style.attrs(
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
                                        xstyle={styles.itemText}
                                    />
                                </span>
                                {/* set the verb and the things the layer is made of on one baseline */}
                                <span {...style.attrs(styles.claimLine)}>
                                    <Swap
                                        row={index()}
                                        isOpen={isOpen()}
                                        today={layer.claim.today}
                                        destack={layer.claim.destack}
                                        xstyle={styles.claimText}
                                    />
                                    <Swap
                                        row={index()}
                                        isOpen={isOpen()}
                                        today={layer.detail.today}
                                        destack={layer.detail.destack}
                                        isPlated
                                        xstyle={styles.detailText}
                                    />
                                </span>
                                {/* say what the layer means for you under the claim */}
                                <Swap
                                    row={index()}
                                    isOpen={isOpen()}
                                    today={layer.note.today}
                                    destack={layer.note.destack}
                                    xstyle={styles.noteText}
                                />
                            </div>
                        )}
                    </For>

                    {/* stand the icebergs behind the cards, riding the waterline */}
                    <Shader
                        aria-hidden="true"
                        fragmentShader={WATER_FRAGMENT}
                        uniforms={waterValues("ice")}
                        speed={1}
                        maxPixelCount={WATER_PIXELS}
                        style={spillStyle}
                        xstyle={[styles.ice, !isPainted() && styles.unpainted]}
                        onFailure={reportFailure}
                    />

                    {/* draw both configurations in the six middle columns */}
                    <div ref={drawingElement} {...style.attrs(lattice.ruleRight, styles.drawing)}>
                        {/* place every entity on its row and column */}
                        {/* sink each silo's hidden layers under its column, showing only the silo on top of it now */}
                        {columnLefts.map((left, column) =>
                            [0, 1, 2, 3].map((index) => (
                                <div
                                    style={{
                                        left: `calc(${frame.cell} * ${left})`,
                                        width: `calc(${frame.cell} * ${columnWidth})`,
                                        top: `calc(${frame.cellRow} * ${(dryRows + index) * 9 + 4.5})`,
                                        opacity: `calc(1 - var(--reveal-${dryRows + index}))`,
                                    }}
                                    {...style.attrs(styles.column, styles.sunkSlot)}
                                >
                                    {silosOf(column).map((id) => (
                                        <div
                                            class={
                                                style.attrs(
                                                    styles.sunkCard,
                                                    siloOn(
                                                        present(todayScenes[today()], "scene"),
                                                        column,
                                                    ) !== id && styles.sunkAway,
                                                ).class
                                            }
                                        >
                                            <Card
                                                entity={lockedLayer(id, index)}
                                                kind="locked"
                                                xstyle={styles.fill}
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
                                turn={tapeTurns[index] ?? 0}
                                left={`calc(${frame.cell} * ${(centreOf(index) + centreOf(index + 1)) / 2})`}
                                xstyle={[styles.tape, isOpen() ? styles.tapeGone : styles.tapeBack]}
                            />
                        ))}
                        {shared.map((entity, index) => (
                            <div
                                {...style.attributes(
                                    [
                                        styles.band,
                                        styles.row(dryRows + index + 1),
                                        styles.cascade(`${240 + index * 80}ms`),
                                    ],
                                    {
                                        "--band-reveal": `var(--reveal-${dryRows + index})`,
                                        "pointer-events": isOpen() ? "auto" : "none",
                                        ...growOutOfPlates(`var(--reveal-${dryRows + index})`),
                                    },
                                )}
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
                                    left: `calc(${frame.cell} * ${quarterLefts[index]})`,
                                    top: `calc(${frame.cellRow} * 49.5)`,
                                    width: `calc(${frame.cell} * ${quarterWidth})`,
                                    opacity: "var(--reveal-5)",
                                    "pointer-events": isOpen() ? "auto" : "none",
                                    translate: "0 calc((1 - var(--reveal-5)) * 40%)",
                                }}
                                {...style.attrs(styles.column)}
                            >
                                <Card entity={host} kind="plain" xstyle={styles.fill} />
                            </div>
                        ))}
                    </div>

                    {/* stand in for the water until the shader paints its first frame */}
                    <div
                        aria-hidden="true"
                        {...style.attrs(styles.pool, (isPainted() || isOpen()) && styles.poolGone)}
                    />
                    <Shader
                        ref={waterElement}
                        aria-hidden="true"
                        fragmentShader={WATER_FRAGMENT}
                        uniforms={waterValues("water")}
                        speed={1}
                        maxPixelCount={WATER_PIXELS}
                        style={spillStyle}
                        xstyle={[styles.water, !isPainted() && styles.unpainted]}
                        onMount={(mount) => {
                            waterline.mount = mount;
                            setIsPainted(true);
                        }}
                        onFailure={reportFailure}
                    />

                    {/* tint the lit layer across the drawing and its legend row, fading from row to row */}
                    {[...layers.keys()].map((index) => (
                        <span
                            aria-hidden="true"
                            style={{ "grid-row": String(index + 1) }}
                            {...style.attrs(styles.lit, lit() !== index && styles.litGone)}
                        />
                    ))}

                    {/* draw the legend's rule over the water, so no line breaks where it rises */}
                    <span aria-hidden="true" {...style.attrs(styles.columnRule)} />
                    {/* list the layers' claims under the drawing below desktop, where words over the wires would not read */}
                    <ol data-component="LayerList" {...style.attrs(styles.list)}>
                        <For each={layers}>
                            {(layer, index) => (
                                <li {...style.attrs(styles.listItem)}>
                                    <span
                                        {...style.attrs(
                                            styles.number,
                                            isOpen() && styles.numberLit,
                                        )}
                                    >
                                        0{index() + 1} {layer.name}
                                    </span>
                                    <b {...style.attrs(styles.listClaim)}>
                                        <Crossfade
                                            isOpen={isOpen()}
                                            stacked={layer.claim.today}
                                            destacked={layer.claim.destack}
                                        />
                                    </b>
                                    <span {...style.attrs(styles.listNote)}>
                                        <Crossfade
                                            isOpen={isOpen()}
                                            stacked={layer.note.today}
                                            destacked={layer.note.destack}
                                        />
                                    </span>
                                </li>
                            )}
                        </For>
                    </ol>
                </figure>
            </div>
        </div>
    );
}

/** Report the water failing to draw, which leaves the still pool in its place. */
function reportFailure(failure: ShaderFailure): void {
    captureException(new Error(`the water shader failed: ${failure}`), {
        tags: { feature: "water" },
    });
}

/** Return the items a shared layer lights up in an open scene. */
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
function centreOf(column: number): number {
    const centre = columnCentres[column];
    if (centre === undefined) {
        throw new TypeError(`missing column ${column}`);
    }

    return centre;
}

/** Return the silos that take turns on a column. */
function silosOf(column: number): readonly string[] {
    const silos = slotApps[column];
    if (!silos) {
        throw new TypeError(`no silos ride column ${column}`);
    }

    return silos;
}

/** Return the silo riding a column in a locked scene. */
function siloOn(scene: (typeof todayScenes)[number], column: number): string {
    const card = scene.lower[column];
    if (!card) {
        throw new TypeError(`no silo rides column ${column}`);
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

/** Return the mask that grows a shared band out of the three plates sunk in its row. */
function growOutOfPlates(reveal: string): JSX.CSSProperties {
    // widen each window from its plate's span to its third of the board, overlapping a little so no gap shows
    const third = boardCells / 3;
    const windows = columnLefts.map((left, index) => {
        const end = index * third - 0.2;
        return {
            left: `calc(${frame.cell} * (${left} + (${end - left}) * ${reveal}))`,
            width: `calc(${frame.cell} * (${columnWidth} + (${third + 0.4 - columnWidth}) * ${reveal}))`,
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

/** Return the four layers a homemade silo rents under water, each with its products' logos. */
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
    xstyle?: style.Styles;
}) {
    // set a thing as one dashed plate today, and as one plate per standard once open
    const plated = (text: string, isOpen: boolean) =>
        properties.isPlated === true ? (
            <span {...style.attrs(styles.plates)}>
                {(isOpen ? text.split(", ") : [text]).map((part) => (
                    <span {...style.attrs(plate.plate, !isOpen && plate.closed)}>{part}</span>
                ))}
            </span>
        ) : (
            text
        );
    const reveal = `var(--reveal-${properties.row})`;

    return (
        <span {...style.attrs(styles.swap, properties.xstyle)}>
            <span
                style={{
                    opacity: `clamp(0, 1 - ${reveal} * 2, 1)`,
                    "pointer-events": properties.isOpen ? "none" : "auto",
                }}
                {...style.attrs(styles.swapText)}
            >
                {plated(properties.today, false)}
            </span>
            <span
                style={{
                    opacity: `clamp(0, ${reveal} * 2 - 1, 1)`,
                    "pointer-events": properties.isOpen ? "auto" : "none",
                }}
                {...style.attrs(styles.swapText)}
            >
                {plated(properties.destack, true)}
            </span>
        </span>
    );
}

/** Return the label taped across a gap between two columns, picked by the silos it joins. */
function tapeLabel(gap: number, scene: (typeof todayScenes)[number]) {
    // place each silo in its column's turn, and step through the labels by coprime strides
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
const styles = style.create({
    shell: {
        backgroundColor: color.background,
        paddingBlock: `1.125rem ${frame.inset}`,
        paddingInline: frame.inset,
        rowGap: "0.75rem",
    },
    labels: {
        display: "grid",
        gridColumn: "1 / -1",
        gridTemplateColumns: {
            default: "repeat(12, minmax(0, 1fr))",
            [media.maxMd]: "repeat(4, minmax(0, 1fr))",
        },
    },
    drawingLabel: {
        gridColumn: { default: "1 / 9", [screen.belowDesktop]: "1 / -1" },
    },
    layersLabel: {
        display: { default: "block", [screen.belowDesktop]: "none" },
        gridColumn: "9 / -1",
        paddingInlineStart: frame.inset,
    },
    window: {
        backgroundColor: color.card,
        borderColor: color.border,
        borderRadius: chrome.radius,
        borderStyle: "solid",
        borderWidth: stroke.border,
        boxShadow: shadow.raised,
        display: "grid",
        gridColumn: "1 / -1",
        gridTemplateRows: "auto auto",
        overflow: "hidden",
    },
    strip: {
        alignItems: "center",
        backgroundColor: color.muted,
        boxSizing: "border-box",
        display: "flex",
        height: chrome.strip,
        paddingInline: "0.875rem",
    },
    figure: {
        display: "grid",
        flexGrow: 1,
        gridTemplateColumns: {
            default: "repeat(12, minmax(0, 1fr))",
            [media.maxMd]: "repeat(4, minmax(0, 1fr))",
        },
        gridTemplateRows: `repeat(6, ${frame.stage})`,
        margin: 0,
        minWidth: 0,
        position: "relative",
    },
    marked: {
        [cardVariables.marks]: "1",
    },
    row: (row: number) => ({
        gridRow: String(row),
    }),
    cascade: (delay: string) => ({
        [cardVariables.cascade]: delay,
    }),
    numberLine: {
        alignItems: "baseline",
        display: "flex",
        gap: "1rem",
        justifyContent: "space-between",
    },
    list: {
        backgroundColor: color.background,
        display: { default: "none", [screen.belowDesktop]: "grid" },
        position: "relative",
        zIndex: 4,
        gridColumn: "1 / -1",
        gridRow: "7 / 8",
        listStyle: "none",
        margin: 0,
        paddingBottom: "1rem",
        paddingTop: "0.5rem",
        paddingInline: frame.inset,
    },
    listItem: {
        borderTopColor: color.border,
        borderTopStyle: "solid",
        borderTopWidth: { default: stroke.border, ":first-child": 0 },
        display: "grid",
        gap: "0.25rem",
        paddingBlock: "0.75rem",
    },
    listClaim: {
        fontSize: "1rem",
        fontWeight: 600,
        lineHeight: "1.375rem",
    },
    listNote: {
        color: color.mutedForeground,
        fontSize: "0.8125rem",
        lineHeight: "1.125rem",
    },
    claim: {
        display: { default: "flex", [screen.belowDesktop]: "none" },
        flexDirection: { default: "column", [screen.belowDesktop]: "row" },
        gap: "0.25rem",
        gridColumn: { default: "9 / span 4", [screen.belowDesktop]: "1 / -1" },
        justifyContent: { default: "center", [media.maxMd]: "flex-start" },
        paddingInline: { default: frame.inset, [media.maxMd]: "0.75rem" },
        position: "relative",
        zIndex: 3,
        alignItems: { default: null, [screen.belowDesktop]: "baseline" },
        alignSelf: { default: null, [screen.belowDesktop]: "start" },
        columnGap: { default: null, [screen.belowDesktop]: "1rem", [media.maxMd]: "0.625rem" },
        paddingTop: { default: null, [screen.belowDesktop]: "0.625rem" },
    },
    claimLine: {
        alignItems: "baseline",
        columnGap: "1rem",
        display: "flex",
        flexWrap: "wrap",
        justifyContent: "space-between",
        rowGap: "0.375rem",
        flexGrow: { default: null, [screen.belowDesktop]: 1 },
    },
    number: {
        color: color.mutedForeground,
        fontFamily: font.code,
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
    claimText: {
        fontSize: "1rem",
        fontWeight: 600,
        lineHeight: "1.375rem",
        whiteSpace: "nowrap",
    },
    lit: {
        backgroundColor: `color-mix(in srgb, ${palette.signal} 8%, transparent)`,
        gridColumn: "1 / -1",
        pointerEvents: "none",
        transition: `opacity 300ms ${easing}`,
        zIndex: 3,
        display: { default: null, [screen.belowDesktop]: "none" },
    },
    litGone: {
        opacity: 0,
    },
    columnRule: {
        borderRightColor: color.border,
        borderRightStyle: "solid",
        borderRightWidth: stroke.border,
        gridColumn: "1 / span 8",
        gridRow: "1 / span 6",
        pointerEvents: "none",
        position: "relative",
        zIndex: 6,
        display: { default: null, [screen.belowDesktop]: "none" },
    },
    drawing: {
        display: "grid",
        gridColumn: { default: "1 / span 8", [screen.belowDesktop]: "1 / -1" },
        gridRow: "1 / span 6",
        gridTemplateColumns: "repeat(3, minmax(0, 1fr))",
        gridTemplateRows: "repeat(6, minmax(0, 1fr))",
        minWidth: 0,
        position: "relative",
        borderRightWidth: { default: stroke.border, [screen.belowDesktop]: 0 },
    },
    fill: {
        width: "100%",
    },
    sunkCard: {
        gridArea: "1 / 1",
        transition: { default: `translate 500ms ${easing} 200ms`, [media.motionReduce]: "none" },
        width: "100%",
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
        paddingInline: `calc(${frame.cell} * 3)`,
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
        top: `calc(${frame.cellRow} * 13.5)`,
        display: { default: null, [media.maxMd]: "none" },
    },
    plates: {
        display: "flex",
        gap: "0.375rem",
        justifyContent: "flex-end",
    },
    noteText: {
        display: { default: "grid", [media.maxMd]: "none" },
        color: color.foreground,
        opacity: 0.72,
        fontSize: "0.8125rem",
        lineHeight: "1.125rem",
        whiteSpace: "nowrap",
    },
    itemText: {
        justifyItems: "end",
        fontFamily: font.code,
        fontSize: "0.75rem",
        letterSpacing: "0.02em",
        lineHeight: "1.125rem",
        opacity: 0.8,
        display: { default: "grid", [screen.belowDesktop]: "none" },
    },
    detailText: {
        marginLeft: "auto",
        color: color.foreground,
        fontFamily: font.code,
        fontSize: "0.8125rem",
        fontWeight: 600,
        letterSpacing: "0.02em",
        lineHeight: "1.375rem",
        translate: "0 0.125rem",
        display: { default: "grid", [media.maxMd]: "none" },
    },
    pool: {
        backgroundColor: palette.water,
        backgroundImage: "linear-gradient(transparent, color-mix(in srgb, black 45%, transparent))",
        borderTopColor: `color-mix(in srgb, white 85%, transparent)`,
        borderTopStyle: "solid",
        borderTopWidth: "2px",
        bottom: 0,
        left: 0,
        opacity: 0.9,
        pointerEvents: "none",
        position: "absolute",
        right: 0,
        top: `calc(${frame.stage} * ${dryRows} - 1px)`,
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
        transition: `opacity 300ms ${easing} 600ms`,
    },
    ice: {
        pointerEvents: "none",
        position: "absolute",
        top: 0,
        transition: `opacity 400ms ${easing}`,
        zIndex: 0,
    },
    water: {
        pointerEvents: "none",
        position: "absolute",
        top: 0,
        transition: `opacity 400ms ${easing}`,
        zIndex: 2,
    },
});
