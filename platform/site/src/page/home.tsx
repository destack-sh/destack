import { createMemo, createSignal, onCleanup, onMount } from "solid-js";

import { homePoints } from "../content/site";
import { Seo } from "../site/seo";
import { Shell } from "../site/shell";
import { createSweep } from "../site/sweep";

/// The ordered Destack-token luminance ramp for the planet surface.
const surfaceGlyphRamp = ".:-|=+*&%#@";

/// The ordered Destack-token luminance ramp for the planet ring.
const ringGlyphRamp = "-=>";

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
    const sweepColumn = createSweep({
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

                        <AsciiPlanet lightColumn={sweepColumn() - planetSweepOffset} />
                    </div>

                    <AsciiLake lightColumn={projectLakeColumn(sweepColumn())} />
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
function AsciiPlanet(props: { lightColumn: number }) {
    return (
        <figure aria-label="The Destack ringed planet" class="ascii-planet" role="img">
            <div aria-hidden="true" class="ascii-planet__art">
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

/// Render the page-wide ASCII lake beneath the hero.
function AsciiLake(props: { lightColumn: number }) {
    return (
        <pre aria-hidden="true" class="ascii-lake">
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
