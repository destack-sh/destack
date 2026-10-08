import { frame } from "../../layout/frame.stylex";
import { color, font, stroke } from "@destack/theme/tokens.stylex";
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
import { plate } from "../figure/plate.stylex";
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
    /** What the layer means for you, as a line under the claim. */
    note: Record<Stack, string>;
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
        claim: { today: "Juggle logins", destack: "Sign in once" },
        detail: { today: "Their accounts", destack: "One account, Every agent" },
        note: { today: "another account with every new app", destack: "one account for all apps" },
        item: {
            today: ({ people, vendors }) => `${people * vendors} logins`,
            destack: ({ people }) => `${people} accounts`,
        },
    },
    {
        name: "Apps",
        claim: { today: "Duct-tape your apps", destack: "Remix your apps" },
        detail: { today: "Closed apps", destack: "TS, HTML, CSS" },
        note: {
            today: "features arrive when they ship them",
            destack: "features arrive when you ask for them",
        },
        item: {
            today: ({ people, vendors }) => `${people * vendors} licences`,
            destack: () => "0 licences",
        },
    },
    {
        name: "Services",
        claim: { today: "Wait on their roadmap", destack: "Build on one API" },
        detail: { today: "Private APIs", destack: "HTTP, OpenAPI" },
        note: {
            today: "integrations break when their API changes",
            destack: "every app calls the same API",
        },
        item: {
            today: ({ vendors }) => `${vendors} APIs`,
            destack: () => "1 API",
        },
    },
    {
        name: "Data",
        claim: { today: "Rent your data", destack: "Own your data" },
        detail: { today: "Vendor formats", destack: "SQL, JSON, MD, S3" },
        note: {
            today: "exports only in the formats they allow",
            destack: "queries in any format you like",
        },
        item: {
            today: ({ vendors }) => `${vendors} silos`,
            destack: () => "1 data store",
        },
    },
    {
        name: "Source",
        claim: { today: "Take it on trust", destack: "Check it yourself" },
        detail: { today: "Closed source", destack: "Git, npm" },
        note: { today: "code you can't read or change", destack: "code you can read and change" },
        item: { today: ({ vendors }) => `${vendors} black boxes`, destack: () => "0 black boxes" },
    },
    {
        name: "Hosts",
        claim: { today: "Run where they say", destack: "Run where you like" },
        detail: { today: "Their cloud", destack: "Node, Docker, Workers" },
        note: { today: "prices set by their margins", destack: "prices set by your hardware" },
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

/** The box of the water and ice layers: past the figure on the sides and bottom, clipped to its rim. */
const spillStyle = {
    "clip-path": `inset(0 ${WATER_SPILL}px ${WATER_SPILL}px)`,
    height: `calc(100% + ${WATER_SPILL}px)`,
    left: `-${WATER_SPILL}px`,
    width: `calc(100% + ${WATER_SPILL * 2}px)`,
};

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
        for (const index of reveals.keys()) {
            // leave the dry rows above the waterline alone
            if (index < dryRows) {
                continue;
            }
            const passed = (level - offset - row * index) / row;
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

    // start the water once the figure is in the page
    onSettled(() => {
        // require the rendered drawing and water
        const { figure, drawing } = elements();

        // flip the stack whenever the page's switch asks for it
        const flip = () => select(isOpen() ? "today" : "destack");
        document.addEventListener(commandEvents.switchStack, flip);

        // read the motion preference
        isStill = window.matchMedia("(prefers-reduced-motion: reduce)").matches;

        // swap one silo at a time while the stack is locked
        const swapping = isStill
            ? undefined
            : setInterval(() => {
                  if (!isOpen()) {
                      setToday((today() + 1) % todayScenes.length);
                  }
              }, swapTime);

        // rest the water on its line, and again through resizes
        const rest = () => {
            // place the bergs and the waterline, and reveal the rows above it
            measure();
            waterline.place(waterlineOf(stack()));
            setMoves(moves() + 1);
            follow(waterline.level());
        };
        rest();
        const resize = new ResizeObserver(rest);
        resize.observe(drawing);

        // pause the water and every animation while the figure is off screen
        const sight = new IntersectionObserver((entries) => {
            for (const entry of entries) {
                figure.toggleAttribute("data-asleep", !entry.isIntersecting);
            }
        });
        sight.observe(figure);

        return () => {
            // stop listening, and stop the observers, timers, and frames
            document.removeEventListener(commandEvents.switchStack, flip);
            sight.disconnect();
            resize.disconnect();
            clearTimeout(settle);
            clearInterval(swapping);
            cancelAnimationFrame(tracing);
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
            }}
            {...style.attrs(
                lattice.frame,
                lattice.ruleBottom,
                styles.figure,
                isOpen() && styles.marked,
            )}
        >
            {/* say what each layer lets you do, verb first */}
            <For each={layers}>
                {(layer, index) => (
                    <div
                        {...style.attributes(
                            [styles.claim, styles.row(index() + 1)],
                            index() < dryRows
                                ? undefined
                                : { opacity: `calc(0.85 + 0.15 * var(--reveal-${index()}))` },
                        )}
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

            {/* draw the frame and the legend's rule over the water, so no line breaks where it rises */}
            <span aria-hidden="true" {...style.attrs(styles.frameRules)} />
            <span aria-hidden="true" {...style.attrs(styles.columnRule)} />
        </figure>
    );
}

/** Report the water failing to draw, which leaves the still pool in its place. */
function reportFailure(failure: ShaderFailure): void {
    captureException(new Error(`the water shader failed: ${failure}`), {
        tags: { feature: "water" },
    });
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

/** Return the mask that grows a shared band out of the three plates sunk in its row: three windows over the plates that widen until they meet. */
function growOutOfPlates(reveal: string): JSX.CSSProperties {
    // widen each window from its plate's span to its third of the board, overlapping a little so no seam shows
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
    xstyle?: style.Styles;
}) {
    // tell whether the row stays dry, and read how far the water has left it
    const isDry = properties.row < dryRows;

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
                    ...(isDry ? {} : { opacity: `clamp(0, 1 - ${reveal} * 2, 1)` }),
                    "pointer-events": properties.isOpen ? "none" : "auto",
                }}
                {...style.attrs(
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
                {...style.attrs(
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
const styles = style.create({
    figure: {
        flexGrow: 1,
        gridTemplateRows: `repeat(6, ${frame.stage})`,
        margin: 0,
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
    claim: {
        display: "flex",
        flexDirection: "column",
        gap: "0.25rem",
        gridColumn: "9 / span 4",
        justifyContent: "center",
        paddingInline: frame.inset,
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
        backgroundColor: `color-mix(in srgb, ${palette.signal} 8%, transparent)`,
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
        borderColor: color.border,
        borderStyle: "solid",
        borderWidth: `${stroke.border} ${stroke.border} ${stroke.border}`,
        inset: `0 calc(-1 * ${stroke.border}) calc(-1 * ${stroke.border})`,
        pointerEvents: "none",
        position: "absolute",
        zIndex: 6,
        [mobile]: { borderInlineWidth: 0 },
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
        [mobile]: { display: "none" },
    },
    plates: {
        display: "flex",
        gap: "0.375rem",
        justifyContent: "flex-end",
    },
    noteText: {
        color: color.foreground,
        opacity: 0.72,
        fontSize: "0.8125rem",
        lineHeight: "1.125rem",
        whiteSpace: "nowrap",
    },
    itemText: {
        fontFamily: font.code,
        fontSize: "0.75rem",
        letterSpacing: "0.02em",
        lineHeight: "1.125rem",
        opacity: 0.8,
        [narrow]: { display: "none" },
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
        [mobile]: { display: "none" },
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
