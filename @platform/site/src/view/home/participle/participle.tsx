import { color, font, stroke } from "@destack/theme/tokens.stylex";
import { media } from "@destack/style/media.stylex";
import * as style from "@destack/style";
import { createSignal, For } from "@destack/view";

import { lattice } from "../../layout/lattice.stylex";
import { palette } from "../../palette.stylex";
import { screen } from "../../layout/screen.stylex";
import { Entry, FigureLabel, type Form } from "../entry/entry";
import { Label } from "../entry/label";
import { StackSwitch } from "../entry/switch";
import { AppRail, appStyles, appText } from "../figure/app";
import { browserStyles } from "../figure/browser";
import { rentedAutomationOf } from "./rented";
import { Favicon, Glyph } from "../figure/glyph";
import { accentOf, type ItemMark } from "../figure/item";
import { type Cell, Ledger, type LedgerRow } from "../figure/ledger";
import { ServiceGlyph } from "../figure/tile";
import { paperGlyphOf } from "../figure/planet";
import { durationOf, type Flow, flows, pathOf, type Step, type Tool, tools, totals } from "./trace";

/** The step the trace opens on: Notetaker reading the event it was told about. */
const firstStep = 4;

/** The participle: every app working as one, in plain sight. */
const participle: Form = {
    syllables: ["de", "stack", "ing"],
    pronunciation: "/diːˈstakɪŋ/",
    partOfSpeech: "present participle",
    sense: {
        definition: "every app working as one, in plain sight",
        highlight: ["as one", "plain sight"],
        sentence: "Any app, agent or automation can act on anything, and you can see why.",
    },
};

/** Show the participle at work: automations spread over rented tools, then traced in your space. */
export function Participle(properties: { isOpen: boolean }) {
    // hold the row looked at, shared by both states, and the step of its trace
    const [row, setRow] = createSignal(0);
    const [selected, setSelected] = createSignal(firstStep);
    const choose = (index: number) => {
        setRow(index);
        setSelected(index === 0 ? firstStep : 0);
    };

    return (
        <section
            {...style.attrs(
                lattice.frame,
                lattice.ruled,
                lattice.section,
                lattice.ruleBottom,
                styles.section,
            )}
        >
            <Entry form={participle} />
            <FigureLabel
                number={5}
                isOpen={properties.isOpen}
                stacked="eight tools, glued by hand"
                destacked="one space, working as one"
            >
                <StackSwitch isOpen={properties.isOpen} place="label" />
            </FigureLabel>
            <div {...style.attrs(lattice.cell, lattice.figureCell, styles.stage)}>
                <Label>
                    {properties.isOpen
                        ? "Map, an app in your space"
                        : `${toolAt(row()).name}, ${toolAt(row()).page.toLowerCase()}`}
                </Label>
                {properties.isOpen ? (
                    <MapApp
                        flow={flowAt(row())}
                        selected={selected()}
                        onSelect={setSelected}
                        onOpen={choose}
                    />
                ) : (
                    <Browser tool={toolAt(row())} onOpen={choose} />
                )}
            </div>
            <div {...style.attrs(lattice.cell, lattice.figureCell, styles.key)}>
                <Label>
                    {properties.isOpen ? "Every automation you own" : "Every automation you rent"}
                </Label>
                <Automations isOpen={properties.isOpen} row={row()} onChoose={choose} />
            </div>
        </section>
    );
}

/** Return the tool at a row, or refuse a row the table lacks. */
function toolAt(index: number): Tool {
    const tool = tools[index];
    if (tool === undefined) {
        throw new Error(`no tool at row ${String(index)}`);
    }

    return tool;
}

/** Return the flow at a row, or refuse a row the table lacks. */
function flowAt(index: number): Flow {
    const flow = flows[index];
    if (flow === undefined) {
        throw new Error(`no flow at row ${String(index)}`);
    }

    return flow;
}

/** Draw a tool's own history page in a browser with a tab for every tool you rent. */
function Browser(properties: { tool: Tool; onOpen: (index: number) => void }) {
    // split the page's address
    const host = () => properties.tool.url.split("/")[0] ?? "";

    return (
        <div data-component="Browser" {...style.attrs(browserStyles.browser, styles.browser)}>
            {/* keep a tab open for every tool */}
            <div data-component="TabStrip" {...style.attrs(browserStyles.strip)}>
                <span aria-hidden="true" {...style.attrs(browserStyles.lights)} />
                <For each={tools}>
                    {(tool, index) => (
                        <span
                            data-component="Tab"
                            title={tool.page}
                            onClick={() => properties.onOpen(index())}
                            {...style.attrs(
                                browserStyles.tab,
                                browserStyles.tabEven,
                                isTool(tool, properties.tool) && browserStyles.tabOn,
                            )}
                        >
                            <Mark mark={tool.mark} />
                            <span
                                {...style.attrs(
                                    browserStyles.tabTitle,
                                    isOther(tool, properties.tool) && browserStyles.tabTitleHidden,
                                )}
                            >
                                {tool.page}
                            </span>
                        </span>
                    )}
                </For>
            </div>

            {/* show the page's address */}
            <div data-component="Toolbar" {...style.attrs(browserStyles.toolbar)}>
                <span {...style.attrs(browserStyles.tools, browserStyles.desktopOnly)}>
                    <Glyph name="back" />
                    <Glyph name="forward" />
                    <Glyph name="reload" />
                </span>
                <span data-component="AddressBar" {...style.attrs(browserStyles.omnibox)}>
                    <Glyph name="lock" size={12} />
                    <span {...style.attrs(browserStyles.url)}>
                        <span {...style.attrs(browserStyles.host)}>{host()}</span>
                        <span {...style.attrs(browserStyles.path)}>
                            {properties.tool.url.slice(host().length)}
                        </span>
                    </span>
                </span>
            </div>

            {/* draw the tool's own page, as the tool draws it */}
            <div {...style.attrs(browserStyles.viewport, styles.viewport)}>
                <div {...style.attrs(styles.rentedWindow)}>
                    {rentedAutomationOf(properties.tool.name)}
                </div>
            </div>
        </div>
    );
}

/** Report whether a tool is other than the one open. */
function isOther(tool: Tool, open: Tool): boolean {
    return tool.name !== open.name;
}

/** Report whether two tools are the same. */
function isTool(tool: Tool, other: Tool): boolean {
    return tool.name === other.name;
}

/** Draw a mark small: an app's tile, a vendor's icon, or a vendor's logo. */
function Mark(properties: { mark: ItemMark }) {
    return (
        <>
            {"app" in properties.mark ? (
                <ServiceGlyph icon={properties.mark.app} />
            ) : "icon" in properties.mark ? (
                <Favicon icon={properties.mark.icon} tint={properties.mark.tint} size={14} />
            ) : null}
        </>
    );
}

/** Report whether an index is the first. */
function isFirst(index: number): boolean {
    return index === 0;
}

/** Draw the Map app: a flow's traced run, its apps, and why each step was allowed. */
function MapApp(properties: {
    flow: Flow;
    selected: number;
    onSelect: (index: number) => void;
    onOpen: (index: number) => void;
}) {
    // read the run's length and the ticks along its time
    const duration = () => durationOf(properties.flow);
    const ticks = () => [0, 1, 2, 3].map((part) => Math.round((duration() * part) / 3));

    return (
        <div data-component="Browser" {...style.attrs(browserStyles.browser, styles.browser)}>
            {/* keep your space open in one tab */}
            <div data-component="TabStrip" {...style.attrs(browserStyles.strip)}>
                <span aria-hidden="true" {...style.attrs(browserStyles.lights)} />
                <span data-component="Tab" {...style.attrs(browserStyles.tab, browserStyles.tabOn)}>
                    <Favicon icon="user" tint={palette.teal} size={14} />
                    <span {...style.attrs(browserStyles.tabTitle)}>Florian</span>
                </span>
            </div>

            {/* show the page's address */}
            <div data-component="Toolbar" {...style.attrs(browserStyles.toolbar)}>
                <span {...style.attrs(browserStyles.tools, browserStyles.desktopOnly)}>
                    <Glyph name="back" />
                    <Glyph name="forward" />
                    <Glyph name="reload" />
                </span>
                <span data-component="AddressBar" {...style.attrs(browserStyles.omnibox)}>
                    <Glyph name="lock" size={12} />
                    <span {...style.attrs(browserStyles.url)}>
                        <span {...style.attrs(browserStyles.host)}>destack.app</span>
                        <span {...style.attrs(browserStyles.path)}>/@florian/map</span>
                    </span>
                </span>
            </div>

            {/* draw the Map app */}
            <div {...style.attrs(browserStyles.viewport, styles.viewport)}>
                <div {...style.attrs(appStyles.app)}>
                    <AppRail
                        open={properties.flow.icon}
                        apps={flows.map((flow) => [flow.icon, flow.name] as const)}
                        onOpen={(icon) =>
                            properties.onOpen(flows.findIndex((flow) => flow.icon === icon))
                        }
                    />
                    <main {...style.attrs(appStyles.main, styles.main)}>
                        <div {...style.attrs(appStyles.page, styles.page)}>
                            {/* name the run */}
                            <div {...style.attrs(appStyles.toolbar)}>
                                <b {...style.attrs(appText.title)}>{properties.flow.title}</b>
                                <div {...style.attrs(appStyles.tools)}>
                                    <span {...style.attrs(appStyles.property)}>
                                        {properties.flow.when}
                                    </span>
                                    <span {...style.attrs(appStyles.property)}>
                                        {duration()} ms
                                    </span>
                                </div>
                            </div>

                            {/* show the apps it went through */}
                            <div {...style.attrs(styles.path)}>
                                <For each={pathOf(properties.flow)}>
                                    {([icon, name], index) => (
                                        <>
                                            {isFirst(index()) ? null : (
                                                <span {...style.attrs(styles.arrow)}>→</span>
                                            )}
                                            <span {...style.attrs(styles.hop)}>
                                                <ServiceGlyph icon={icon} />
                                                {name}
                                            </span>
                                        </>
                                    )}
                                </For>
                            </div>

                            {/* draw every step along the run's time */}
                            <div {...style.attrs(styles.waterfall)}>
                                <div {...style.attrs(styles.step, styles.axis)}>
                                    <span>Step</span>
                                    <span {...style.attrs(styles.ticks)}>
                                        <For each={ticks()}>{(tick) => <span>{tick}</span>}</For>
                                    </span>
                                </div>
                                <For each={properties.flow.steps}>
                                    {(each, index) => (
                                        <StepRow
                                            step={each}
                                            duration={duration()}
                                            isSelected={index() === properties.selected}
                                            onSelect={() => properties.onSelect(index())}
                                        />
                                    )}
                                </For>
                            </div>
                        </div>
                    </main>
                    <div {...style.attrs(appStyles.statusBar)}>
                        <span {...style.attrs(appStyles.status)}>
                            <span {...style.attrs(appStyles.dot)} />
                            Yours · on your laptop
                        </span>
                        <span {...style.attrs(appStyles.statusEnd, styles.wideFlex)}>
                            {properties.flow.counts[0]} runs today · every step allowed
                        </span>
                    </div>
                </div>
            </div>
        </div>
    );
}

/** Draw one step: its app's tile indented by its depth, its name, and its bar on the time axis. */
function StepRow(properties: {
    step: Step;
    duration: number;
    isSelected: boolean;
    onSelect: () => void;
}) {
    return (
        <button
            type="button"
            onPointerEnter={() => properties.onSelect()}
            onClick={() => properties.onSelect()}
            {...style.attrs(styles.step, styles.stepButton, properties.isSelected && styles.stepOn)}
        >
            <span {...style.attrs(styles.stepName, styles.indent(properties.step.depth))}>
                <ServiceGlyph icon={properties.step.icon} />
                <span {...style.attrs(styles.stepText)}>{properties.step.name}</span>
                <span {...style.attrs(styles.why, properties.isSelected && styles.whyOn)}>
                    {properties.step.allowed}
                </span>
            </span>
            <span {...style.attrs(styles.track)}>
                <span
                    {...style.attrs(
                        styles.bar,
                        styles.barAt(
                            properties.step.start / properties.duration,
                            (properties.step.end - properties.step.start) / properties.duration,
                            paperGlyphOf(properties.step.icon),
                        ),
                    )}
                />
            </span>
        </button>
    );
}

/** List every automation as the bills list every app: a row each, its runs, failures and traces. */
function Automations(properties: {
    isOpen: boolean;
    row: number;
    onChoose: (index: number) => void;
}) {
    // set each tool or flow as a ledger row in the state shown
    const rows = (): readonly LedgerRow[] =>
        properties.isOpen
            ? flows.map((flow) =>
                  rowOf(flow.job, flow.name, { app: flow.icon }, palette.signal, flow.counts),
              )
            : tools.map((tool) =>
                  rowOf(
                      tool.job,
                      tool.name,
                      tool.mark,
                      accentOf(tool.name, tool.tint),
                      tool.counts,
                  ),
              );
    const total = () => (properties.isOpen ? totals.destacked : totals.stacked);

    return (
        <div data-component="Automations" {...style.attrs(styles.holdings)}>
            <Ledger
                rows={rows()}
                total={{
                    name: total()[0],
                    note: "",
                    cells: countsOf([total()[1], total()[2], total()[3]]),
                }}
                isWhole={properties.isOpen}
                lit={properties.row}
                isPressable
                onPoint={(index) => {
                    if (index !== undefined) {
                        properties.onChoose(index);
                    }
                }}
                onPress={properties.onChoose}
            />
        </div>
    );
}

/** Return the ledger row of a tool or flow: its job, name, mark, colour and counts. */
function rowOf(
    job: string,
    name: string,
    mark: ItemMark,
    tint: string,
    counts: readonly [runs: string, failed: string, traced: string],
): LedgerRow {
    return {
        label: job,
        name,
        marks: [mark],
        tint,
        note: "",
        isEmpty: false,
        badge: undefined,
        cells: countsOf(counts),
    };
}

/** Return a row's counts as ledger figures: runs and failures drop on phones, traces stay. */
function countsOf(
    counts: readonly [runs: string, failed: string, traced: string],
): readonly Cell[] {
    return [
        {
            caption: "runs",
            unit: "runs today",
            value: counts[0],
            isQuiet: true,
            isWide: true,
            isFloor: false,
        },
        {
            caption: "failed",
            unit: "failed runs today",
            value: counts[1],
            isQuiet: true,
            isWide: true,
            isFloor: false,
        },
        {
            caption: "traced",
            unit: "runs you can trace",
            value: counts[2],
            isQuiet: false,
            isWide: false,
            isFloor: false,
        },
    ];
}

/** The participle section styles. */
const styles = style.create({
    section: {
        gridTemplateRows: {
            default: "auto auto minmax(0, 1fr)",
            [screen.belowDesktop]: "auto",
        },
    },
    stage: {
        gridColumn: { default: "1 / 8", [screen.belowDesktop]: "1 / -1" },
        gridTemplateRows: "auto minmax(0, 1fr)",
    },
    key: {
        gridColumn: { default: "8 / -1", [screen.belowDesktop]: "1 / -1" },
        gridTemplateRows: "auto minmax(0, 1fr)",
    },

    // windows
    main: {
        gridTemplateRows: "minmax(0, 1fr)",
    },
    page: {
        gap: "0.875rem",
        maskImage: "none",
        paddingBlock: "1.25rem",
        paddingInline: { default: "1.5rem", [media.maxMd]: "1rem" },
    },

    // trace
    path: {
        alignItems: "center",
        display: { default: "flex", [media.maxMd]: "none" },
        flexWrap: "wrap",
        gap: "0.375rem",
    },
    hop: {
        alignItems: "center",
        backgroundColor: color.muted,
        borderRadius: "6px",
        display: "inline-flex",
        fontSize: "0.75rem",
        fontWeight: 600,
        gap: "0.375rem",
        paddingBlock: "0.1875rem",
        paddingInlineEnd: "0.5rem",
        paddingInlineStart: "0.1875rem",
    },
    arrow: {
        color: color.mutedForeground,
        fontSize: "0.75rem",
    },
    waterfall: {
        display: "grid",
    },
    step: {
        alignItems: "center",
        columnGap: "1rem",
        display: "grid",
        gridTemplateColumns: "minmax(0, 1.5fr) minmax(0, 1fr)",
        minHeight: "1.625rem",
    },
    axis: {
        borderBottomColor: color.border,
        borderBottomStyle: "solid",
        borderBottomWidth: stroke.border,
        color: color.mutedForeground,
        fontSize: "0.6875rem",
        minHeight: "1.5rem",
    },
    ticks: {
        display: "flex",
        fontFamily: font.code,
        justifyContent: "space-between",
    },
    stepButton: {
        backgroundColor: "transparent",
        borderRadius: "4px",
        borderWidth: 0,
        color: color.foreground,
        cursor: "pointer",
        fontFamily: font.text,
        paddingBlock: 0,
        paddingInline: 0,
        textAlign: "start",
        width: "100%",
    },
    stepOn: {
        backgroundColor: `color-mix(in srgb, ${palette.signal} 10%, transparent)`,
    },
    stepName: {
        alignItems: "center",
        gridColumn: "1",
        gridRow: "1",
        display: "flex",
        gap: "0.5rem",
        minWidth: 0,
    },
    indent: (depth: number) => ({
        paddingInlineStart: `${depth * 1.25}rem`,
    }),
    stepText: {
        fontSize: "0.8125rem",
        overflow: "hidden",
        textOverflow: "ellipsis",
        whiteSpace: "nowrap",
    },
    track: {
        gridColumn: "2",
        gridRow: "1",
        height: "0.625rem",
        position: "relative",
    },
    bar: {
        borderRadius: "2px",
        height: "100%",
        minWidth: "3px",
        position: "absolute",
    },
    barAt: (start: number, length: number, tint: string) => ({
        backgroundColor: tint,
        left: `${start * 100}%`,
        width: `${length * 100}%`,
    }),
    why: {
        backgroundColor: color.card,
        borderRadius: "4px",
        boxShadow: `0 0 0 1px ${color.border}`,
        color: color.mutedForeground,
        display: "none",
        fontSize: "0.6875rem",
        flexShrink: 0,
        marginLeft: "auto",
        marginRight: "0.375rem",
        paddingInline: "0.375rem",
        whiteSpace: "nowrap",
    },
    whyOn: {
        display: { default: "block", [media.maxMd]: "none" },
    },

    // holdings, as the bills draw their rows
    holdings: {
        display: "grid",
        gridTemplateRows: "minmax(0, 1fr) auto",
    },
    browser: {
        boxShadow: "none",
        height: { default: "100%", [screen.belowDesktop]: "34rem" },
    },
    viewport: {
        maskImage: "none",
        position: "relative",
    },
    rentedWindow: {
        containerType: "inline-size",
        height: "125%",
        left: 0,
        overflow: "clip",
        position: "absolute",
        top: 0,
        transform: "scale(0.8)",
        transformOrigin: "0 0",
        width: "125%",
    },
    wideFlex: {
        display: { default: "flex", [media.maxMd]: "none" },
    },
});
