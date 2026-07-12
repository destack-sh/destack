import { createMemo, createSignal, onCleanup, onMount } from "solid-js";

import { homePoints } from "../content/site";
import { Seo } from "../site/seo";
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

/// The final shared sweep column.
const lastSweepColumn = 110;

/// The horizontal planet offset within the shared sweep.
const planetSweepOffset = 18;

/// The width of the planet and star coordinate field.
const planetFieldWidth = 64;

/// The height of the planet and star coordinate field.
const planetFieldHeight = 32;

/// The width of the upper-right nebular field.
const nebulaFieldWidth = 72;

/// The height of the upper-right nebular field.
const nebulaFieldHeight = 15;

/// The number of shared steps between nebular flow changes.
const nebulaPhaseDivisor = 3;

/// The deterministic glyph currents flowing through each filament.
const nebulaPatterns = [".  -~  +*  ~-  ", "  .  --~  +  . "] as const;

/// The width of the ASCII lake.
const lakeWidth = 140;

/// The horizontal inset of each successive lake row.
const lakeRowInsets = [0, 8, 18, 30, 42] as const;

/// The horizontal lake center beneath the planet.
const lakeCenterRatio = 0.72;

/// The interval between small breaks in each lake row.
const lakeBreakInterval = 37;

/// The ordered Destack-token luminance ramp for the lake.
const lakeGlyphRamp = ".-~|=&#";

/// The Destack token patterns repeated through successive lake rows.
const lakePatterns = [
    "... ..= ... ",
    "..=~==||",
    "~==||..=",
    "||==~..=",
    "..=||==~~",
] as const;

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

/// Properties supplied to one lower action section.
type ActionSectionProps = {
    /// The selected action.
    action: string;

    /// The selected action description.
    description: string;

    /// The displayed action number.
    number: string;
};

/// One public action identifier.
type Action = (typeof homePoints)[number]["action"];

/// The first non-install action shown when the URL does not select one.
const defaultPoint = homePoints.find((point) => point.action !== "install") ?? homePoints[0];

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
    const [selectedAction, setSelectedAction] = createSignal<Action>(defaultPoint.action);
    const selectedPoint = createMemo(
        () => homePoints.find((point) => point.action === selectedAction()) ?? defaultPoint,
    );
    const selectedIndex = createMemo(() => homePoints.indexOf(selectedPoint()));

    // select URL-addressed actions after hydration and on navigation
    onMount(() => {
        const selectHash = () => {
            const action = window.location.hash.slice(1);
            const point = homePoints.find((candidate) => candidate.action === action);

            setSelectedAction(point?.action ?? defaultPoint.action);
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

            <article class="home-page">
                {/* Main pitch */}
                <section class="home-hero">
                    {/* Nebular field */}
                    <AsciiNebula phase={sweep.phase()} />

                    <div class="home-hero__body">
                        <div class="home-hero__copy">
                            <div class="home-hero__heading">
                                <p class="home-hero__label">[destack]</p>
                                <p class="home-hero__statement">
                                    the absurdly integrated computing stack
                                </p>
                            </div>

                            <ol class="home-points">
                                {homePoints.map((point, index) => (
                                    <li>
                                        <a
                                            aria-current={
                                                selectedAction() === point.action
                                                    ? "location"
                                                    : undefined
                                            }
                                            href={`#${point.action}`}
                                            onClick={() => setSelectedAction(point.action)}
                                            data-shortcut={String(index)}
                                            title={`Alt+${index}: ${point.action}`}
                                        >
                                            <span class="home-points__number">
                                                {pointNumber(
                                                    index,
                                                    selectedAction() === point.action,
                                                )}
                                            </span>
                                            <strong class="home-points__action">
                                                [{point.action}]
                                            </strong>
                                            <span>{point.description}</span>
                                        </a>
                                    </li>
                                ))}
                            </ol>
                        </div>

                        <AsciiPlanet
                            lightColumn={sweep.column() - planetSweepOffset}
                            onRelease={sweep.release}
                            onSteer={(column) => sweep.steer(column + planetSweepOffset)}
                            phase={sweep.phase()}
                        />

                        <AsciiLake
                            lightColumn={projectLakeColumn(sweep.trailColumn())}
                            onRelease={sweep.release}
                            onSteer={(column) => sweep.steer(unprojectLakeColumn(column))}
                        />
                    </div>
                </section>

                {/* Selected action */}
                <section class="home-stage">
                    {homePoints.map((point) => (
                        <span
                            aria-hidden="true"
                            class="home-stage__anchor"
                            id={point.action}
                        />
                    ))}

                    <PlaceholderSection
                        action={selectedPoint().action}
                        description={selectedPoint().description}
                        number={String(selectedIndex()).padStart(2, "0")}
                    />
                </section>
            </article>
        </Shell>
    );
}

/// Render the sparse upper-right nebular field.
function AsciiNebula(props: { phase: number }) {
    return (
        <pre aria-hidden="true" class="ascii-nebula">
            {renderAsciiNebula(props.phase)}
        </pre>
    );
}

/// Render one deterministic frame of two flowing nebular filaments.
function renderAsciiNebula(phase: number) {
    const flowPhase = Math.floor(phase / nebulaPhaseDivisor);

    return Array.from({ length: nebulaFieldHeight }, (_, row) => {
        const firstCenter = 14 + 2 * row + 3 * Math.sin(0.65 * row);
        const secondCenter = 33 + 0.8 * row + 5 * Math.sin(0.45 * row + 1.4);

        return Array.from({ length: nebulaFieldWidth }, (_, column) => {
            const firstDistance = Math.abs(column - firstCenter);
            const secondDistance = Math.abs(column - secondCenter);
            const isFirst = firstDistance <= secondDistance;
            const distance = isFirst ? firstDistance : secondDistance;

            // keep each current narrow and leave most of the field empty
            if (distance > 3.2) {
                return " ";
            }

            // translate glyphs along the stable filament geometry
            const pattern = nebulaPatterns[isFirst ? 0 : 1];
            const patternIndex = wrapIndex(column + 3 * row - flowPhase, pattern.length);
            const glyph = pattern[patternIndex];

            return glyph === " " && distance < 1.2 ? "." : glyph;
        }).join("");
    }).join("\n");
}

/// Wrap one possibly negative value into an array index.
function wrapIndex(value: number, length: number) {
    return ((value % length) + length) % length;
}

/// Format one action index, marking the selected row in place.
function pointNumber(index: number, isSelected: boolean) {
    const number = String(index).padStart(2, "0");

    return isSelected ? `>${number.slice(1)}` : number;
}

/// Render an empty lower section for an action.
function PlaceholderSection(props: ActionSectionProps) {
    return (
        <section class="home-action-section">
            <header class="home-action-section__header">
                [{props.number}:{props.action}]
            </header>
            <div aria-hidden="true" class="home-action-section__body">
                <span>...</span>
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
        <figure aria-label="The Destack ringed planet" class="ascii-planet" role="img">
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

/// Render the page-wide ASCII lake beneath the hero.
function AsciiLake(props: {
    lightColumn: number;
    onRelease: () => void;
    onSteer: (column: number) => void;
}) {
    // map pointer movement into the wider lake field
    const steer = (event: PointerEvent & { currentTarget: HTMLPreElement }) => {
        // preserve native touch scrolling
        if (event.pointerType === "touch") {
            return;
        }

        const bounds = event.currentTarget.getBoundingClientRect();
        const progress = (event.clientX - bounds.left) / bounds.width;

        props.onSteer(progress * lakeWidth);
    };

    return (
        <pre
            aria-hidden="true"
            class="ascii-lake"
            onPointerLeave={props.onRelease}
            onPointerMove={steer}
        >
            {renderAsciiLake(props.lightColumn)}
        </pre>
    );
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

/// Project the shared sweep onto the wider lake field.
function projectLakeColumn(column: number) {
    const sweepWidth = lastSweepColumn - firstSweepColumn;
    const progress = (column - firstSweepColumn) / sweepWidth;
    const fieldWidth = lakeWidth + 2 * asciiLightWidth;

    return progress * fieldWidth - asciiLightWidth;
}

/// Project a lake field coordinate back onto the shared sweep.
function unprojectLakeColumn(column: number) {
    const fieldWidth = lakeWidth + 2 * asciiLightWidth;
    const progress = (column + asciiLightWidth) / fieldWidth;
    const sweepWidth = lastSweepColumn - firstSweepColumn;

    return progress * sweepWidth + firstSweepColumn;
}

/// Render one deterministic lake frame with tapered depth.
function renderAsciiLake(lightColumn: number) {
    const centerColumn = Math.floor(lakeWidth * lakeCenterRatio);

    return lakeRowInsets.map((inset, row) => {
        const rowWidth = lakeWidth - 2 * inset;
        const centeredStart = Math.round(centerColumn - rowWidth / 2);
        const startColumn = Math.max(0, Math.min(lakeWidth - rowWidth, centeredStart));
        const endColumn = startColumn + rowWidth;
        const pattern = lakePatterns[row];

        return Array.from({ length: lakeWidth }, (_, column) => {
            if (column < startColumn || column >= endColumn) {
                return " ";
            }

            const rowColumn = column - startColumn;
            const breakOffset = (rowColumn + 11 * row) % lakeBreakInterval;

            if (breakOffset <= row % 2) {
                return " ";
            }

            const character = pattern[rowColumn % pattern.length];
            const patternIndex = lakeGlyphRamp.indexOf(character);

            if (patternIndex < 0) {
                return character;
            }

            const lightDistance = Math.abs(column - lightColumn);
            const isLit = lightDistance < 2 * asciiLightWidth;
            const brightness = isLit ? 2 : 0;
            const nextIndex = Math.min(lakeGlyphRamp.length - 1, patternIndex + brightness);

            return lakeGlyphRamp[nextIndex];
        }).join("");
    }).join("\n");
}
