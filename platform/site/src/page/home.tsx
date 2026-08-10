import { createMemo, createSignal, onCleanup, onMount } from "solid-js";

import { type HomeExample, homeExamples, installCommand } from "../content/site";
import { Seo } from "../site/seo";
import { highlightExample } from "../site/highlight";
import { Shell } from "../site/shell";
import { createSweep } from "../site/sweep";

/// The ordered Destack-token luminance ramp for the planet surface.
const surfaceGlyphRamp = ".:-|=+*&%#@";

/// The ordered Destack-token luminance ramp for the planet ring.
const ringGlyphRamp = "-=>";

/// The ordered luminance ramp for the sparse star field.
const starGlyphRamp = ".+*";

/// One slow deterministic ASCII twinkle cycle.
const starTwinkleCycle = "........+*+.........";

/// The directional Destack tokens repeated through successive ring rows.
const ringPatterns = ["->", "=>", ">>", ">>>"] as const;

/// The width of the moving ASCII light.
const asciiLightWidth = 14;

/// The horizontal light offset added per row.
const asciiLightSlope = 0.35;

/// The interval between ASCII light steps.
const asciiLightIntervalMilliseconds = 220;

/// The first shared sweep column.
const firstSweepColumn = -asciiLightWidth;

/// The horizontal planet offset within the shared sweep.
const planetSweepOffset = 18;

/// The width of the planet and star coordinate field.
const planetFieldWidth = 64;

/// The height of the planet and star coordinate field.
const planetFieldHeight = 32;

/// The final shared sweep column.
const lastSweepColumn = planetSweepOffset + planetFieldWidth + asciiLightWidth;

/// One fixed star in the planet coordinate field.
type Star = {
    /// The horizontal star coordinate.
    column: number;

    /// The star's offset within the shared twinkle cycle.
    phase: number;

    /// The vertical star coordinate.
    row: number;
};

/// The sparse deterministic stars surrounding the planet.
const stars: readonly Star[] = [
    { column: 5, phase: 0, row: 5 },
    { column: 16, phase: 5, row: 5 },
    { column: 8, phase: 10, row: 7 },
    { column: 3, phase: 15, row: 9 },
    { column: 7, phase: 3, row: 11 },
    { column: 3, phase: 8, row: 13 },
    { column: 5, phase: 13, row: 15 },
    { column: 1, phase: 18, row: 17 },
    { column: 61, phase: 1, row: 20 },
    { column: 57, phase: 6, row: 22 },
    { column: 3, phase: 11, row: 24 },
    { column: 58, phase: 16, row: 25 },
];

/// Properties supplied to the selected example preview.
type ExamplePreviewProps = {
    /// The selected example.
    example: HomeExample;
};

/// One public action identifier.
type Action = (typeof homeExamples)[number]["action"];

/// The first action shown when the URL does not select one.
const defaultExample = homeExamples[0];

/// The real Destack mark sampled into a fixed-width luminance field.
const planet = [
    "",
    "",
    "",
    "",
    "",
    "                          .::-------:..",
    "                     .:-+*#########****+=:.",
    "                   :=*########*************+-.",
    "                 :+############***************-........",
    "               .=############******************+*%##%###**=:",
    "              .*##############***********++++++++*######*#%%=.",
    "             .+++****************+++++++++++++++++=-==:=*:+@#:",
    "           . +******+++++++++++++++++++++++++++++++. *:=#+%%=",
    "          . :**********++++++++*****************+++==+*#%@#-",
    "            =*****+++++++**************++++++++++++*#%%%#-.",
    "          .-*********************************++**#%%@#+-.",
    "       .-+#@*+++++**********************+++***#%@@%#-.",
    "     .-#%@%#*+++++++++++++++++++++++++++**#%%@%#**+=",
    "    -#@%%*+=.+***++++++++++++++++++**#%%@@%%#*+++++: .",
    "   =%%+#=:*. .***********+++***##%%@@%%##**+++++++- .",
    "  :#@+.*=.==-:-+++******###%%%@%%%##**+++++++++++- .",
    "  .=%%**##*######%%%%%%%%%%%##**++++++++++++++++: .",
    "    :=*####%#####*#%###******+******++++++++++-. .",
    "         ......... .-+********+++++++++++++=:.  .",
    "                  .  .:-=++**++++++++++=-:.   .",
    "                    .    ..:::-----::..",
    "",
    "",
    "",
    "",
    "",
    "",
].join("\n");

/// The sampled ring separated from the bright page background.
const planetRing = [
    "",
    "",
    "",
    "",
    "",
    "",
    "",
    "",
    "",
    "                                                 %##%###**=:",
    "                                                  ######*#%%=",
    "                                                   -     :+@#:",
    "                                                     *:  +%%=",
    "                                                    =  #%@#-",
    "                                                    #%%%#-",
    "           -                                     #%%@#+-",
    "        -+#@                                  #%@@%",
    "      -#%@%#                              #%%@%",
    "    -#@%%*+=.                        #%%@@%",
    "   =%%+   *                     #%%@@%%",
    "  :#@+       :           ##%%%@%%",
    "   =%%**##*######%%%%%%%%%%",
    "    :=*####%#####*#",
    "",
    "",
    "",
    "",
    "",
    "",
    "",
    "",
    "",
].join("\n");

/// The ring mask filled with directional Destack operators.
const tokenizedPlanetRing = fillAsciiMask(planetRing, ringPatterns);

/// Render the public Destack homepage.
export function HomePage() {
    const sweep = createSweep({
        firstColumn: firstSweepColumn,
        lastColumn: lastSweepColumn,
        stepMilliseconds: asciiLightIntervalMilliseconds,
    });
    const [selectedAction, setSelectedAction] = createSignal<Action>(defaultExample.action);
    const selectedExample = createMemo(
        () =>
            homeExamples.find((example) => example.action === selectedAction()) ?? defaultExample,
    );

    // select URL-addressed actions after hydration and on navigation
    onMount(() => {
        const selectHash = () => {
            const action = window.location.hash.slice(1);
            const example = homeExamples.find((candidate) => candidate.action === action);

            setSelectedAction(example?.action ?? defaultExample.action);
        };

        selectHash();
        window.addEventListener("hashchange", selectHash);

        onCleanup(() => window.removeEventListener("hashchange", selectHash));
    });

    return (
        <Shell>
            <Seo
                description={
                    "Destack is a universal software engine for building complete " +
                    "software systems."
                }
            />

            <article
                class={
                    "mx-auto grid h-full min-h-0 w-full max-w-[min(var(--site-width),100vw)] " +
                    "grid-rows-[minmax(17rem,0.8fr)_minmax(25rem,1.2fr)] gap-4 " +
                    "pr-[var(--site-gutter-right)] pb-6 pl-[var(--site-gutter-left)] " +
                    "text-[length:var(--site-font-size)] max-md:h-auto max-md:grid-rows-[auto_auto] " +
                    "max-md:pr-[var(--site-gutter-right)] max-md:pb-4 " +
                    "max-md:pl-[var(--site-gutter-left)]"
                }
            >
                {/* Main pitch */}
                <section
                    class={
                        "grid min-h-0 min-w-0 grid-cols-[minmax(0,0.9fr)_minmax(24rem,1.1fr)] " +
                        "items-center gap-8 overflow-hidden pt-6 pb-3 " +
                        "max-[900px]:grid-cols-[minmax(0,1fr)_minmax(20rem,1fr)] " +
                        "max-[900px]:gap-6 max-md:grid-cols-1 max-md:gap-4 max-md:pt-9 max-md:pb-4"
                    }
                >
                    <div class="grid min-w-0 content-center gap-2">
                        <h1
                            class={
                                "m-0 text-[clamp(2rem,3.2vw,2.875rem)] leading-[1.02] " +
                                "font-[650] tracking-[-0.045em] text-destack-text"
                            }
                        >
                            <span class="block">absurdly integrated</span>
                            <span class="block">standardized</span>
                            <span class="block">computing stack</span>
                        </h1>
                        <p class="mt-1.5 mb-0 text-base leading-[1.4] font-semibold text-destack-text">
                            TypeScript++, web standards, familiar APIs
                        </p>
                        <ul class="mt-1.5 mb-0 grid list-none gap-1 p-0 leading-[1.45] text-destack-soft">
                            <li class="before:mr-2.5 before:text-destack-accent before:content-['•']">
                                sandboxed VM and true AOT native targets
                            </li>
                            <li class="before:mr-2.5 before:text-destack-accent before:content-['•']">
                                strong typing, tests, simulation, and debugging
                            </li>
                            <li class="before:mr-2.5 before:text-destack-accent before:content-['•']">
                                precise control over every host binding
                            </li>
                        </ul>

                        {/* Installation */}
                        <div
                            class={
                                "mt-3 flex min-w-0 items-stretch self-start border " +
                                "border-destack-frame font-mono text-xs leading-none"
                            }
                        >
                            <a
                                class={
                                    "flex shrink-0 items-center border-r border-destack-frame " +
                                    "px-3 py-2.5 font-semibold text-destack-text no-underline " +
                                    "hover:text-destack-accent"
                                }
                                href="/docs/"
                            >
                                get started
                            </a>
                            <code class="min-w-0 overflow-hidden px-3 py-2.5 text-ellipsis whitespace-nowrap text-destack-soft">
                                {installCommand}
                            </code>
                        </div>
                    </div>

                    <AsciiPlanet
                        lightColumn={sweep.column() - planetSweepOffset}
                        onRelease={sweep.release}
                        onSteer={(column) => sweep.steer(column + planetSweepOffset)}
                        phase={sweep.phase()}
                    />
                </section>

                {/* Product workbench */}
                <section
                    aria-label="Destack actions"
                    class="grid min-h-0 min-w-0 content-start grid-rows-[auto_minmax(0,1fr)]"
                >
                    <nav
                        aria-label="Destack action previews"
                        class="min-w-0 overflow-x-auto border-y border-destack-frame"
                    >
                        <ol class="m-0 flex w-max min-w-full list-none p-0">
                            {homeExamples.map((example, index) => (
                                <li class="flex-[1_0_auto]" id={example.action}>
                                    <a
                                        aria-current={
                                            selectedAction() === example.action
                                                ? "location"
                                                : undefined
                                        }
                                        href={`#${example.action}`}
                                        onClick={() => setSelectedAction(example.action)}
                                        data-shortcut={String(index)}
                                        title={`Alt+${index}: ${example.action}`}
                                        class={
                                            "flex min-h-[var(--site-control-height)] " +
                                            "items-center justify-center border-b-2 " +
                                            "border-transparent px-3 py-2 font-mono text-xs " +
                                            "leading-none text-destack-soft hover:text-destack-text " +
                                            "aria-[current=location]:border-destack-accent " +
                                            "aria-[current=location]:text-destack-text"
                                        }
                                    >
                                        <strong class="font-semibold">{example.action}</strong>
                                    </a>
                                </li>
                            ))}
                        </ol>
                    </nav>

                    <ExamplePreview example={selectedExample()} />
                </section>
            </article>
        </Shell>
    );
}

/// Render the lightweight selected example placeholder.
function ExamplePreview(props: ExamplePreviewProps) {
    return (
        <section
            aria-live="polite"
            class={
                "mt-4 grid min-h-0 grid-rows-[auto_minmax(0,1fr)] overflow-hidden rounded-sm " +
                "border border-destack-frame bg-destack-panel max-md:min-h-[22rem]"
            }
        >
            <header
                class={
                    "grid grid-cols-[7rem_minmax(0,1fr)] items-baseline gap-4 " +
                    "border-b border-destack-frame px-4 py-3.5 " +
                    "max-[900px]:grid-cols-[5rem_minmax(0,1fr)] " +
                    "max-md:grid-cols-1 max-md:gap-1"
                }
            >
                <h2 class="m-0 font-mono text-[0.8125rem] leading-[1.4] font-semibold">
                    {props.example.action}
                </h2>
                <p class="m-0 leading-[1.4] text-destack-soft">
                    {props.example.description}
                </p>
            </header>

            <ExampleCode example={props.example} />
        </section>
    );
}

/// Render one numbered static code listing.
function ExampleCode(props: { example: HomeExample }) {
    const lines = createMemo(() =>
        highlightExample(props.example.source.code, props.example.source.language),
    );

    return (
        <section class="grid min-h-0 grid-rows-[auto_minmax(0,1fr)]">
            <header
                class={
                    "flex items-center justify-between border-b border-destack-frame " +
                    "px-4 py-2 font-mono text-xs text-destack-soft"
                }
            >
                <span class="text-destack-text">{props.example.source.title}</span>
                <span>{props.example.source.language}</span>
            </header>

            <div class="syntax min-h-0 overflow-auto py-4 font-mono text-[0.8125rem] leading-6">
                <ol aria-label={props.example.source.title} class="m-0 list-none p-0">
                    {lines().map((line, index) => (
                        <li class="grid min-w-max grid-cols-[3.25rem_minmax(0,1fr)]">
                            <span
                                aria-hidden="true"
                                class={
                                    "border-r border-destack-frame pr-3 text-right " +
                                    "text-destack-soft select-none"
                                }
                            >
                                {index + 1}
                            </span>
                            <code
                                class="px-4 whitespace-pre text-destack-text"
                                innerHTML={line || " "}
                            />
                        </li>
                    ))}
                </ol>

                {/* Result */}
                <div
                    class={
                        "mt-3 grid min-w-max grid-cols-[3.25rem_minmax(0,1fr)] " +
                        "text-destack-text"
                    }
                >
                    <span
                        aria-hidden="true"
                        class="pr-3 text-right text-destack-accent select-none"
                    >
                        →
                    </span>
                    <div class="flex min-w-0 items-baseline gap-3 px-4">
                        <span class="shrink-0 text-destack-soft">{props.example.result.title}</span>
                        <pre class="m-0 whitespace-pre-wrap">{props.example.result.text}</pre>
                    </div>
                </div>
            </div>
        </section>
    );
}

/// Render the actual Destack mark through sampled ASCII characters.
function AsciiPlanet(props: {
    lightColumn: number;
    onRelease: () => void;
    onSteer: (column: number) => void;
    phase: number;
}) {
    // map pointer movement into the shared fixed-width ASCII field
    const steer = (event: PointerEvent & { currentTarget: HTMLDivElement }) => {
        // preserve native touch scrolling
        if (event.pointerType === "touch") {
            return;
        }

        const bounds = event.currentTarget.getBoundingClientRect();
        const progress = (event.clientX - bounds.left) / bounds.width;

        props.onSteer(progress * planetFieldWidth);
    };

    return (
        <figure
            aria-label="The Destack ringed planet"
            class="ascii-planet m-0 grid w-[min(100%,27rem)] min-w-0 justify-self-end justify-items-center overflow-hidden max-md:justify-self-center"
            role="img"
        >
            <div
                aria-hidden="true"
                class="ascii-planet__art"
                onPointerLeave={props.onRelease}
                onPointerMove={steer}
            >
                <pre class="ascii-planet__stars">
                    {illuminateAscii(
                        renderAsciiStars(props.phase),
                        props.lightColumn,
                        starGlyphRamp,
                    )}
                </pre>
                <pre class="ascii-planet__surface">
                    {illuminateAscii(planet, props.lightColumn, surfaceGlyphRamp)}
                </pre>
                <pre class="ascii-planet__ring">
                    {illuminateAscii(tokenizedPlanetRing, props.lightColumn, ringGlyphRamp)}
                </pre>
            </div>
        </figure>
    );
}

/// Render the fixed stars into the planet coordinate field.
function renderAsciiStars(phase: number) {
    const field = Array.from({ length: planetFieldHeight }, () =>
        Array.from({ length: planetFieldWidth }, () => " "),
    );

    // place every authored star at its stable coordinate
    for (const star of stars) {
        const cycleIndex = (phase + star.phase) % starTwinkleCycle.length;
        field[star.row][star.column] = starTwinkleCycle[cycleIndex];
    }

    return field.map((row) => row.join("")).join("\n");
}

/// Fill each visible mask segment with one repeating token pattern.
function fillAsciiMask(source: string, patterns: readonly [string, ...string[]]) {
    return source
        .split("\n")
        .map((line, row) => {
            const pattern = patterns[row % patterns.length];
            let segmentColumn = 0;

            return Array.from(line, (character) => {
                if (character === " ") {
                    segmentColumn = 0;

                    return character;
                }

                const replacement = pattern[segmentColumn % pattern.length];
                segmentColumn += 1;

                return replacement;
            }).join("");
        })
        .join("\n");
}

/// Raise glyph density within one diagonal moving light.
function illuminateAscii(source: string, lightColumn: number, glyphRamp: string) {
    return source
        .split("\n")
        .map((line, row) => {
            const rowLightColumn = lightColumn - row * asciiLightSlope;

            return Array.from(line, (character, column) => {
                const index = glyphRamp.indexOf(character);
                const distance = Math.abs(column - rowLightColumn);

                if (index < 0 || distance >= asciiLightWidth) {
                    return character;
                }

                const isCenter = distance < asciiLightWidth / 3;
                const brightness = isCenter ? 2 : 1;
                const nextIndex = Math.min(glyphRamp.length - 1, index + brightness);

                return glyphRamp[nextIndex];
            }).join("");
        })
        .join("\n");
}
