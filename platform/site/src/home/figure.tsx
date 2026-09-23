import { color } from "@destack/theme/tokens.stylex";
import * as stylex from "@destack/style";
import { createMemo, createSignal, For, type JSX, onSettled } from "@destack/view";

import { Ice } from "../effect/ice";
import { sound } from "../effect/sound";
import { Sparks } from "../effect/sparks";
import { drainAt, nightWater, paperWater, travel, Water } from "../effect/water";
import { lattice } from "../style/lattice.stylex";
import { tokens } from "../style/tokens.stylex";
import { Band, Card, DuctTape, type Entity, type Reveal } from "./card";
import { boardCells, columnCentres, columnLefts, columnWidth, rowCells } from "./board";
import { Flotsam } from "./flotsam";
import { Remix } from "./remix";

const mobile = "@media (max-width: 767px)";
const still = "@media (prefers-reduced-motion: reduce)";

/// The rows above the waterline today.
const dryRows = 2;
/// The switch's name for the stack today, set in crooked letters.
const stacked = "\u201cThe Stack\u201d";

/// How far each letter of the stack's name leans, in degrees, and sags, in pixels.
const crooked = [
    [-8, 1],
    [-7, 1],
    [5, -1],
    [-3, 2],
    [0, 0],
    [8, 0],
    [-6, -2],
    [4, 1],
    [-5, -1],
    [6, 0],
    [7, -1],
];

/// The milliseconds of calm on the water before things start to drift past.
const adriftDelay = 20000;

/// The milliseconds the shattered ice waits before it clumps back together as the water returns.
const reformDelay = 1100;

/// The two configurations the figure compares.
type Stack = "today" | "destack";

/// One layer of the stack in both configurations.
type Layer = {
    /// The layer name.
    name: string;
    /// What you do with the layer, verb first.
    claim: Record<Stack, string>;
    /// What kind of thing the layer is made of.
    topic: Record<Stack, string>;
    /// What the layer is made of.
    detail: Record<Stack, string>;
};

/// The six layers, from the users of the stack down to where it runs.
const layers: readonly Layer[] = [
    {
        name: "Users",
        claim: { today: "Beg for entry", destack: "Bring everyone" },
        topic: { today: "", destack: "" },
        detail: { today: "", destack: "" },
    },
    {
        name: "Apps",
        claim: { today: "Duct-tape silos", destack: "Remix software" },
        topic: { today: "Siloed apps", destack: "Open standards" },
        detail: { today: "Vertical icebergs", destack: "TS, HTML, CSS" },
    },
    {
        name: "Services",
        claim: { today: "Await roadmaps", destack: "Standardise logic" },
        topic: { today: "Hidden logic", destack: "Open protocols" },
        detail: { today: "Private APIs", destack: "HTTP, OpenAPI" },
    },
    {
        name: "Data",
        claim: { today: "Rent your data", destack: "Own your data" },
        topic: { today: "Locked data", destack: "Standard formats" },
        detail: { today: "Proprietary formats", destack: "SQL, JSON, MD" },
    },
    {
        name: "Source",
        claim: { today: "Trust blindly", destack: "Fork the code" },
        topic: { today: "No access", destack: "Conventional tools" },
        detail: { today: "Closed source", destack: "Git, npm" },
    },
    {
        name: "Hosts",
        claim: { today: "Pay their markup", destack: "Run everywhere" },
        topic: { today: "Vendor lock-in", destack: "Simple deployment" },
        detail: { today: "Their cloud", destack: "Node, Worker" },
    },
];

/// The vendor that owns each column's layers under water.
const owners = ["Notion", "Slack", "GitHub"];

/// The layers each vendor keeps under water, one per submerged row.
const locked: readonly Entity[] = [
    { label: "Services", icon: "services", role: "Vendor" },
    { label: "Data", icon: "storage", role: "Vendor" },
    { label: "Source", icon: "source", role: "Vendor" },
    { label: "Cloud", icon: "cloud", role: "Vendor" },
];

/// The layers every Destack app shares, one per band row, with what each holds and shows inside.
const shared: readonly {
    entity: Entity;
    items: readonly Entity[];
    reveals: readonly Reveal[];
}[] = [
    {
        entity: { label: "Services", icon: "services", role: "Standard backend" },
        items: [
            { label: "Auth", icon: "auth", role: "Service" },
            { label: "Sync", icon: "sync", role: "Service" },
            { label: "Search", icon: "search", role: "Service" },
            { label: "AI", icon: "ai", role: "Service" },
            { label: "Notify", icon: "notify", role: "Service" },
        ],
        reveals: [
            {
                kind: "code",
                name: "openapi.yaml",
                lines: [
                    "/auth/session:",
                    "  post:",
                    "    summary: Sign in",
                    "    security: passkey",
                    "    200: Session",
                ],
            },
            {
                kind: "code",
                name: "openapi.yaml",
                lines: [
                    "/tasks:",
                    "  get:",
                    "    summary: List",
                    "    query: due, owner",
                    "    200: Task[]",
                ],
            },
            {
                kind: "code",
                name: "openapi.yaml",
                lines: [
                    "/ai/complete:",
                    "  post:",
                    "    summary: Run model",
                    "    body: Prompt",
                    "    200: Completion",
                ],
            },
        ],
    },
    {
        entity: { label: "Data", icon: "storage", role: "Central storage" },
        items: [
            { label: "SQLite", icon: "table", role: "Storage" },
            { label: "Postgres", icon: "storage", role: "Storage" },
            { label: "Files", icon: "file", role: "Storage" },
            { label: "Vectors", icon: "vector", role: "Storage" },
        ],
        reveals: [
            {
                kind: "code",
                name: "tasks.sql",
                lines: [
                    "create table tasks (",
                    "  id text primary key,",
                    "  title text,",
                    "  due date,",
                    "  owner text",
                    ");",
                ],
            },
            {
                kind: "code",
                name: "plan.md",
                lines: [
                    "# Launch plan",
                    "- [x] Book the venue",
                    "- [ ] Send invites",
                    "- [ ] Ship v2",
                ],
            },
            {
                kind: "code",
                name: "t_0931.json",
                lines: [
                    "{",
                    '  "id": "t_0931",',
                    '  "title": "Ship v2",',
                    '  "due": "2026-10-01"',
                    "}",
                ],
            },
        ],
    },
    {
        entity: { label: "Source", icon: "source", role: "Universal packages" },
        items: [
            { label: "@you/stack", icon: "package", role: "Package" },
            { label: "@you/planner", icon: "fork", role: "Fork" },
            { label: "@friend/recipes-v3", icon: "fork", role: "Fork" },
        ],
        reveals: [
            {
                kind: "code",
                name: "git log",
                lines: [
                    "3f9a2c group by week",
                    "e02d4f fork tasks",
                    "a81e07 sum a column",
                    "9d44b1 add due dates",
                    "c4f1d8 dark theme",
                ],
            },
            {
                kind: "code",
                name: "package.json",
                lines: [
                    "{",
                    '  "version": "1.0.0",',
                    '  "forkOf": "tasks",',
                    '  "license": "MIT"',
                    "}",
                ],
            },
            {
                kind: "code",
                name: "planner.diff",
                lines: [
                    "@@ planner.tsx",
                    "- <List of={tasks} />",
                    "+ <Week of={tasks} />",
                    "+ <Grid days={7} />",
                ],
            },
        ],
    },
];

/// The hosts a space can run on.
const hosts: readonly { entity: Entity; reveal: Reveal }[] = [
    {
        entity: { label: "Your laptop", icon: "computer", role: "Host" },
        reveal: {
            kind: "code",
            name: "terminal",
            lines: ["$ destack dev", "ready on :3000", "synced with your server", "works offline"],
        },
    },
    {
        entity: { label: "Your server", icon: "hosts", role: "Host" },
        reveal: {
            kind: "code",
            name: "terminal",
            lines: [
                "$ docker compose up",
                "destack  running",
                "backups  nightly",
                "uptime   41 days",
            ],
        },
    },
    {
        entity: { label: "Your cloud", icon: "cloud", role: "Host" },
        reveal: {
            kind: "code",
            name: "terminal",
            lines: ["$ destack deploy", "workers  3 regions", "scales   to zero", "tls      ready"],
        },
    },
];

/// The length of a strip of duct tape, in pixels.
const tapeLength = 72;
/// How far each end of a strip of duct tape grips into its card, in pixels.
const tapeGrip = 13;
/// How far below each card's middle the two ends of each strip are stuck, in pixels.
const tapeDrops = [
    [-5, 3],
    [4, -4],
];

/// The entry each shared layer lights up in each remix scene: services, data, source.
const traffic: readonly (readonly number[])[] = [
    [0, 0, 0],
    [1, 1, 1],
    [3, 3, 2],
];

/// The connectors taped between the vendor apps.
const tapes = ["APIs", "MCPs"];

/// Compare apps today, as icebergs, with the open Destack stack revealed by draining the water.
export function StackFigure(props: { onChange: (isOpen: boolean) => void }) {
    const [stack, setStack] = createSignal<Stack>("today");
    const [isPainted, setIsPainted] = createSignal(false);
    const [scene, setScene] = createSignal(0);
    const [isLive, setIsLive] = createSignal(false);
    const [isAdrift, setIsAdrift] = createSignal(false);
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
    const frame = { offset: 0, width: 0, height: 0, depth: 0 };
    const measure = () => {
        const bounds = drawing.getBoundingClientRect();
        frame.offset = bounds.top - canvas.getBoundingClientRect().top;
        frame.width = bounds.width;
        frame.height = bounds.height;
        frame.depth = canvas.clientHeight;
    };

    // return the waterline of a configuration in canvas pixels
    const waterlineOf = (next: Stack) =>
        next === "destack"
            ? frame.depth + 12
            : frame.offset + (frame.height * dryRows) / layers.length;

    // reveal each row by how far the waterline has passed it
    const follow = (level: number) => {
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
    const aim = (event: PointerEvent) => {
        const bounds = figure.getBoundingClientRect();
        light.target = { x: event.clientX - bounds.left, y: event.clientY - bounds.top };
        const isControl = (event.target as Element).closest("button, a") !== null;
        light.isOn = event.pointerType === "mouse" && event.buttons === 0 && !isControl;
    };

    // put the searchlight out
    const leave = () => {
        light.isOn = false;
    };

    // select a configuration, keep the reader's choice, and move the water
    const select = (next: Stack) => {
        setStack(next);
        props.onChange(next === "destack");

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
            const origin = canvas.getBoundingClientRect();
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
                    canvas.clientWidth,
                    light.waterline + 10,
                    2,
                );
            }, 120);
        }

        // sound the change, and switch the background from sea to piano or back
        sound.play(next === "destack" ? "destack" : "restack");
        sound.follow(next);

        // shatter the ice as the water drains, or clump it back together just before the water returns
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

    onSettled(() => {
        const isStill = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
        drift();
        const scheme = window.matchMedia("(prefers-color-scheme: dark)");

        // pick the water palette for the page theme
        const palette = () => {
            const theme = document.documentElement.dataset.theme;
            const isDark = theme === "dark" || (theme === undefined && scheme.matches);
            return isDark ? nightWater : paperWater;
        };

        // start the ice and water, or leave the figure dry when the browser has no WebGL
        const waterline = () => (frame.height * dryRows) / layers.length;
        const shift = () =>
            drawing.getBoundingClientRect().left - canvas.getBoundingClientRect().left;
        let riders: { element: HTMLElement; bergs: number[] }[] | undefined;
        let strips: SVGElement[] | undefined;

        // stick both ends of a tape strip to its cards, then span, turn, and stretch it
        const stick = (tape: SVGElement, gap: number, lifts: number[], tilts: number[]) => {
            const cell = frame.width / boardCells;
            const middle = (frame.height / (rowCells * layers.length)) * rowCells * 1.5;
            const half = (cell * columnWidth) / 2 - tapeGrip;
            const anchor = (berg: number, side: number, drop: number) => {
                const angle = (tilts[berg] * Math.PI) / 180;
                const x = side * half;
                return {
                    x: cell * columnCentres[berg] + x * Math.cos(angle) - drop * Math.sin(angle),
                    y: middle + lifts[berg] + x * Math.sin(angle) + drop * Math.cos(angle),
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
            ice = new Ice(iceCanvas, centres, !isStill, (lifts, tilts) => {
                riders ??= [...figure.querySelectorAll<HTMLElement>("[data-bob]")].map(
                    (element) => ({
                        element,
                        bergs: element.dataset.bob!.split(" ").map(Number),
                    }),
                );
                for (const { element, bergs } of riders) {
                    const lift = bergs.reduce((sum, berg) => sum + lifts[berg], 0) / bergs.length;
                    const tilt = bergs.reduce((sum, berg) => sum + tilts[berg], 0) / bergs.length;
                    element.style.setProperty("--lift", `${lift.toFixed(2)}px`);
                    element.style.setProperty("--tilt", `${tilt.toFixed(2)}deg`);
                }
                strips ??= [...figure.querySelectorAll<SVGElement>("[data-tape]")];
                for (const tape of strips) {
                    stick(tape, Number(tape.dataset.tape), lifts, tilts);
                }
            });
            ice.place(waterline(), frame.height - waterline(), shift());
            water = new Water(canvas, palette(), waterlineOf(stack()), !isStill, follow);
            if (!isStill) {
                sparks = new Sparks(sparkCanvas);
                sparks.drain = { x: canvas.clientWidth * drainAt, y: canvas.clientHeight + 30 };
            }
            water.shader.request();
        } catch (error) {
            console.error("water rendering failed", error);
        }

        // repaint on theme changes and keep the ice and surface on the waterline through resizes
        const repaint = () => water?.paint(palette());
        const themes = new MutationObserver(repaint);
        themes.observe(document.documentElement, { attributeFilter: ["data-theme"] });
        scheme.addEventListener("change", repaint);
        const resize = new ResizeObserver(() => {
            measure();
            ice?.place(waterline(), frame.height - waterline(), shift());
            water?.place(waterlineOf(stack()));
        });
        resize.observe(drawing);

        // swing the searchlight after the pointer, clear the water under it, and show inside the boxes it falls on
        let beam: number | undefined;
        let isLooking = false;
        const shine = () => {
            beam = requestAnimationFrame(shine);
            if (!light.isOn && light.radius === 0) {
                return;
            }

            // measure the boxes with an inside, and light up only under water or over a visible box
            const bounds = figure.getBoundingClientRect();
            const boxes = [...figure.querySelectorAll<HTMLElement>("[data-inside]")];
            const places = boxes.map((box) => box.getBoundingClientRect());
            const pointer = { x: light.target.x + bounds.left, y: light.target.y + bounds.top };
            const isOverBox = places.some(
                (place, index) =>
                    pointer.x >= place.left &&
                    pointer.x <= place.right &&
                    pointer.y >= place.top &&
                    pointer.y <= place.bottom &&
                    boxes[index].checkVisibility({ opacityProperty: true }),
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

            lens.style.opacity = radius > 0 ? "1" : "0";
            if (radius > 0 !== isLooking) {
                isLooking = radius > 0;
                figure.style.setProperty("--looking", isLooking ? "running" : "paused");
            }
            lens.style.translate = `${light.x - radius}px ${light.y - radius}px`;
            lens.style.width = `${radius * 2}px`;
            lens.style.height = `${radius * 2}px`;
            water?.shine(light.x, light.y, radius);
            boxes.forEach((box, index) => {
                box.style.setProperty(
                    "--lens-x",
                    `${light.x + bounds.left - places[index].left}px`,
                );
                box.style.setProperty("--lens-y", `${light.y + bounds.top - places[index].top}px`);
                box.style.setProperty("--lens-radius", `${radius}px`);
            });
        };

        // pause the ice, water, and searchlight while the figure is off screen
        const sight = new IntersectionObserver(([entry]) => {
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
            sight.disconnect();
            themes.disconnect();
            resize.disconnect();
            scheme.removeEventListener("change", repaint);
            clearTimeout(settle);
            clearTimeout(calm);
            clearInterval(swirling);
            sparks?.stop();
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
            style={{ "--reveal-2": "0", "--reveal-3": "0", "--reveal-4": "0", "--reveal-5": "0" }}
            {...stylex.attrs(lattice.frame, lattice.ruleBottom, styles.figure)}
        >
            {/* say what each layer lets you do, verb first */}
            <For each={layers}>
                {(layer, index) => (
                    <div
                        style={{ "--row": String(index() + 1) }}
                        {...stylex.attrs(lattice.ruleRight, styles.claim)}
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
                        <Swap
                            row={index()}
                            isOpen={isOpen()}
                            today={layer.claim.today}
                            destack={layer.claim.destack}
                            style={styles.claimText}
                        />
                    </div>
                )}
            </For>

            {/* drift flotsam along the waterline, in front of the cards and under the water, once it has been calm a while */}
            <Flotsam
                isAdrift={isAdrift()}
                surfacedAt={surfacedAt()}
                waterline={`calc(${tokens.stage} * ${dryRows})`}
            />

            {/* draw both configurations in the six middle columns */}
            <div ref={drawing} {...stylex.attrs(lattice.ruleRight, styles.drawing)}>
                <canvas
                    ref={iceCanvas}
                    aria-hidden="true"
                    {...stylex.attrs(styles.ice, !isPainted() && styles.unpainted)}
                />

                {/* join the shared layers with one wire down the middle, then fan out to the hosts */}
                <Wire column={43.5} top={25} length={4} reveal={3} />
                <Wire column={43.5} top={34} length={4} reveal={4} />
                <Wire column={43.5} top={43} length={1.5} reveal={5} />
                <span style={{ opacity: "var(--reveal-5)" }} {...stylex.attrs(styles.fanBar)} />
                {columnLefts.map((left) => (
                    <Wire column={left + 10.5} top={44.5} length={2.5} reveal={5} />
                ))}

                {/* place every entity on its row and column */}
                {columnLefts.map((left, column) =>
                    locked.map((entity, index) => (
                        <div
                            style={{
                                left: `calc(${tokens.cell} * ${left})`,
                                top: `calc(${tokens.cellRow} * ${(dryRows + index) * 9 + 4.5})`,
                                opacity: `calc(1 - var(--reveal-${dryRows + index}))`,
                            }}
                            {...stylex.attrs(styles.column)}
                        >
                            <Card
                                entity={{ ...entity, role: `${owners[column]}'s` }}
                                kind="locked"
                                reveal={{ kind: "cipher" }}
                                style={styles.fill}
                            />
                        </div>
                    )),
                )}
                {tapes.map((tape, index) => (
                    <DuctTape
                        label={tape}
                        gap={index}
                        style={[
                            styles.tape,
                            index === 1 && styles.tapeRight,
                            isOpen() ? styles.tapeGone : styles.tapeBack,
                        ]}
                    />
                ))}
                {shared.map((entity, index) => (
                    <div
                        style={{
                            "--row": String(dryRows + index + 1),
                            "--cascade": `${820 + index * 220}ms`,
                            "clip-path": `inset(-2px calc((1 - var(--reveal-${dryRows + index})) * 50%))`,
                        }}
                        {...stylex.attrs(styles.band)}
                    >
                        <Band
                            entity={entity.entity}
                            items={entity.items}
                            reveals={entity.reveals}
                            active={isLive() ? traffic[scene()][index] : -1}
                        />
                    </div>
                ))}
                {/* move users, agents, and apps around freely over the layers below */}
                <Remix
                    isOpen={isOpen()}
                    isLive={isLive()}
                    revealOf={(row) => reveals[row]}
                    onScene={setScene}
                />

                {hosts.map((host, index) => (
                    <div
                        style={{
                            left: `calc(${tokens.cell} * ${columnLefts[index]})`,
                            top: `calc(${tokens.cellRow} * 49.5)`,
                            opacity: "var(--reveal-5)",
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

            {/* switch between today and Destack in the top right cell */}
            <button
                type="button"
                role="switch"
                aria-label="Destack"
                aria-checked={isOpen() ? "true" : "false"}
                onClick={() => select(isOpen() ? "today" : "destack")}
                {...stylex.attrs(styles.switch, isOpen() && styles.switchOn)}
            >
                <span aria-hidden="true" {...stylex.attrs(styles.switchName)}>
                    <span {...stylex.attrs(styles.stacked)}>
                        {[...stacked].map((letter, index) => (
                            <span
                                style={{
                                    "--lean": `${crooked[index][0]}deg`,
                                    "--sag": `${crooked[index][1]}px`,
                                    "transition-delay": `${index * 35}ms`,
                                    "animation-delay": `${-index * 0.41}s`,
                                }}
                                class={
                                    stylex.attrs(styles.wobbly, isOpen() && styles.tumbled).class
                                }
                            >
                                {letter}
                            </span>
                        ))}
                    </span>
                    <span {...stylex.attrs(styles.destacked, !isOpen() && styles.sunk)}>
                        DESTACK
                    </span>
                </span>
                <span
                    aria-hidden="true"
                    {...stylex.attrs(styles.track, isOpen() ? styles.trackOn : styles.trackOff)}
                >
                    <span
                        {...stylex.attrs(styles.knob, isOpen() ? styles.knobOn : styles.knobNudge)}
                    />
                </span>
                <span
                    aria-hidden="true"
                    {...stylex.attrs(styles.prompt, isOpen() && styles.hidden)}
                >
                    Destack it
                </span>
                <svg
                    aria-hidden="true"
                    viewBox="0 0 90 56"
                    {...stylex.attrs(styles.swoosh, isOpen() && styles.hidden)}
                >
                    <path
                        d="M10 45 C 34 48, 52 38, 54 20"
                        {...stylex.attrs(styles.swooshOutline)}
                    />
                    <path d="M10 45 C 34 48, 52 38, 54 20" {...stylex.attrs(styles.swooshShaft)} />
                    <path d="M54 7 L45 22 L63 22 Z" {...stylex.attrs(styles.swooshHead)} />
                </svg>
            </button>

            {/* name what each layer is made of */}
            <For each={layers.slice(1)}>
                {(layer, index) => (
                    <div style={{ "--row": String(index() + 2) }} {...stylex.attrs(styles.detail)}>
                        <Swap
                            row={index() + 1}
                            isOpen={isOpen()}
                            today={`0${index() + 2} ${layer.topic.today}`}
                            destack={`0${index() + 2} ${layer.topic.destack}`}
                            style={styles.number}
                        />
                        <Swap
                            row={index() + 1}
                            isOpen={isOpen()}
                            today={layer.detail.today}
                            destack={layer.detail.destack}
                            style={styles.detailText}
                        />
                    </div>
                )}
            </For>

            {/* stand in for the water until the shader paints its first frame */}
            <div
                aria-hidden="true"
                {...stylex.attrs(styles.pool, (isPainted() || isOpen()) && styles.poolGone)}
            />
            <canvas
                ref={canvas}
                aria-hidden="true"
                {...stylex.attrs(styles.water, !isPainted() && styles.unpainted)}
            />

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

/// Draw one wire down a column of grid cells, from a row for a length in cells, as its row drains.
function Wire(props: { column: number; top: number; length: number; reveal: number }) {
    return (
        <span
            style={{
                "--column": String(props.column),
                "--top": String(props.top),
                "--length": String(props.length),
                opacity: `var(--reveal-${props.reveal})`,
            }}
            {...stylex.attrs(styles.wire)}
        />
    );
}

/// Crossfade a row's text from today to Destack as the water leaves the row.
function Swap(props: {
    row: number;
    isOpen: boolean;
    today: string;
    destack: string;
    style?: stylex.Styles;
}) {
    const isSurface = props.row < dryRows;
    const reveal = `var(--reveal-${props.row})`;

    return (
        <span {...stylex.attrs(styles.swap, props.style)}>
            <span
                style={isSurface ? undefined : { opacity: `clamp(0, 1 - ${reveal} * 2, 1)` }}
                {...stylex.attrs(
                    styles.swapText,
                    isSurface && (props.isOpen ? styles.swapOut : styles.swapReturn),
                )}
            >
                {props.today}
            </span>
            <span
                style={isSurface ? undefined : { opacity: `clamp(0, ${reveal} * 2 - 1, 1)` }}
                {...stylex.attrs(
                    styles.swapText,
                    isSurface && (props.isOpen ? styles.swapIn : styles.swapLeave),
                )}
            >
                {props.destack}
            </span>
        </span>
    );
}

/// Return a layer number colour that lights up orange as the water leaves its row.
function numberOnWater(row: number): JSX.CSSProperties {
    if (row < dryRows) {
        return {};
    }

    return {
        color: `color-mix(in srgb, var(--destack-color-primary) calc(var(--reveal-${row}) * 100%), var(--destack-color-mutedForeground))`,
    };
}

const easing = "cubic-bezier(0.6, 0, 0.2, 1)";

const spring = "cubic-bezier(0.3, 1.3, 0.5, 1)";

const nudge = stylex.keyframes({
    "0%, 84%, 100%": { transform: "translateX(0)" },
    "90%": { transform: "translateX(0.25rem)" },
    "95%": { transform: "translateX(0.0625rem)" },
});

const wobble = stylex.keyframes({
    from: { transform: "rotate(-2deg) translateY(-0.5px)" },
    to: { transform: "rotate(2deg) translateY(0.5px)" },
});

const styles = stylex.create({
    figure: {
        gridTemplateRows: `repeat(6, ${tokens.stage})`,
        margin: 0,
        position: "relative",
        [mobile]: { gridTemplateRows: "none" },
    },
    claim: {
        display: "flex",
        flexDirection: "column",
        gap: "0.25rem",
        gridColumn: "1 / span 2",
        gridRow: "var(--row)",
        justifyContent: "center",
        paddingInline: "1rem",
        position: "relative",
        zIndex: 0,
        [mobile]: { display: "none" },
    },
    number: {
        color: color.mutedForeground,
        fontFamily: tokens.monoFont,
        fontSize: "0.6875rem",
        letterSpacing: "0.1em",
        lineHeight: "1rem",
        textTransform: "uppercase",
        transition: `color 300ms ${easing}`,
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
    },
    drawing: {
        display: "grid",
        gridColumn: "3 / span 8",
        gridRow: "1 / span 6",
        gridTemplateColumns: "repeat(3, minmax(0, 1fr))",
        gridTemplateRows: "repeat(6, minmax(0, 1fr))",
        minWidth: 0,
        position: "relative",
        [mobile]: {
            borderRightWidth: 0,
            gridColumn: "1 / -1",
            gridRow: "auto",
            gridTemplateRows: `repeat(6, ${tokens.stage})`,
            order: -1,
        },
    },
    fill: {
        width: "100%",
    },
    band: {
        alignItems: "center",
        display: "grid",
        gridColumn: "1 / -1",
        gridRow: "var(--row)",
        paddingInline: `calc(${tokens.cell} * 6)`,
        position: "relative",
        zIndex: 1,
    },

    hidden: {
        opacity: 0,
        pointerEvents: "none",
    },
    wire: {
        backgroundColor: color.primary,
        height: `calc(${tokens.cellRow} * var(--length))`,
        left: `calc(${tokens.cell} * var(--column) - 0.5px)`,
        position: "absolute",
        top: `calc(${tokens.cellRow} * var(--top))`,
        width: "1px",
        zIndex: 0,
    },
    fanBar: {
        backgroundColor: color.primary,
        height: "1px",
        left: `calc(${tokens.cell} * 16.5)`,
        position: "absolute",
        top: `calc(${tokens.cellRow} * 44.5)`,
        width: `calc(${tokens.cell} * 54)`,
        zIndex: 0,
    },
    column: {
        display: "flex",
        position: "absolute",
        transform: "translateY(-50%)",
        width: `calc(${tokens.cell} * 22)`,
        zIndex: 1,
    },
    tape: {
        left: `calc(${tokens.cell} * 30.5)`,
        top: `calc(${tokens.cellRow} * 13.5)`,
        [mobile]: { display: "none" },
    },
    tapeRight: {
        left: `calc(${tokens.cell} * 57.5)`,
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
    switchOn: {
        backgroundColor: tokens.signal,
        color: tokens.signalInk,
    },
    trackOff: {
        ":hover": { color: color.primary },
    },
    stacked: {
        color: color.foreground,
        display: "flex",
        fontFamily: '"Comic Sans MS", "Chalkboard SE", "Comic Neue", cursive',
        fontSize: "1rem",
        whiteSpace: "pre",
        fontWeight: 700,
        gridArea: "1 / 1",
        letterSpacing: "0.02em",
    },
    wobbly: {
        animationDirection: "alternate",
        animationDuration: "1.6s",
        animationIterationCount: "infinite",
        animationName: wobble,
        animationTimingFunction: "ease-in-out",
        display: "inline-block",
        rotate: "var(--lean)",
        transition: `opacity 300ms ${easing}, translate 420ms ${easing}, rotate 420ms ${easing}`,
        translate: "0 var(--sag)",
        [still]: { animationName: "none", transition: "none" },
    },
    tumbled: {
        animationName: "none",
        opacity: 0,
        rotate: "calc(var(--lean) * 5)",
        translate: "0 0.9em",
    },
    destacked: {
        fontFamily: tokens.posterFont,
        fontSize: "1.125rem",
        gridArea: "1 / 1",
        letterSpacing: "0.04em",
        transition: `opacity 360ms ${easing} 180ms, translate 480ms ${easing} 180ms`,
        [still]: { transition: "none" },
    },
    sunk: {
        opacity: 0,
        transitionDelay: "0ms",
        translate: "0 -0.6em",
    },
    prompt: {
        backgroundColor: tokens.signal,
        borderColor: tokens.signalInk,
        borderStyle: "solid",
        borderWidth: "1.5px",
        bottom: "-0.875rem",
        color: tokens.signalInk,
        fontFamily: tokens.monoFont,
        fontSize: "0.75rem",
        fontWeight: 700,
        letterSpacing: "0.1em",
        lineHeight: 1,
        padding: "0.375rem 0.5rem",
        position: "absolute",
        right: "calc(0.25rem + 80px)",
        textTransform: "uppercase",
        transform: "rotate(-2deg)",
        transition: `opacity 300ms ${easing}`,
        whiteSpace: "nowrap",
        zIndex: 3,
    },
    swoosh: {
        bottom: "-0.875rem",
        height: "56px",
        overflow: "visible",
        pointerEvents: "none",
        position: "absolute",
        right: "0.25rem",
        transition: `opacity 300ms ${easing}`,
        width: "90px",
        zIndex: 3,
    },
    swooshOutline: {
        fill: "none",
        stroke: tokens.signalInk,
        strokeLinecap: "round",
        strokeWidth: 7.5,
    },
    swooshShaft: {
        fill: "none",
        stroke: tokens.signal,
        strokeLinecap: "round",
        strokeWidth: 4.5,
    },
    swooshHead: {
        fill: tokens.signal,
        stroke: tokens.signalInk,
        strokeLinejoin: "round",
        strokeWidth: 1.5,
    },
    switchName: {
        display: "grid",
        whiteSpace: "nowrap",
    },
    track: {
        borderColor: "currentColor",
        borderStyle: "solid",
        borderWidth: "1.5px",
        display: "block",
        height: "1.625rem",
        position: "relative",
        width: "3rem",
    },
    trackOn: {
        borderColor: tokens.signalInk,
    },
    knob: {
        backgroundColor: "currentColor",
        height: "1rem",
        left: "0.1875rem",
        position: "absolute",
        top: "0.1875rem",
        transition: `transform 520ms ${spring}, background-color 300ms ${easing}`,
        width: "1rem",
        [still]: { transition: "none" },
    },
    knobNudge: {
        animationDuration: "4.5s",
        animationIterationCount: "infinite",
        animationName: nudge,
        [still]: { animationName: "none" },
    },
    knobOn: {
        transform: "translateX(1.375rem)",
    },
    switch: {
        alignItems: "center",
        backgroundColor: "transparent",
        borderWidth: 0,
        color: color.foreground,
        cursor: "pointer",
        display: "flex",
        gridColumn: "11 / span 2",
        gridRow: 1,
        justifyContent: "space-between",
        paddingInline: "1rem",
        position: "relative",
        transition: `background-color 300ms ${easing}, color 300ms ${easing}`,
        zIndex: 2,
        [mobile]: { gridColumn: "1 / -1", gridRow: "auto", minHeight: tokens.bar, order: -2 },
    },
    detail: {
        display: "flex",
        flexDirection: "column",
        gap: "0.25rem",
        gridColumn: "11 / span 2",
        gridRow: "var(--row)",
        justifyContent: "center",
        paddingInline: "1rem",
        position: "relative",
        zIndex: 0,
        [mobile]: { display: "none" },
    },
    detailText: {
        color: color.foreground,
        fontFamily: tokens.monoFont,
        fontSize: "0.8125rem",
        fontWeight: 600,
        letterSpacing: "0.02em",
        lineHeight: "1.375rem",
        translate: "0 0.125rem",
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
        top: `calc(100% * ${dryRows} / ${layers.length} - 1px)`,
        transition: `opacity 400ms ${easing}`,
        zIndex: 2,
        [mobile]: { display: "none" },
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
    sparks: {
        height: "100%",
        inset: 0,
        pointerEvents: "none",
        position: "absolute",
        width: "100%",
        zIndex: 3,
    },
    water: {
        height: "100%",
        inset: 0,
        pointerEvents: "none",
        position: "absolute",
        transition: `opacity 400ms ${easing}`,
        width: "100%",
        zIndex: 2,
    },
});
