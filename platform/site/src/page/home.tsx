import { createMemo, createSignal, onCleanup, onMount, type Component } from "solid-js";
import { Dynamic } from "solid-js/web";

import { Seo } from "../component/seo";
import { Shell } from "../component/shell";
import { createSweep, SweepText } from "../component/sweep";

/// The ordered ASCII luminance ramp.
const asciiRamp = ".:-=+*#%@";

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

/// The first sweep column assigned to a capability action.
const actionSweepFirstColumn = 20;

/// The sweep distance between successive capability actions.
const actionSweepStride = 10;

/// The width of the ASCII lake.
const lakeWidth = 140;

/// The horizontal inset of each successive lake row.
const lakeRowInsets = [0, 8, 18, 30, 42] as const;

/// The horizontal lake center beneath the planet.
const lakeCenterRatio = 0.72;

/// The interval between small breaks in each lake row.
const lakeBreakInterval = 37;

/// The ordered lake character ramp.
const lakeRamp = "~-=+#";

/// The public installation command.
const installCommand = "curl -fsSL https://destack.sh/install | sh";

/// Properties supplied to one lower action section.
type ActionSectionProps = {
    /// The selected action.
    action: string;

    /// The selected action description.
    description: string;

    /// The displayed action number.
    number: string;
};

/// One linked product action and its lower section.
type Point = {
    /// The action identifier.
    action: string;

    /// The short action description.
    description: string;

    /// The lower section rendered for the action.
    section: Component<ActionSectionProps>;
};

/// The current Destack product outline.
const points = [
    {
        action: "build",
        description: "correct and optimal full stack software",
        section: PlaceholderSection,
    },
    {
        action: "write",
        description: "familiar TypeScript, Node, and Web code",
        section: PlaceholderSection,
    },
    {
        action: "read",
        description: "idiomatic modern TS + Rust-y features",
        section: PlaceholderSection,
    },
    {
        action: "compile",
        description: "to sandboxed VM and true AOT native targets",
        section: PlaceholderSection,
    },
    {
        action: "control",
        description: "precise access over every host binding",
        section: PlaceholderSection,
    },
    {
        action: "check",
        description: "with strong types and userland lints",
        section: PlaceholderSection,
    },
    {
        action: "test",
        description: "every byte and cycle of your systems",
        section: PlaceholderSection,
    },
    {
        action: "simulate",
        description: "the entire application end-to-end",
        section: PlaceholderSection,
    },
    {
        action: "debug",
        description: "backward, in parallel or slow motion",
        section: PlaceholderSection,
    },
] as const satisfies readonly Point[];

/// One public action identifier.
type Action = (typeof points)[number]["action"];

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

/// Render the public Destack homepage.
export function HomePage() {
    const sweepColumn = createSweep({
        firstColumn: firstSweepColumn,
        lastColumn: lastSweepColumn,
        stepMilliseconds: asciiLightIntervalMilliseconds,
    });
    const [selectedAction, setSelectedAction] = createSignal<Action>(points[0].action);
    const selectedPoint = createMemo(
        () => points.find((point) => point.action === selectedAction()) ?? points[0],
    );
    const selectedIndex = createMemo(() => points.indexOf(selectedPoint()));

    // select URL-addressed actions after hydration and on navigation
    onMount(() => {
        const selectHash = () => {
            const action = window.location.hash.slice(1);
            const point = points.find((candidate) => candidate.action === action);

            setSelectedAction(point?.action ?? points[0].action);
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
                    <div class="home-hero__heading">
                        <p class="home-hero__label">
                            <SweepText
                                column={sweepColumn()}
                                firstColumn={0}
                                text="[destack]"
                            />
                        </p>
                        <p class="home-hero__statement">
                            the absurdly integrated computing stack
                        </p>
                    </div>

                    <div class="home-hero__body">
                        <div class="home-hero__copy">
                            <ol class="home-points">
                                {points.map((point, index) => (
                                    <li>
                                        <a
                                            aria-current={
                                                selectedAction() === point.action
                                                    ? "location"
                                                    : undefined
                                            }
                                            href={`#${point.action}`}
                                            onClick={() => setSelectedAction(point.action)}
                                        >
                                            <span class="home-points__number">
                                                {String(index + 1).padStart(2, "0")}
                                            </span>
                                            <strong class="home-points__action">
                                                <SweepText
                                                    column={sweepColumn()}
                                                    firstColumn={
                                                        actionSweepFirstColumn +
                                                        index * actionSweepStride
                                                    }
                                                    text={`[${point.action}]`}
                                                />
                                            </strong>
                                            <span>{point.description}</span>
                                        </a>
                                    </li>
                                ))}
                            </ol>

                            {/* Installation */}
                            <div class="home-install" id="install">
                                <code>
                                    <SweepText
                                        class="home-install__prompt"
                                        column={sweepColumn()}
                                        firstColumn={lastSweepColumn - 5}
                                        text="$"
                                    />
                                    <span>{installCommand}</span>
                                </code>
                            </div>
                        </div>

                        <AsciiPlanet lightColumn={sweepColumn() - planetSweepOffset} />
                    </div>

                    <AsciiLake lightColumn={projectLakeColumn(sweepColumn())} />
                </section>

                {/* Selected action */}
                <section class="home-stage">
                    {points.map((point) => (
                        <span
                            aria-hidden="true"
                            class="home-stage__anchor"
                            id={point.action}
                        />
                    ))}

                    <Dynamic
                        action={selectedPoint().action}
                        component={selectedPoint().section}
                        description={selectedPoint().description}
                        number={String(selectedIndex() + 1).padStart(2, "0")}
                    />
                </section>
            </article>
        </Shell>
    );
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
                    {illuminateAscii(planet, props.lightColumn)}
                </pre>
                <pre class="ascii-planet__ring">
                    {illuminateAscii(planetRing, props.lightColumn)}
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

/// Raise glyph density within one diagonal moving light.
function illuminateAscii(source: string, lightColumn: number) {
    return source
        .split("\n")
        .map((line, row) => {
            const rowLightColumn = lightColumn - row * asciiLightSlope;

            return Array.from(line, (character, column) => {
                const index = asciiRamp.indexOf(character);
                const distance = Math.abs(column - rowLightColumn);

                if (index < 0 || distance >= asciiLightWidth) {
                    return character;
                }

                const isCenter = distance < asciiLightWidth / 3;
                const brightness = isCenter ? 2 : 1;
                const nextIndex = Math.min(asciiRamp.length - 1, index + brightness);

                return asciiRamp[nextIndex];
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

        return Array.from({ length: lakeWidth }, (_, column) => {
            if (column < startColumn || column >= endColumn) {
                return " ";
            }

            const rowColumn = column - startColumn;
            const breakOffset = (rowColumn + 11 * row) % lakeBreakInterval;

            if (breakOffset <= row % 2) {
                return " ";
            }

            const patternIndex = (Math.floor(rowColumn / 6) + 2 * row) % 3;
            const lightDistance = Math.abs(column - lightColumn);
            const isLit = lightDistance < 2 * asciiLightWidth;
            const brightness = isLit ? 2 : 0;
            const nextIndex = Math.min(lakeRamp.length - 1, patternIndex + brightness);

            return lakeRamp[nextIndex];
        }).join("");
    }).join("\n");
}
