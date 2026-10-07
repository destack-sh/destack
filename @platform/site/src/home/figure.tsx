import { color } from "@destack/theme/tokens.stylex";
import { present } from "@destack/schema";
import * as stylex from "@destack/style";
import { createMemo, createSignal, For, type JSX, onSettled } from "@destack/view";

import { isDarkPage, isWeakGraphics } from "../effect/gl";
import { charge } from "../effect/goo";
import {
    nightWater,
    paperWater,
    travel,
    Water,
    type WaterPalette,
    waterSpill,
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
} from "./board";
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
        claim: { today: "Take it on trust", destack: "Fork the code" },
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

/** The milliseconds each silo holds its column before the next swap. */
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

/** The outline of each iceberg's tip above the water, one per column, in a box 100 wide and 60 tall resting on the waterline. */
const tips: readonly string[] = [
    "4,60 13,38 24,43 37,14 49,24 60,4 72,28 84,22 96,60",
    "5,60 16,30 28,36 41,8 54,20 66,12 78,34 88,30 95,60",
    "4,60 12,42 25,26 38,32 50,6 63,18 75,14 86,36 96,60",
];

/** The facet lines inside each iceberg's tip, one per column, in the same box. */
const facets: readonly string[] = [
    "37,14 42,60 M60,4 58,42 72,28",
    "41,8 46,60 M66,12 64,44 78,34",
    "50,6 47,60 M25,26 30,48 M75,14 72,46",
];

/** How far each strip of duct tape is turned off level, in degrees, one per gap. */
const tapeTurns: readonly number[] = [-4, 3];

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
/** The gaps between the three columns that duct tape spans. */
const tapeGaps = [0, 1];

/** Compare apps today, their lower layers under water, with the open Destack stack revealed by draining it. */
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
    let canvasElement: HTMLCanvasElement | undefined;
    let water: Water | undefined;
    let settle: ReturnType<typeof setTimeout> | undefined;

    // read the rendered elements or throw when the figure lacks them
    const elements = () => {
        if (!figureElement || !drawingElement || !canvasElement) {
            throw new TypeError("the stack figure rendered without its drawing and canvas");
        }

        return { figure: figureElement, drawing: drawingElement, canvas: canvasElement };
    };

    // hold how far the water has left each row
    const reveals = layers.map(() => 0);

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

        // stand an iceberg under each column, from the dry rows down to the bottom of the drawing
        const cell = frame.width / boardCells;
        water?.shape({
            centres: columnCentres.map((centre) => frame.shift + cell * centre),
            half: (cell * columnWidth) / 2,
            top: frame.offset + (frame.height * dryRows) / layers.length,
            bottom: frame.offset + frame.height,
        });
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
            settle = setTimeout(() => setIsLive(true), travel);
        }

        // move the water, or reveal every row at once without it
        if (water) {
            water.moveTo(waterlineOf(next));
        } else {
            follow(next === "destack" ? Number.MAX_SAFE_INTEGER : 0);
        }
    };

    // start the water once the figure is in the page
    onSettled(() => {
        // require the rendered drawing and canvas
        const { figure, drawing, canvas } = elements();

        // flip the stack whenever the page's switch asks for it
        const flip = () => select(isOpen() ? "today" : "destack");
        document.addEventListener(commandEvents.switchStack, flip);

        // read the motion preference
        const isStill = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
        const scheme = window.matchMedia("(prefers-color-scheme: dark)");

        // swap one silo at a time while the stack is locked
        const swapping = isStill
            ? undefined
            : setInterval(() => {
                  if (!isOpen()) {
                      setToday((today() + 1) % todayScenes.length);
                  }
              }, swapTime);

        // start the water, or leave the figure dry when the browser has no WebGL
        measure();
        try {
            water = new Water(
                canvas,
                palette(),
                waterlineOf(stack()),
                !isStill && !isWeakGraphics(),
                follow,
            );
            measure();
            water.shader.request();
        } catch (error) {
            log.error("water.render.failed", telemetry.exceptionAttributes(error, false));
        }

        // repaint on theme changes and keep the water on the waterline through resizes
        const repaint = () => water?.paint(palette());
        const themes = new MutationObserver(repaint);
        themes.observe(document.documentElement, { attributeFilter: ["data-theme"] });
        scheme.addEventListener("change", repaint);
        const resize = new ResizeObserver(() => {
            // measure the drawing again and rest the water on its line
            measure();
            water?.place(waterlineOf(stack()));
        });
        resize.observe(drawing);

        // pause the water and every animation while the figure is off screen
        const sight = new IntersectionObserver((entries) => {
            for (const entry of entries) {
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
            clearInterval(swapping);
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

            {/* draw both configurations in the six middle columns */}
            <div ref={drawingElement} {...stylex.attrs(lattice.ruleRight, styles.drawing)}>
                {/* raise each iceberg's tip above the waterline behind the cards, sinking it as the water drains */}
                {columnLefts.map((left, column) => (
                    <svg
                        aria-hidden="true"
                        viewBox="0 0 100 60"
                        preserveAspectRatio="none"
                        style={{
                            left: `calc(${tokens.cell} * ${left})`,
                            width: `calc(${tokens.cell} * ${columnWidth})`,
                            top: `calc(${tokens.cellRow} * ${dryRows * 9 - 13})`,
                            height: `calc(${tokens.cellRow} * 13)`,
                            opacity: "calc(1 - var(--reveal-2))",
                            translate: "0 calc(var(--reveal-2) * 30%)",
                        }}
                        {...stylex.attrs(styles.tip)}
                    >
                        <polygon points={tips[column] ?? ""} {...stylex.attrs(styles.tipFace)} />
                        <path d={`M${facets[column] ?? ""}`} {...stylex.attrs(styles.tipFacet)} />
                    </svg>
                ))}

                {/* place every entity on its row and column */}
                {/* sink each silo's hidden layers under its column, showing only the silo on top of it now */}
                {columnLefts.map((left, column) =>
                    [0, 1, 2, 3].map((index) => (
                        <div
                            style={{
                                left: `calc(${tokens.cell} * ${left})`,
                                width: `calc(${tokens.cell} * ${columnWidth})`,
                                top: `calc(${tokens.cellRow} * ${(dryRows + index) * 9 + 4.5})`,
                                opacity: `calc(1 - var(--reveal-${dryRows + index}))`,
                            }}
                            {...stylex.attrs(styles.column, styles.sunkSlot)}
                        >
                            {silosOf(column).map((id) => (
                                <div
                                    class={
                                        stylex.attrs(
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
                        turn={tapeTurns[index] ?? 0}
                        left={`calc(${tokens.cell} * ${(centreOf(index) + centreOf(index + 1)) / 2})`}
                        style={[styles.tape, isOpen() ? styles.tapeGone : styles.tapeBack]}
                    />
                ))}
                {shared.map((entity, index) => (
                    <div
                        style={{
                            "--row": String(dryRows + index + 1),
                            "--cascade": `${240 + index * 80}ms`,
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

/** Return the label taped across a gap between two columns, picked by the pair of silos it joins, so any swap on either side retapes it. */
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
        transition: `opacity 300ms ${easing} 600ms`,
        [still]: { transition: "none" },
    },
    swapLeave: {
        opacity: 0,
        transition: `opacity 250ms ${easing} 500ms`,
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
        transition: `translate 500ms ${easing} 200ms`,
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
    tip: {
        overflow: "visible",
        pointerEvents: "none",
        position: "absolute",
        zIndex: 0,
    },
    tipFace: {
        fill: `color-mix(in srgb, ${color.card} 94%, ${tokens.signalInk})`,
        stroke: `color-mix(in srgb, ${tokens.signalInk} 28%, transparent)`,
        strokeLinejoin: "round",
        strokeWidth: 1,
        vectorEffect: "non-scaling-stroke",
    },
    tipFacet: {
        fill: "none",
        stroke: `color-mix(in srgb, ${tokens.signalInk} 12%, transparent)`,
        strokeWidth: 1,
        vectorEffect: "non-scaling-stroke",
    },
    tape: {
        top: `calc(${tokens.cellRow} * 13.5)`,
        [mobile]: { display: "none" },
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
        transition: `opacity 300ms ${easing} 600ms`,
    },
    water: {
        pointerEvents: "none",
        position: "absolute",
        top: 0,
        transition: `opacity 400ms ${easing}`,
        zIndex: 2,
    },
});
