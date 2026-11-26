import { type Message, SOURCE_LOCALE, t } from "@destack/locale";
import { ActionInput } from "@destack/object";
import { type JsonObject, type JsonValue, present } from "@destack/schema";
import * as style from "@destack/style";
import type { ObjectTypeReference } from "@destack/sync";
import type { SetStep } from "@destack/package/declare";
import type { Step } from "@destack/view/scenario";
import { text } from "@destack/theme/text";
import { color, space, weight } from "@destack/theme/tokens.stylex";
import {
    type CheckResult,
    createEffect,
    createMemo,
    createSignal,
    For,
    ExampleRequest,
    type JSX,
    onCleanup,
    onSettled,
    ExampleEvent,
    Show,
    useLocale,
    useEnvironment,
    untrack,
    useExamplePage,
} from "@destack/view";
import {
    Breadcrumb,
    BreadcrumbItem,
    BreadcrumbList,
    BreadcrumbPage,
    BreadcrumbSeparator,
} from "../../breadcrumb/index.ts";
import { Icon } from "@destack/ui/icon";
import { Inspector } from "../../inspector/index.ts";
import { ResizableHandle, ResizablePanel, ResizablePanelGroup } from "../../resizable/index.ts";
import {
    Select,
    SelectContent,
    SelectItem,
    SelectTrigger,
    SelectValue,
} from "../../select/index.ts";
import { Separator } from "../../separator/index.ts";
import { ToggleGroup, ToggleGroupItem } from "../../toggle-group/index.ts";
import { Toolbar, ToolbarGroup } from "../../toolbar/index.ts";
import { AxisChoice, axesOf, Canvas, Matrix, pageOf, type Viewport } from "./canvas.tsx";
import { Outline, type Subject } from "./outline.tsx";
import { Pane } from "./pane.tsx";
import { Badge } from "../../badge/index.ts";
import { Player } from "./player.tsx";
import { wordsOf } from "./words.ts";

/** The choice of no input for the matrix's rows. */
const NONE = "none";

/** The zooms the canvas offers, as fractions of the page's own size. */
const ZOOMS = ["0.5", "0.75", "1", "1.25"] as const;

/** The styles of the editor's panes, the stage's bar and the inspector. */
const styles = style.create({
    editor: {
        height: "100dvh",
        backgroundColor: color.background,
    },
    bar: {
        display: "flex",
        alignItems: "center",
        gap: space[2],
        width: "100%",
        minWidth: 0,
    },
    trail: {
        flexGrow: 1,
        minWidth: 0,
    },
    rule: {
        height: space[4],
    },
    group: {
        display: "flex",
        alignItems: "center",
        gap: space[1],
    },
    stage: {
        position: "relative",
        height: "100%",
    },
    layer: {
        position: "absolute",
        inset: 0,
    },
    hidden: {
        visibility: "hidden",
    },
    side: {
        display: "grid",
        alignContent: "start",
        gap: space[3],
        padding: space[3],
    },
    title: {
        flexGrow: 1,
        fontWeight: weight.semibold,
    },
    description: {
        margin: 0,
        color: color.mutedForeground,
    },
    heading: {
        margin: 0,
        marginBlockStart: space[2],
        color: color.mutedForeground,
        fontWeight: weight.medium,
        textTransform: "uppercase",
        letterSpacing: "0.04em",
    },
});

/** What the example page tells its host: its description, and its component's inputs and values. */
interface Declaration {
    /** The name of the example or scenario the page shows. */
    readonly name: string;
    /** The sentence saying what it shows. */
    readonly description: Message | undefined;
    /** The inputs a person changes the example by. */
    readonly inputs: readonly ActionInput[];
    /** The value of each input. */
    readonly values: JsonObject;
}

/** How long the player waits between the steps it plays, in milliseconds, to follow them by eye. */
const STEP_PAUSE = 400;

/** Browse a package's examples on a canvas, laid out by their choices, beside what they show. */
export function Editor(properties: {
    /** The name of the example shown first. */
    readonly first: string;
}): JSX.Element {
    // group the page's examples and scenarios by what they show, leaving the editor out
    const page = useExamplePage();
    const { index } = page;
    const subjects = createMemo((): readonly Subject[] => {
        // file each under its subject, examples apart from scenarios
        const grouped = new Map<string, { examples: string[]; scenarios: string[] }>();
        for (const entry of index) {
            const [subject = entry.name, member = ""] = entry.name.split(".");
            const group = grouped.get(subject) ?? { examples: [], scenarios: [] };
            (entry.kind === "example" ? group.examples : group.scenarios).push(member);
            grouped.set(subject, group);
        }
        grouped.delete("editor");

        return [...grouped]
            .map(([name, group]) => ({ name, ...group }))
            .toSorted((left, right) => left.name.localeCompare(right.name));
    });

    // track the example shown, how the canvas frames it, and its globals
    const [name, setName] = createSignal(properties.first);
    const [viewport, setViewport] = createSignal<Viewport>("fill");
    const [zoom, setZoom] = createSignal<string>("1");
    const [locale, setLocale] = createSignal<string>(SOURCE_LOCALE);
    const [direction, setDirection] = createSignal<"rtl">();
    const [appearance, setAppearance] = createSignal<"light" | "dark">();
    const trail = createMemo(() => name().split("."));
    const globals = createMemo((): JsonObject => ({
        locale: locale(),
        ...(direction() === undefined ? {} : { direction: direction() }),
        ...(appearance() === undefined ? {} : { theme: { appearance: appearance() } }),
    }));

    // read what the page tells about its example, or its scenario's steps and their results
    const [declaration, setDeclaration] = createSignal<Declaration>();
    const [steps, setSteps] = createSignal<readonly (SetStep | Step)[]>([]);
    const [results, setResults] = createSignal<readonly (readonly CheckResult[])[]>([]);
    const [isPlaying, setPlaying] = createSignal(false);
    const [run, setRun] = createSignal(0);
    const isScenario = createMemo(
        () => index.find((entry) => entry.name === name())?.kind === "scenario",
    );
    const environment = useEnvironment();
    let frame: HTMLIFrameElement | undefined;
    const { origin } = new URL(environment.navigation.currentEntry.url);
    const tell = (request: ExampleRequest) => {
        const target = frame?.contentWindow;
        if (target !== null && target !== undefined) {
            ExampleRequest.send(target, request, origin);
        }
    };
    const listen = (event: MessageEvent<unknown>) => {
        // take the page's messages alone
        const received =
            event.source === frame?.contentWindow ? ExampleEvent.read(event, origin) : undefined;

        // take an example's inputs and their values
        if (received?.destack === "example") {
            const inputs = received.inputs.map((each) => ActionInput.from(each, refuseObjects));
            const { name: shownName, description, values } = received;
            setDeclaration({ name: shownName, description, inputs, values });
        }
        // take a scenario's steps
        else if (received?.destack === "scenario") {
            const { name: shownName, description } = received;
            setDeclaration({ name: shownName, description, inputs: [], values: {} });
            setSteps(received.steps);
        }
        // take a step's results, and go on to the next while playing
        else if (received?.destack === "step") {
            const next = [...results(), received.results];
            setResults(next);
            if (isPlaying() && next.length < steps().length) {
                environment.clock.setTimeout(act, STEP_PAUSE);
            } else {
                setPlaying(false);
            }
        }
    };

    // act the scenario's next step, play every step left, or start it over on a fresh page
    const act = () => {
        tell({ destack: "act", index: results().length });
    };
    const play = () => {
        setPlaying(true);
        act();
    };
    const reset = () => {
        setPlaying(false);
        setResults([]);
        setRun((count) => count + 1);
    };
    // load the first example's page once
    const first = untrack(() => pageOf(name(), globals()));

    // show another example, other globals or a fresh run in the loaded page, in place
    createEffect(
        () => ({ name: name(), globals: globals(), run: run() }),
        ({ name: next, globals: given }, previous) => {
            if (previous !== undefined) {
                tell({ destack: "show", name: next, globals: given, properties: {}, matrix: null });
            }
        },
    );
    onSettled(() => {
        window.addEventListener("message", listen);
        onCleanup(() => window.removeEventListener("message", listen));
    });

    // send a changed property to the page, and store it
    const change = (input: string, value: JsonValue | undefined) => {
        setDeclaration((current) => {
            const shown = present(current, "the declaration whose property changes");

            return { ...shown, values: { ...shown.values, [input]: value ?? null } };
        });
        tell({ destack: "set", properties: { [input]: value ?? null } });
    };

    // take every value of a cell of the matrix
    const take = (values: JsonObject) => {
        for (const [input, value] of Object.entries(values)) {
            change(input, value);
        }
    };

    // open another example, keeping what the last one's page told until the next one tells
    const choose = (next: string) => {
        // forget the last scenario's steps and the matrix's axes
        setSteps([]);
        setResults([]);
        setPlaying(false);
        setChosen(undefined);
        setColumns(undefined);
        setRows(undefined);
        setName(next);
    };

    // write the editor in the person's language, and the languages it offers by their names
    const localization = useLocale();
    const languages = new Intl.DisplayNames([localization.tag], { type: "language" });

    // lay the example out as a matrix where what it shows declares choices, else on the canvas
    const [chosen, setChosen] = createSignal<"matrix" | "canvas">();
    const axes = createMemo(() => axesOf(declaration()?.inputs ?? []));
    const view = createMemo(
        () =>
            chosen() ??
            (axes().some((input) => input.value.kind === "choice") ? "matrix" : "canvas"),
    );

    // lay the first choice out as columns and the second as rows, until a person picks others
    const [columns, setColumns] = createSignal<string>();
    const [rows, setRows] = createSignal<string>();
    const across = createMemo(() => axes().find((input) => input.name === columns()) ?? axes()[0]);
    const down = createMemo(() => {
        const picked = rows();

        return picked === NONE
            ? undefined
            : (axes().find((input) => input.name === picked) ??
                  axes().find((input) => input.name !== across()?.name));
    });
    const labelOf = (input: ActionInput) =>
        input.label === undefined ? wordsOf(input.name, true) : localization.render(input.label);
    const named = () => axes().map((each) => ({ name: each.name, label: labelOf(each) }));

    return (
        <ResizablePanelGroup direction="horizontal" xstyle={styles.editor}>
            {/* Outline */}
            <ResizablePanel defaultSize={17} minSize={12}>
                <Outline subjects={subjects()} shown={name()} onChoose={choose} />
            </ResizablePanel>
            <ResizableHandle aria-label={localization.render(t`Resize the outline`)} />

            {/* Stage */}
            <ResizablePanel defaultSize={61}>
                <Pane
                    header={
                        <Toolbar
                            aria-label={localization.render(t.context("title")`Stage`)}
                            xstyle={styles.bar}
                        >
                            {/* Trail */}
                            <Breadcrumb xstyle={styles.trail}>
                                <BreadcrumbList>
                                    <BreadcrumbItem>
                                        {wordsOf(trail()[0] ?? "", true)}
                                    </BreadcrumbItem>
                                    <BreadcrumbSeparator />
                                    <BreadcrumbItem>
                                        <BreadcrumbPage>
                                            {wordsOf(trail()[1] ?? "", true)}
                                        </BreadcrumbPage>
                                    </BreadcrumbItem>
                                </BreadcrumbList>
                            </Breadcrumb>

                            {/* View */}
                            <Show when={!isScenario() && axes().length > 0}>
                                <ToggleGroup
                                    size="sm"
                                    variant="segmented"
                                    aria-label={localization.render(t.context("label")`View`)}
                                    value={view()}
                                    disallowEmptySelection
                                    onValueChange={(value) => {
                                        if (value === "matrix" || value === "canvas") {
                                            setChosen(value);
                                        }
                                    }}
                                >
                                    <ToggleGroupItem value="matrix">
                                        <Icon name="squares-four" />
                                        {localization.render(t.context("option")`Matrix`)}
                                    </ToggleGroupItem>
                                    <ToggleGroupItem value="canvas">
                                        <Icon name="frame-corners" />
                                        {localization.render(t.context("option")`Canvas`)}
                                    </ToggleGroupItem>
                                </ToggleGroup>
                                <Separator orientation="vertical" xstyle={styles.rule} />
                            </Show>

                            {/* Matrix axes, or the canvas's viewport and zoom */}
                            <Show
                                when={view() === "matrix" ? across() : undefined}
                                fallback={
                                    <ToolbarGroup xstyle={styles.group}>
                                        <ToggleGroup
                                            size="sm"
                                            variant="segmented"
                                            aria-label={localization.render(
                                                t.context("label")`Viewport`,
                                            )}
                                            value={viewport()}
                                            disallowEmptySelection
                                            onValueChange={(value) => {
                                                if (
                                                    value === "fill" ||
                                                    value === "phone" ||
                                                    value === "tablet" ||
                                                    value === "desktop"
                                                ) {
                                                    setViewport(value);
                                                }
                                            }}
                                        >
                                            <ToggleGroupItem
                                                value="fill"
                                                aria-label={localization.render(
                                                    t.context("option")`Fill`,
                                                )}
                                            >
                                                <Icon name="frame-corners" />
                                            </ToggleGroupItem>
                                            <ToggleGroupItem
                                                value="phone"
                                                aria-label={localization.render(
                                                    t.context("option")`Phone`,
                                                )}
                                            >
                                                <Icon name="device-mobile" />
                                            </ToggleGroupItem>
                                            <ToggleGroupItem
                                                value="tablet"
                                                aria-label={localization.render(
                                                    t.context("option")`Tablet`,
                                                )}
                                            >
                                                <Icon name="device-tablet" />
                                            </ToggleGroupItem>
                                            <ToggleGroupItem
                                                value="desktop"
                                                aria-label={localization.render(
                                                    t.context("option")`Desktop`,
                                                )}
                                            >
                                                <Icon name="desktop" />
                                            </ToggleGroupItem>
                                        </ToggleGroup>
                                        <Choice
                                            label={localization.render(t.context("label")`Zoom`)}
                                            value={zoom()}
                                            values={ZOOMS}
                                            words={(value) =>
                                                localization.number(Number(value), {
                                                    style: "percent",
                                                })
                                            }
                                            onChange={setZoom}
                                        />
                                    </ToolbarGroup>
                                }
                            >
                                {(column) => (
                                    <ToolbarGroup xstyle={styles.group}>
                                        <AxisChoice
                                            label={localization.render(t.context("label")`Columns`)}
                                            icon={
                                                <Icon
                                                    name="columns"
                                                    label={localization.render(
                                                        t.context("label")`Columns`,
                                                    )}
                                                />
                                            }
                                            value={column().name}
                                            axes={named()}
                                            onChange={setColumns}
                                        />
                                        <AxisChoice
                                            label={localization.render(t.context("label")`Rows`)}
                                            icon={
                                                <Icon
                                                    name="rows"
                                                    label={localization.render(
                                                        t.context("label")`Rows`,
                                                    )}
                                                />
                                            }
                                            value={down()?.name ?? NONE}
                                            axes={[
                                                {
                                                    name: NONE,
                                                    label: localization.render(
                                                        t.context("option")`None`,
                                                    ),
                                                },
                                                ...named(),
                                            ]}
                                            onChange={setRows}
                                        />
                                    </ToolbarGroup>
                                )}
                            </Show>
                            <Separator orientation="vertical" xstyle={styles.rule} />

                            {/* Globals */}
                            <ToolbarGroup xstyle={styles.group}>
                                <Choice
                                    label={localization.render(t.context("label")`Language`)}
                                    value={locale()}
                                    values={[SOURCE_LOCALE, ...page.locales]}
                                    words={(value) => languages.of(value) ?? value}
                                    onChange={setLocale}
                                />
                                <ToggleGroup
                                    size="sm"
                                    variant="segmented"
                                    aria-label={localization.render(t.context("label")`Direction`)}
                                    value={direction()}
                                    onValueChange={(value) =>
                                        setDirection(value === "rtl" ? value : undefined)
                                    }
                                >
                                    <ToggleGroupItem
                                        value="rtl"
                                        aria-label={localization.render(t`Right to left`)}
                                    >
                                        <Icon name="arrows-left-right" />
                                    </ToggleGroupItem>
                                </ToggleGroup>
                                <ToggleGroup
                                    size="sm"
                                    variant="segmented"
                                    aria-label={localization.render(t.context("label")`Appearance`)}
                                    value={appearance()}
                                    onValueChange={(value) =>
                                        setAppearance(
                                            value === "light" || value === "dark"
                                                ? value
                                                : undefined,
                                        )
                                    }
                                >
                                    <ToggleGroupItem
                                        value="light"
                                        aria-label={localization.render(t.context("option")`Light`)}
                                    >
                                        <Icon name="sun" />
                                    </ToggleGroupItem>
                                    <ToggleGroupItem
                                        value="dark"
                                        aria-label={localization.render(t.context("option")`Dark`)}
                                    >
                                        <Icon name="moon" />
                                    </ToggleGroupItem>
                                </ToggleGroup>
                            </ToolbarGroup>
                        </Toolbar>
                    }
                >
                    {/* Canvas, always loaded since its page tells what the example shows */}
                    <div {...style.attrs(styles.stage)}>
                        <div {...style.attrs(styles.layer, view() === "matrix" && styles.hidden)}>
                            <Canvas
                                source={first}
                                title={name()}
                                viewport={viewport()}
                                zoom={Number(zoom())}
                                onFrame={(created) => {
                                    frame = created;
                                }}
                            />
                        </div>
                        <Show when={view() === "matrix"}>
                            <div {...style.attrs(styles.layer)}>
                                <Show when={declaration()}>
                                    {(shown) => (
                                        <Show when={across()}>
                                            {(column) => (
                                                <Matrix
                                                    name={shown().name}
                                                    globals={globals()}
                                                    values={shown().values}
                                                    columns={column()}
                                                    rows={down()}
                                                    onChoose={take}
                                                    onOpen={(values) => {
                                                        take(values);
                                                        setChosen("canvas");
                                                    }}
                                                />
                                            )}
                                        </Show>
                                    )}
                                </Show>
                            </div>
                        </Show>
                    </div>
                </Pane>
            </ResizablePanel>
            <ResizableHandle aria-label={localization.render(t`Resize the inspector`)} />

            {/* Inspector */}
            <ResizablePanel defaultSize={22} minSize={14}>
                <Pane
                    isSide
                    header={
                        <>
                            <span {...style.attrs(text.sm, styles.title)}>
                                {wordsOf(trail()[0] ?? "", true)}
                            </span>
                            <Badge variant="soft" color="gray">
                                {isScenario()
                                    ? localization.render(t.context("option")`Scenario`)
                                    : localization.render(t.context("option")`Example`)}
                            </Badge>
                        </>
                    }
                >
                    {/* What the page shows, once it tells */}
                    <Show when={declaration()}>
                        {(shown) => (
                            <div {...style.attrs(styles.side)}>
                                <Show when={shown().description}>
                                    {(description) => (
                                        <p {...style.attrs(text.sm, styles.description)}>
                                            {localization.render(description())}
                                        </p>
                                    )}
                                </Show>
                                <Show when={isScenario()}>
                                    <h3 {...style.attrs(text.xs, styles.heading)}>
                                        {localization.render(t.context("title")`Steps`)}
                                    </h3>
                                    <Player
                                        steps={steps()}
                                        results={results()}
                                        onStep={act}
                                        onPlay={play}
                                        onReset={reset}
                                    />
                                </Show>
                                <Show when={!isScenario()}>
                                    <h3 {...style.attrs(text.xs, styles.heading)}>
                                        {localization.render(t.context("title")`Properties`)}
                                    </h3>
                                    <Show
                                        when={shown().inputs.length > 0}
                                        fallback={
                                            <p {...style.attrs(text.sm, styles.description)}>
                                                {localization.render(
                                                    t`This component declares no properties.`,
                                                )}
                                            </p>
                                        }
                                    >
                                        <Inspector
                                            aria-label={localization.render(
                                                t.context("title")`Properties`,
                                            )}
                                            inputs={shown().inputs}
                                            values={shown().values}
                                            onValueChange={change}
                                        />
                                    </Show>
                                </Show>
                            </div>
                        )}
                    </Show>
                </Pane>
            </ResizablePanel>
        </ResizablePanelGroup>
    );
}

/** Render a choice of the bar, such as the language. */
function Choice(properties: {
    /** What it chooses. */
    readonly label: string;
    /** The value chosen. */
    readonly value: string;
    /** The values to choose from. */
    readonly values: readonly string[];
    /** Write a value as a person reads it. */
    readonly words: (value: string) => string;
    /** Handle another value being chosen. */
    readonly onChange: (value: string) => void;
}): JSX.Element {
    return (
        <Select
            size="sm"
            variant="ghost"
            aria-label={properties.label}
            title={properties.label}
            value={properties.value}
            onValueChange={(value) => {
                if (value !== undefined) {
                    properties.onChange(value);
                }
            }}
        >
            <SelectTrigger>
                <SelectValue />
            </SelectTrigger>
            <SelectContent>
                <For each={properties.values}>
                    {(value) => <SelectItem value={value}>{properties.words(value)}</SelectItem>}
                </For>
            </SelectContent>
        </Select>
    );
}

/** Refuse the type of an example's object input, which a page never describes. */
function refuseObjects(reference: ObjectTypeReference): never {
    throw new TypeError(`an example's inputs take no objects, such as ${reference.type}`);
}
