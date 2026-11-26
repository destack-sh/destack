import { t } from "@destack/locale";
import warningCircleIcon from "@destack/icon/phosphor/warning-circle";
import { defineExample } from "@destack/package/declare";
import { Duration, Schedule, ScheduleLink, ScheduleLinkType, TimeZone } from "@destack/schema";
import { createMemo, createSignal, Show, useLocale } from "@destack/view";
import { createElementSize } from "@destack/view/primitives/resize-observer";
import * as style from "@destack/style";
import { color, swatch } from "@destack/theme/tokens.stylex";
import {
    Timeline,
    TimelineAxis,
    TimelineBar,
    TimelineBarAddon,
    TimelineBarList,
    TimelineCell,
    TimelineColumn,
    TimelineBody,
    TimelineTrack,
    TimelineGrid,
    TimelineHeader,
    type TimelineItem,
    type TimelineRowMove,
    TimelineLinkLayer,
    TimelineMinimap,
    TimelineNow,
    TimelineRange,
    TimelineRow,
    TimelineRowGuide,
    TimelineRowLabel,
    TimelineRowList,
    TimelineRowToggle,
    TimelineToday,
    TimelineToolbar,
    TimelineZoom,
} from "./index.ts";
import { Dialog, DialogContent, DialogHeader, DialogTitle } from "../dialog/index.ts";
import { KINDS, RUN, Step } from "../example/run.ts";
import { Field, FieldLabel } from "../field/index.ts";
import { Icon } from "../icon/index.ts";
import { Input } from "../input/index.ts";
import { Label } from "../label/index.ts";
import { Stack } from "../stack/index.ts";
import { Switch } from "../switch/index.ts";
import { ToggleGroup, ToggleGroupItem } from "../toggle-group/index.ts";

/** The styles of the trace's rows and the plan's toolbar. */
const styles = style.create({
    name: {
        flexGrow: 1,
        minWidth: 0,
        whiteSpace: "nowrap",
        overflow: "hidden",
        textOverflow: "ellipsis",
    },
    kind: {
        display: "inline-flex",
        flexShrink: 0,
        color: color.mutedForeground,
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
});

/** The milliseconds of a day, which the link editor writes lags in. */
const DAY = Duration.milliseconds({ days: 1 });

/** The width in pixels from which the run shows figures beside its labels. */
const WIDE = 560;

/** The run's steps on a timeline. */
const TRACE: readonly TimelineItem[] = RUN.map((each) => Step.item(each));

/** An agent's run: what each step is, how long it took, its tokens, and the one that failed. */
export const timelineTrace = defineExample({
    of: Timeline,
    name: "trace",
    description: t`An agent's run: each step's kind, time and tokens, and the one that failed.`,
    layout: "padded",
    properties: {
        axis: "offset",
    },
    render: (properties) => {
        // leave the figures out where they would take the bars' room
        const locale = useLocale();
        const [element, setElement] = createSignal<HTMLElement | undefined>(undefined, {
            ownedWrite: true,
        });
        const size = createElementSize(element);
        const isWide = (): boolean => (size.width ?? WIDE) >= WIDE;

        return (
            <Timeline {...properties} ref={setElement} items={TRACE}>
                <TimelineHeader corner="Step">
                    <Show when={isWide()}>
                        <TimelineColumn width="4rem" xstyle={styles.figure}>
                            Time
                        </TimelineColumn>
                        <TimelineColumn width="4rem" xstyle={styles.figure}>
                            Tokens
                        </TimelineColumn>
                    </Show>
                    <TimelineAxis />
                </TimelineHeader>
                <TimelineBody>
                    <TimelineGrid />
                    <TimelineRowList>
                        {(node) => {
                            // read what the row's step shows
                            const shown = (): Step => Step.of(node().item.key);
                            const seconds = (): number =>
                                ((node().item.end ?? 0) - (node().item.start ?? 0)) / 1_000;

                            return (
                                <TimelineRow>
                                    <TimelineRowLabel>
                                        <TimelineRowGuide />
                                        <TimelineRowToggle />
                                        <span {...style.attrs(styles.kind)}>
                                            <Icon icon={KINDS[shown().kind].icon} />
                                        </span>
                                        <span {...style.attrs(styles.name)}>
                                            {node().item.label}
                                        </span>
                                        <Show when={Step.hasFailed(shown())}>
                                            <span {...style.attrs(styles.failed)}>
                                                <Icon
                                                    icon={warningCircleIcon}
                                                    weight={
                                                        shown().error === undefined
                                                            ? "regular"
                                                            : "fill"
                                                    }
                                                    label={shown().error ?? "A step below failed"}
                                                />
                                            </span>
                                        </Show>
                                    </TimelineRowLabel>
                                    <Show when={isWide()}>
                                        <TimelineCell xstyle={styles.figure}>
                                            {locale.number(seconds(), {
                                                style: "unit",
                                                unit: "second",
                                                unitDisplay: "narrow",
                                                maximumFractionDigits: 1,
                                            })}
                                        </TimelineCell>
                                        <TimelineCell xstyle={styles.figure}>
                                            <Show when={shown().tokens}>
                                                {(tokens) => <>{locale.number(tokens())}</>}
                                            </Show>
                                        </TimelineCell>
                                    </Show>
                                    <TimelineTrack />
                                </TimelineRow>
                            );
                        }}
                    </TimelineRowList>
                </TimelineBody>
            </Timeline>
        );
    },
});

/** The start of the request, an instant in launch week. */
const REQUEST = Date.UTC(2026, 10, 19, 9, 0, 0);

/** A request's spans: the handler, its queries and the page it renders. */
const SPANS: readonly TimelineItem[] = [
    { key: "request", label: "GET /launch", start: REQUEST, end: REQUEST + 1_180 },
    {
        key: "auth",
        label: "verify session",
        start: REQUEST + 12,
        end: REQUEST + 64,
        parent: "request",
        color: "cyan",
    },
    {
        key: "signups",
        label: "SELECT signups",
        start: REQUEST + 70,
        end: REQUEST + 410,
        parent: "request",
        color: "purple",
    },
    {
        key: "render",
        label: "render page",
        start: REQUEST + 420,
        end: REQUEST + 1_150,
        parent: "request",
        color: "green",
    },
];

/** A request's spans as the timeline draws them alone. */
export const timelineRequest = defineExample({
    of: Timeline,
    name: "request",
    description: t`A request's spans as the timeline draws them without children.`,
    layout: "padded",
    properties: {
        axis: "offset",
    },
    render: (properties) => <Timeline {...properties} items={SPANS} />,
});

/** Read the instant of a day's start in Vienna. */
function day(date: string): number {
    return TimeZone.instant("Europe/Vienna", date, "00:00");
}

/** A launch plan's tasks, the later ones depending on the earlier. */
const PLAN: readonly TimelineItem[] = [
    {
        key: "design",
        label: "Design the pricing page",
        start: day("2026-11-02"),
        end: day("2026-11-06"),
        progress: 1,
    },
    {
        key: "build",
        label: "Build the pricing page",
        start: day("2026-11-06"),
        end: day("2026-11-13"),
        color: "green",
        progress: 0.6,
        baseline: { start: day("2026-11-05"), end: day("2026-11-12") },
        milestones: [{ key: "preview", label: "Preview build", at: day("2026-11-10") }],
    },
    {
        key: "copy",
        label: "Write the launch post",
        start: day("2026-11-09"),
        end: day("2026-11-16"),
        color: "orange",
        progress: 0.25,
        deadline: day("2026-11-14"),
    },
    {
        key: "press",
        label: "Brief the press",
        start: day("2026-11-12"),
        color: "purple",
    },
    {
        key: "changelog",
        label: "Write the changelog",
    },
    {
        key: "launch",
        label: "Launch",
        start: day("2026-11-16"),
        end: day("2026-11-16"),
        color: "red",
    },
];

/** The plan's dependencies: building waits on design, the launch on building and the post. */
const LINKS: readonly ScheduleLink[] = [
    { from: "design", to: "build", type: "finish-to-start", lag: 0 },
    { from: "build", to: "launch", type: "finish-to-start", lag: 0 },
    { from: "copy", to: "launch", type: "finish-to-start", lag: 0 },
];

/** Place a moved item under its new parent, after the item before it or before the item after. */
function placed(items: readonly TimelineItem[], move: TimelineRowMove): TimelineItem[] {
    // take the item out and give it its new parent
    const moved = items.find((item) => item.key === move.key);
    if (moved === undefined) {
        throw new RangeError(`timeline plan item '${move.key}' not found`);
    }
    const { parent: _parent, ...rest } = moved;
    const item = move.parent === undefined ? rest : { ...rest, parent: move.parent };
    const others = items.filter((each) => each.key !== move.key);

    // insert it after the item before it, else before the item after it, else at the end
    const after = others.findIndex((each) => each.key === move.previous);
    const before = others.findIndex((each) => each.key === move.next);
    const index = after !== -1 ? after + 1 : before !== -1 ? before : others.length;

    return others.toSpliced(index, 0, item);
}

/** A launch plan whose moves push the tasks waiting on them, its critical path drawn solid. */
export const timelinePlan = defineExample({
    of: Timeline,
    name: "plan",
    description: t`A launch plan of linked tasks that move and stretch along the days.`,
    layout: "padded",
    render: () => {
        // save the tasks where people leave them, and the links they draw and remove
        const [items, setItems] = createSignal<readonly TimelineItem[]>(PLAN);
        const [links, setLinks] = createSignal<readonly ScheduleLink[]>(LINKS);

        // order the dated tasks by their links, for the critical path and the tasks a change pushes
        const schedule = createMemo(() => {
            const tasks = items().flatMap((item) =>
                item.start === undefined || item.end === undefined
                    ? []
                    : [{ ...item, start: item.start, end: item.end }],
            );
            const keys = new Set(tasks.map((task) => task.key));

            return new Schedule(
                tasks,
                links().filter((link) => keys.has(link.from) && keys.has(link.to)),
            );
        });

        // move the tasks a change pushes along only while the person asks for it
        const [isPushing, setPushing] = createSignal(true);

        // edit an opened link's type and lag in a dialog, each change applied at once
        const [editing, setEditing] = createSignal<ScheduleLink>();
        const change = (changed: ScheduleLink) => {
            setLinks((current) =>
                current.map((each) => (ScheduleLink.equals(each, changed) ? changed : each)),
            );
            setEditing(changed);
        };

        return (
            <Stack space="2">
                <Label>
                    <Switch checked={isPushing()} onCheckedChange={setPushing} />
                    Move linked tasks along
                </Label>
                <Timeline
                    items={items().map((item) =>
                        !schedule().has(item.key)
                            ? item
                            : schedule().isCritical(item.key)
                              ? { ...item, variant: "solid" as const }
                              : { ...item, slack: schedule().slack(item.key) },
                    )}
                    timeZone="Europe/Vienna"
                    links={links()}
                    propagate={(changed) => {
                        // leave the other tasks where they are while pushing is off
                        if (!isPushing()) {
                            return changed;
                        }

                        // push the tasks that wait on the changed dated tasks, by their links
                        const isScheduled = (item: TimelineItem) =>
                            item.start !== undefined &&
                            item.end !== undefined &&
                            schedule().has(item.key);
                        const tasks = changed.flatMap((item) =>
                            item.start === undefined || item.end === undefined || !isScheduled(item)
                                ? []
                                : [{ ...item, start: item.start, end: item.end }],
                        );

                        return [
                            ...changed.filter((item) => !isScheduled(item)),
                            ...(tasks.length === 0 ? [] : schedule().move(tasks)),
                        ];
                    }}
                    onItemsChange={(changed) => {
                        // replace each changed item
                        const byKey = new Map(changed.map((item) => [item.key, item]));
                        setItems((current) => current.map((item) => byKey.get(item.key) ?? item));
                    }}
                    canLink={(link) => schedule().allows(link)}
                    onLinkCreate={(link) => setLinks((current) => [...current, link])}
                    onRowMove={(move) => setItems((current) => placed(current, move))}
                    onLinkAction={setEditing}
                    onLinkDelete={(link) =>
                        setLinks((current) =>
                            current.filter((each) => !ScheduleLink.equals(each, link)),
                        )
                    }
                >
                    <TimelineToolbar>
                        <TimelineToday />
                        <TimelineZoom />
                    </TimelineToolbar>
                    <TimelineHeader corner="Task">
                        <TimelineColumn width="4rem">Done</TimelineColumn>
                        <TimelineAxis />
                    </TimelineHeader>
                    <TimelineBody>
                        <TimelineGrid />
                        <TimelineRange
                            start={day("2026-11-16")}
                            end={day("2026-11-21")}
                            color="red"
                        >
                            Launch week
                        </TimelineRange>
                        <TimelineNow />
                        <TimelineLinkLayer />
                        <TimelineRowList>
                            {(node) => (
                                <TimelineRow>
                                    <TimelineRowLabel />
                                    <TimelineCell>
                                        <Show when={node().item.progress}>
                                            {(done) => (
                                                <>{`${String(Math.round(done() * 100))}%`}</>
                                            )}
                                        </Show>
                                    </TimelineCell>
                                    <TimelineTrack />
                                </TimelineRow>
                            )}
                        </TimelineRowList>
                    </TimelineBody>
                    <TimelineMinimap />
                </Timeline>
                <Dialog
                    open={editing() !== undefined}
                    onOpenChange={(isOpen) => {
                        if (!isOpen) {
                            setEditing(undefined);
                        }
                    }}
                >
                    <DialogContent>
                        <DialogHeader>
                            <DialogTitle>Dependency</DialogTitle>
                        </DialogHeader>
                        <Show when={editing()}>
                            {(link) => (
                                <Stack space="3">
                                    <ToggleGroup
                                        aria-label="Type"
                                        variant="outline"
                                        disallowEmptySelection
                                        value={link().type}
                                        onValueChange={(type) =>
                                            change({
                                                ...link(),
                                                type: ScheduleLinkType.parse(type),
                                            })
                                        }
                                    >
                                        <ToggleGroupItem value="finish-to-start">
                                            Finish to start
                                        </ToggleGroupItem>
                                        <ToggleGroupItem value="start-to-start">
                                            Start to start
                                        </ToggleGroupItem>
                                        <ToggleGroupItem value="finish-to-finish">
                                            Finish to finish
                                        </ToggleGroupItem>
                                        <ToggleGroupItem value="start-to-finish">
                                            Start to finish
                                        </ToggleGroupItem>
                                    </ToggleGroup>
                                    <Field>
                                        <FieldLabel>Lag in days</FieldLabel>
                                        <Input
                                            type="number"
                                            value={String(link().lag / DAY)}
                                            onChange={(event) =>
                                                change({
                                                    ...link(),
                                                    lag: Math.round(
                                                        Number(event.currentTarget.value) * DAY,
                                                    ),
                                                })
                                            }
                                        />
                                    </Field>
                                </Stack>
                            )}
                        </Show>
                    </DialogContent>
                </Dialog>
            </Stack>
        );
    },
});

/** A team's week: each person a row, their tasks packed into lanes where they overlap. */
const TEAM: readonly TimelineItem[] = [
    { key: "ada", label: "Ada Lovelace" },
    { key: "grace", label: "Grace Hopper" },
    { key: "linus", label: "Linus Pauling" },
    {
        key: "pricing",
        label: "Pricing page",
        row: "ada",
        start: day("2026-11-02"),
        end: day("2026-11-05"),
        color: "blue",
    },
    {
        key: "checkout",
        label: "Checkout",
        row: "ada",
        start: day("2026-11-04"),
        end: day("2026-11-07"),
        color: "green",
    },
    {
        key: "post",
        label: "Launch post",
        row: "grace",
        start: day("2026-11-03"),
        end: day("2026-11-06"),
        color: "orange",
    },
    {
        key: "press",
        label: "Press kit",
        row: "linus",
        start: day("2026-11-05"),
        end: day("2026-11-07"),
        color: "purple",
    },
];

/** A team's week, each person a row, tasks moved in time and between people. */
export const timelineTeam = defineExample({
    of: Timeline,
    name: "team",
    description: t`A team's week, each person a row, tasks moved in time and between people.`,
    layout: "padded",
    render: () => {
        const [items, setItems] = createSignal<readonly TimelineItem[]>(TEAM);

        return (
            <Timeline
                items={items()}
                timeZone="Europe/Vienna"
                onItemsChange={(changed) =>
                    setItems((all) =>
                        all.map((item) => changed.find((each) => each.key === item.key) ?? item),
                    )
                }
            >
                <TimelineHeader corner="Person" />
                <TimelineBody>
                    <TimelineGrid />
                    <TimelineNow />
                    <TimelineRowList />
                </TimelineBody>
            </Timeline>
        );
    },
});

/** An agent's run, each step's time beside its bar. */
export const timelineBarAddonTime = defineExample({
    of: TimelineBarAddon,
    name: "time",
    description: t`An agent's run, each step's time beside its bar.`,
    layout: "padded",
    render: (properties) => (
        <Timeline items={TRACE} axis="offset">
            <TimelineBody>
                <TimelineRowList>
                    {(node) => (
                        <TimelineRow>
                            <TimelineRowLabel>{node().item.label}</TimelineRowLabel>
                            <TimelineTrack>
                                <TimelineBarList>
                                    {(item) => (
                                        <TimelineBar>
                                            <TimelineBarAddon {...properties}>
                                                {`${String(((item().end ?? 0) - (item().start ?? 0)) / 1_000)}s`}
                                            </TimelineBarAddon>
                                        </TimelineBar>
                                    )}
                                </TimelineBarList>
                            </TimelineTrack>
                        </TimelineRow>
                    )}
                </TimelineRowList>
            </TimelineBody>
        </Timeline>
    ),
});
