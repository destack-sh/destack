import warningCircleIcon from "@destack/icon/phosphor/warning-circle";
import { defineExample } from "@destack/package/declare";
import * as style from "@destack/style";
import { text } from "@destack/theme/text";
import { color, space, stroke, swatch, weight } from "@destack/theme/tokens.stylex";
import { createEffect, createSignal, For, type JSX, Show, useLocale } from "@destack/view";
import { Alert, AlertDescription, AlertTitle } from "../alert/index.ts";
import { Badge } from "../badge/index.ts";
import { Button } from "../button/index.ts";
import {
    Collapsible,
    CollapsibleContent,
    CollapsibleIndicator,
    CollapsibleTrigger,
} from "../collapsible/index.ts";
import {
    Composer,
    ComposerAddon,
    ComposerFrame,
    ComposerInput,
    ComposerSubmit,
} from "../composer/index.ts";
import { CopyButton } from "../copy-button/index.ts";
import { DataList, DataListItem, DataListLabel, DataListValue } from "../data-list/index.ts";
import { KINDS, RUN, Step, type StepValue } from "../example/run.ts";
import { Icon } from "../icon/index.ts";
import { Marker, MarkerContent, MarkerDescription, MarkerIcon } from "../marker/index.ts";
import {
    Message,
    MessageAuthor,
    MessageAvatar,
    MessageContent,
    MessageHeader,
} from "../message/index.ts";
import { ResizableHandle, ResizablePanel, ResizablePanelGroup } from "../resizable/index.ts";
import { Stack } from "../stack/index.ts";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "../tabs/index.ts";
import {
    Timeline,
    TimelineAxis,
    TimelineBar,
    TimelineBarList,
    TimelineBody,
    TimelineCell,
    TimelineColumn,
    TimelineGrid,
    TimelineHeader,
    type TimelineItem,
    TimelineRow,
    TimelineRowGuide,
    TimelineRowLabel,
    TimelineRowList,
    TimelineRowToggle,
    TimelineTrack,
    useTimeline,
} from "../timeline/index.ts";
import {
    Tree,
    TreeItem,
    TreeItemGroup,
    TreeItemGuide,
    TreeItemRow,
    TreeItemToggle,
} from "../tree/index.ts";
import { App, Face, PEOPLE } from "./app.tsx";

/** The narrow screens, on which the details stand below. */
const NARROW = "@media (max-width: 60rem)";

/** The styles of the viewer. */
const styles = style.create({
    viewer: {
        flexGrow: 1,
        width: "auto",
        height: "auto",
        minHeight: 0,
        margin: `calc(-1 * ${space[5]})`,
        flexDirection: { default: "row", [NARROW]: "column" },
    },
    divider: {
        display: { default: null, [NARROW]: "none" },
    },
    main: {
        padding: space[4],
        overflow: "auto",
    },
    details: {
        padding: space[4],
        overflow: "auto",
    },
    thread: {
        paddingBlockStart: space[3],
    },
    chosen: {
        color: color.foreground,
        fontWeight: weight.medium,
    },
    kind: {
        display: "inline-flex",
        flexShrink: 0,
        color: color.mutedForeground,
    },
    name: {
        flexGrow: 1,
        minWidth: 0,
        whiteSpace: "nowrap",
        overflow: "hidden",
        textOverflow: "ellipsis",
    },
    failed: {
        display: "inline-flex",
        flexShrink: 0,
        color: swatch.destructiveText,
    },
    figure: {
        justifyContent: "flex-end",
        borderInlineEndWidth: 0,
        color: color.mutedForeground,
        fontVariantNumeric: "tabular-nums",
    },
    ringed: {
        outlineWidth: stroke.ring,
        outlineStyle: "solid",
        outlineColor: color.ring,
        outlineOffset: stroke.border,
    },
    heading: {
        display: "flex",
        alignItems: "center",
        gap: space[2],
        margin: 0,
        fontWeight: weight.semibold,
    },
    section: {
        display: "flex",
        alignItems: "center",
        justifyContent: "space-between",
        margin: 0,
        color: color.mutedForeground,
        fontWeight: weight.medium,
    },
    key: {
        color: color.mutedForeground,
    },
    value: {
        minWidth: 0,
        overflowWrap: "anywhere",
    },
});

/** A view of the run, which shows and changes the step chosen. */
interface RunViewProperties {
    /** The key of the step chosen. */
    readonly step: string;
    /** Handle the person choosing another step. */
    readonly onStepChange: (key: string) => void;
}

/** Write how long a step took. */
function Seconds(properties: { readonly step: Step }): JSX.Element {
    const locale = useLocale();

    return (
        <>
            {locale.number(properties.step.end - properties.step.start, {
                style: "unit",
                unit: "second",
                unitDisplay: "narrow",
                maximumFractionDigits: 1,
            })}
        </>
    );
}

/** Mark a step that failed, filled, or one over it, hollow. */
function Failure(properties: { readonly step: Step }): JSX.Element {
    return (
        <Show when={Step.hasFailed(properties.step)}>
            <span {...style.attrs(styles.failed)}>
                <Icon
                    icon={warningCircleIcon}
                    weight={properties.step.error === undefined ? "regular" : "fill"}
                    label={properties.step.error ?? "A step below failed"}
                />
            </span>
        </Show>
    );
}

/** Render the run as its steps in order, each under the one that took it. */
function Thread(properties: RunViewProperties & { readonly parent?: string }): JSX.Element {
    const steps = () => RUN.filter((each) => each.parent === properties.parent);

    return (
        <Stack space="2">
            <For each={steps()}>
                {(step) => (
                    <ThreadStep
                        shown={step}
                        step={properties.step}
                        onStepChange={properties.onStepChange}
                    />
                )}
            </For>
        </Stack>
    );
}

/** Render a step of the thread, with the steps it took. */
function ThreadStep(properties: RunViewProperties & { readonly shown: Step }): JSX.Element {
    // choose the step as its steps fold
    const isChosen = (): boolean => properties.step === properties.shown.key;
    const isParent = (): boolean => RUN.some((each) => each.parent === properties.shown.key);
    const line = (): JSX.Element => (
        <>
            <MarkerIcon>
                <Icon icon={KINDS[properties.shown.kind].icon} />
            </MarkerIcon>
            <MarkerContent>{properties.shown.name}</MarkerContent>
            <MarkerDescription>
                <Seconds step={properties.shown} />
            </MarkerDescription>
        </>
    );
    const choose = (): void => properties.onStepChange(properties.shown.key);
    const status = () => (properties.shown.error === undefined ? undefined : "error");

    return (
        <Show
            when={isParent()}
            fallback={
                <Marker
                    status={status()}
                    aria-pressed={isChosen() ? "true" : "false"}
                    xstyle={isChosen() && styles.chosen}
                    onClick={choose}
                    render={(attributes) => <button type="button" {...attributes} />}
                >
                    {line()}
                </Marker>
            }
        >
            <Collapsible defaultOpen>
                <Marker
                    status={status()}
                    xstyle={isChosen() && styles.chosen}
                    onClick={choose}
                    render={(attributes) => <CollapsibleTrigger {...attributes} />}
                >
                    {line()}
                    <CollapsibleIndicator />
                </Marker>
                <CollapsibleContent variant="line">
                    <Thread
                        parent={properties.shown.key}
                        step={properties.step}
                        onStepChange={properties.onStepChange}
                    />
                </CollapsibleContent>
            </Collapsible>
        </Show>
    );
}

/** Bring the chosen step into the timeline's view. */
function Reveal(properties: { readonly step: string }): JSX.Element {
    const control = useTimeline();
    createEffect(
        () => (control.tree.items().has(properties.step) ? properties.step : undefined),
        (key) => {
            if (key !== undefined) {
                control.reveal(key);
            }
        },
    );

    return null;
}

/** Read the step a row stands for, none for a row of one name. */
function shown(key: string): Step | undefined {
    return RUN.find((step) => step.key === key);
}

/** Read the first step of a name. */
function namesake(name: string): Step | undefined {
    return RUN.find((step) => step.name === name);
}

/** Render the run on a time axis, each step in its row or the steps of one name in one. */
function Trace(properties: RunViewProperties & { readonly isByName: boolean }): JSX.Element {
    // give each name its row, or each step
    const items = (): readonly TimelineItem[] =>
        properties.isByName
            ? [
                  ...[...new Set(RUN.map((step) => step.name))].map((name) => ({
                      key: name,
                      label: name,
                  })),
                  ...RUN.map(({ parent: _parent, ...step }) => ({
                      ...Step.item(step),
                      row: step.name,
                  })),
              ]
            : RUN.map((step) => Step.item(step));

    return (
        <Timeline
            axis="offset"
            items={items()}
            selection={{
                multiple: true,
                behavior: "replace",
                get value() {
                    return [properties.step];
                },
                onValueChange: (keys) => {
                    const key = keys.at(-1);
                    if (key !== undefined && shown(key) !== undefined) {
                        properties.onStepChange(key);
                    }
                },
            }}
        >
            <Reveal step={properties.step} />
            <TimelineHeader corner={properties.isByName ? "Name" : "Step"}>
                <TimelineColumn width="4rem" xstyle={styles.figure}>
                    {properties.isByName ? "Calls" : "Time"}
                </TimelineColumn>
                <TimelineAxis />
            </TimelineHeader>
            <TimelineBody>
                <TimelineGrid />
                <TimelineRowList>
                    {(node) => {
                        const step = () => shown(node().item.key) ?? namesake(node().item.label);

                        return (
                            <TimelineRow>
                                <TimelineRowLabel>
                                    <TimelineRowGuide />
                                    <TimelineRowToggle />
                                    <Show when={step()}>
                                        {(found) => (
                                            <span {...style.attrs(styles.kind)}>
                                                <Icon icon={KINDS[found().kind].icon} />
                                            </span>
                                        )}
                                    </Show>
                                    <span {...style.attrs(styles.name)}>{node().item.label}</span>
                                    <Show when={shown(node().item.key)}>
                                        {(found) => <Failure step={found()} />}
                                    </Show>
                                </TimelineRowLabel>
                                <TimelineCell xstyle={styles.figure}>
                                    <Show
                                        when={shown(node().item.key)}
                                        fallback={
                                            RUN.filter((each) => each.name === node().item.label)
                                                .length
                                        }
                                    >
                                        {(found) => <Seconds step={found()} />}
                                    </Show>
                                </TimelineCell>
                                <TimelineTrack>
                                    <TimelineBarList>
                                        {(item) => (
                                            <TimelineBar
                                                xstyle={
                                                    item().key === properties.step && styles.ringed
                                                }
                                            />
                                        )}
                                    </TimelineBarList>
                                </TimelineTrack>
                            </TimelineRow>
                        );
                    }}
                </TimelineRowList>
            </TimelineBody>
        </Timeline>
    );
}

/** Render the row of a value: its key, and the value where nothing is under it. */
function ValueRow(properties: { readonly name: string; readonly value: StepValue }): JSX.Element {
    return (
        <TreeItemRow>
            <TreeItemGuide />
            <TreeItemToggle />
            <span {...style.attrs(styles.key)}>{properties.name}</span>
            <Show when={typeof properties.value !== "object" || properties.value === null}>
                <span {...style.attrs(styles.value)}>{JSON.stringify(properties.value)}</span>
            </Show>
        </TreeItemRow>
    );
}

/** Render a value as a tree: its keys fold, its ends show. */
function Value(properties: { readonly value: StepValue; readonly path: string }): JSX.Element {
    const entries = (): readonly (readonly [string, StepValue])[] =>
        typeof properties.value === "object" && properties.value !== null
            ? Object.entries(properties.value)
            : [];

    return (
        <For each={entries()}>
            {([key, value]) => {
                const path = `${properties.path}.${key}`;
                return (
                    <TreeItem value={path} defaultExpanded>
                        <ValueRow name={key} value={value} />
                        <Show when={typeof value === "object" && value !== null}>
                            <TreeItemGroup>
                                <Value value={value} path={path} />
                            </TreeItemGroup>
                        </Show>
                    </TreeItem>
                );
            }}
        </For>
    );
}

/** Render what a step took or gave, with a button to copy it. */
function Payload(properties: { readonly title: string; readonly value: StepValue }): JSX.Element {
    const isTree = (): boolean => typeof properties.value === "object" && properties.value !== null;

    return (
        <section aria-label={properties.title}>
            <h4 {...style.attrs(text.xs, styles.section)}>
                {properties.title}
                <CopyButton
                    value={JSON.stringify(properties.value, null, 2)}
                    label={`Copy the ${properties.title.toLowerCase()}`}
                    size="icon-sm"
                />
            </h4>
            <Show
                when={isTree()}
                fallback={
                    <div {...style.attrs(text.sm)}>
                        {typeof properties.value === "string"
                            ? properties.value
                            : JSON.stringify(properties.value)}
                    </div>
                }
            >
                <Tree aria-label={properties.title}>
                    <Value value={properties.value} path="value" />
                </Tree>
            </Show>
        </section>
    );
}

/** Render the details of the chosen step, what it took and gave, and what people say about it. */
function Details(properties: { readonly step: Step }): JSX.Element {
    // keep the comments of each step
    const locale = useLocale();
    const [comments, setComments] = createSignal<Readonly<Record<string, readonly string[]>>>({
        "rates-1": ["The rates service drops the first call after a deploy."],
    });
    const [words, setWords] = createSignal("");

    return (
        <Stack space="5">
            <h3 {...style.attrs(text.base, styles.heading)}>
                <Icon icon={KINDS[properties.step.kind].icon} />
                {properties.step.name}
                <Badge variant="secondary">{KINDS[properties.step.kind].label}</Badge>
            </h3>
            <Show when={properties.step.error}>
                {(error) => (
                    <Alert variant="destructive">
                        <Icon icon={warningCircleIcon} />
                        <AlertTitle>This step failed</AlertTitle>
                        <AlertDescription>{error()}</AlertDescription>
                    </Alert>
                )}
            </Show>
            <DataList orientation="horizontal">
                <DataListItem>
                    <DataListLabel>Time</DataListLabel>
                    <DataListValue>
                        <Seconds step={properties.step} />
                    </DataListValue>
                </DataListItem>
                <Show when={properties.step.tokens}>
                    {(tokens) => (
                        <DataListItem>
                            <DataListLabel>Tokens</DataListLabel>
                            <DataListValue>{locale.number(tokens())}</DataListValue>
                        </DataListItem>
                    )}
                </Show>
            </DataList>
            <Show when={properties.step.input}>
                {(value) => <Payload title="Input" value={value()} />}
            </Show>
            <Show when={properties.step.output}>
                {(value) => <Payload title="Output" value={value()} />}
            </Show>
            <Stack space="3" aria-label="Comments" role="group">
                <For each={comments()[properties.step.key] ?? []}>
                    {(comment) => (
                        <Message>
                            <MessageAvatar position="top">
                                <Face person={PEOPLE.jonas} />
                            </MessageAvatar>
                            <MessageContent>
                                <MessageHeader>
                                    <MessageAuthor>{PEOPLE.jonas.name}</MessageAuthor>
                                </MessageHeader>
                                <div {...style.attrs(text.sm)}>{comment}</div>
                            </MessageContent>
                        </Message>
                    )}
                </For>
                <Composer
                    onSubmit={() => {
                        // add the comment to the step
                        const said = words();
                        const { key } = properties.step;
                        setComments((all) => ({ ...all, [key]: [...(all[key] ?? []), said] }));
                        setWords("");
                    }}
                >
                    <ComposerFrame variant="outline">
                        <ComposerInput
                            placeholder="Comment on this step"
                            value={words()}
                            onInput={(event) => setWords(event.currentTarget.value)}
                        />
                        <ComposerAddon>
                            <ComposerSubmit />
                        </ComposerAddon>
                    </ComposerFrame>
                </Composer>
            </Stack>
        </Stack>
    );
}

/** An agent's run as a thread, a timeline and lanes, with the chosen step beside them. */
export const sceneTrace = defineExample({
    of: Timeline,
    name: "viewer",
    description: "an agent's run as a thread, a timeline and lanes, one step chosen across all",
    layout: "fullscreen",
    render: () => {
        const [step, setStep] = createSignal("rates-1");

        return (
            <App
                name="Runs"
                groups={[
                    {
                        label: "Today",
                        places: [
                            { name: "Compare the quotes", icon: <Icon icon={KINDS.agent.icon} /> },
                            {
                                name: "Draft the launch post",
                                icon: <Icon icon={KINDS.agent.icon} />,
                            },
                            { name: "Sort the inbox", icon: <Icon icon={KINDS.agent.icon} /> },
                        ],
                    },
                ]}
                current="Compare the quotes"
                actions={<Button variant="outline">Run again</Button>}
            >
                <ResizablePanelGroup xstyle={styles.viewer}>
                    <ResizablePanel defaultSize={62} minSize="320px" xstyle={styles.main}>
                        <Tabs defaultValue="timeline">
                            <TabsList>
                                <TabsTrigger value="thread">Thread</TabsTrigger>
                                <TabsTrigger value="timeline">Timeline</TabsTrigger>
                                <TabsTrigger value="lanes">Lanes</TabsTrigger>
                            </TabsList>
                            <TabsContent value="thread" xstyle={styles.thread}>
                                <Thread step={step()} onStepChange={setStep} />
                            </TabsContent>
                            <TabsContent value="timeline" xstyle={styles.thread}>
                                <Trace step={step()} onStepChange={setStep} isByName={false} />
                            </TabsContent>
                            <TabsContent value="lanes" xstyle={styles.thread}>
                                <Trace step={step()} onStepChange={setStep} isByName />
                            </TabsContent>
                        </Tabs>
                    </ResizablePanel>
                    <ResizableHandle aria-label="Resize the details" xstyle={styles.divider} />
                    <ResizablePanel minSize="260px" xstyle={styles.details}>
                        <Details step={Step.of(step())} />
                    </ResizablePanel>
                </ResizablePanelGroup>
            </App>
        );
    },
});
