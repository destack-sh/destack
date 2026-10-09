import { Badge } from "@destack/ui/badge";
import { ItemContent, ItemDescription, ItemTitle } from "@destack/ui/item";
import * as style from "@destack/style";
import { text } from "@destack/theme/text";
import { color, radius, space } from "@destack/theme/tokens.stylex";
import {
    createMemo,
    createSignal,
    For,
    Loading,
    onCleanup,
    Show,
    useService,
    useView,
} from "@destack/view";
import type { Event } from "@destack/event";
import * as kinds from "../event/index.ts";
import { eventService } from "@destack/event/service";
import { observabilityService } from "../service/index.ts";
import { EventFeed, useEvents } from "./feed.ts";
import { FilterField, MoreButton, OpenButton, styles, timeOf, useFilter } from "./page.tsx";

/** The time line of a trace: each span a row indented below its parent, its bar placed by its start and length. */
const spanStyles = style.create({
    span: {
        display: "grid",
        gridTemplateColumns: "minmax(12rem, 1fr) 2fr",
        alignItems: "center",
        gap: space[2],
    },
    track: {
        position: "relative",
        height: space[2],
    },
    bar: (left: string, width: string) => ({
        position: "absolute",
        left,
        width,
        minWidth: "2px",
        height: "100%",
        borderRadius: radius[1],
        backgroundColor: color.primary,
    }),
    failed: {
        backgroundColor: color.destructive,
    },
    indent: (depth: number) => ({
        paddingInlineStart: `calc(${depth} * ${space[3]})`,
    }),
});

/** The microseconds in a millisecond, as events time themselves in microseconds. */
const MICROSECONDS = 1000;

/** One span of a trace as the time line shows it. */
interface TraceSpan {
    /** The span, as 16 hexadecimal digits. */
    readonly span: string;
    /** The parent span, absent for a root. */
    readonly parent: string | undefined;
    /** The span's name. */
    readonly name: string;
    /** The start time, in Unix microseconds. */
    readonly time: number;
    /** The duration, in microseconds. */
    readonly duration: number;
    /** The outcome. */
    readonly status: string;
    /** The trace. */
    readonly trace: string;
}

/** List a space's recent traces by their root spans the typed filter selects, newest first, opening the one chosen. */
export default function Traces() {
    // follow the root spans the filter selects, and the open trace
    const view = useView();
    const filter = useFilter();
    const [open, setOpen] = createSignal<string>();
    const roots = useEvents(
        () => {
            const where = filter.where();

            return where === undefined
                ? undefined
                : {
                      kind: "span",
                      scope: view.space,
                      where: where === "" ? "parent = null" : `parent = null AND (${where})`,
                  };
        },
        () => true,
    );

    return (
        <main {...style.attrs(text.footnote, styles.page)} aria-label="Traces">
            {/* Filter */}
            <header {...style.attrs(styles.header)}>
                <FilterField filter={filter} placeholder="status = error AND duration > 500000" />
            </header>

            {/* Traces beside the open one */}
            <div {...style.attrs(styles.split)}>
                <Loading fallback={<p>Loading traces</p>}>
                    <div {...style.attrs(styles.detail)}>
                        <TraceList traces={roots().events} open={open()} choose={setOpen} />
                        <MoreButton more={roots().more} />
                    </div>
                </Loading>
                <Show when={open()}>{(id) => <TraceDetail id={id()} />}</Show>
            </div>
        </main>
    );
}

/** List traces by their root spans, opening the one chosen. */
function TraceList(properties: {
    /** The root span events, newest first. */
    traces: readonly Event[];
    /** The open trace. */
    open: string | undefined;
    /** Open a trace. */
    choose: (id: string) => void;
}) {
    return (
        <ol {...style.attrs(styles.list)} aria-label="Traces">
            <For each={properties.traces}>
                {(event) => {
                    const row = spanOf(event);

                    return (
                        <li>
                            <OpenButton
                                slot="trace"
                                isOpen={properties.open === row.trace}
                                open={() => properties.choose(row.trace)}
                            >
                                <ItemContent>
                                    <ItemTitle>
                                        {row.name}
                                        <Badge
                                            variant={
                                                row.status === "error" ? "destructive" : "secondary"
                                            }
                                        >
                                            {row.status}
                                        </Badge>
                                    </ItemTitle>
                                    <ItemDescription>
                                        {timeOf(row.time / MICROSECONDS)} ·{" "}
                                        {row.duration / MICROSECONDS} ms
                                    </ItemDescription>
                                </ItemContent>
                            </OpenButton>
                        </li>
                    );
                }}
            </For>
        </ol>
    );
}

/** Show one trace's spans as a tree on its time line, following new spans as they commit. */
function TraceDetail(properties: {
    /** The open trace. */
    id: string;
}) {
    // read the trace through observability, then follow its spans
    const events = useService(eventService);
    const observability = useService(observabilityService);
    const view = useView();
    const spans = createMemo(() => {
        // follow the open trace until another opens or the detail closes
        const stopping = new AbortController();
        onCleanup(() => stopping.abort());
        const scope = view.space;
        const trace = properties.id;
        const feed = new EventFeed(events, { kind: "span", scope, where: `trace = "${trace}"` });
        const first = async () => ({ events: (await observability.trace({ scope, trace })).spans });

        return feed.follow(first, true, stopping.signal);
    });
    const read = createMemo(() => spans().events.map(spanOf));

    // measure the trace's start, length, root and failures
    const start = () => Math.min(...read().map((span) => span.time));
    const length = () => Math.max(...read().map((span) => span.time + span.duration)) - start();
    const root = () => read().find((span) => span.parent === undefined);
    const failed = () => read().filter((span) => span.status === "error").length;

    return (
        <Loading fallback={<p>Loading the trace</p>}>
            <article {...style.attrs(styles.detail)} aria-label="Trace">
                {/* Root and totals */}
                <h2>{root()?.name}</h2>
                <p {...style.attrs(styles.muted)}>
                    {timeOf(start() / MICROSECONDS)} · {length() / MICROSECONDS} ms · {failed()}{" "}
                    failed of {read().length}
                </p>

                {/* Time line */}
                <ol {...style.attrs(styles.list)} aria-label="Spans">
                    <For each={tree(read())}>
                        {({ span, depth }) => (
                            <SpanRow
                                span={span}
                                depth={depth}
                                start={start()}
                                duration={length()}
                            />
                        )}
                    </For>
                </ol>
            </article>
        </Loading>
    );
}

/** Show one span as a row of its trace's time line. */
function SpanRow(properties: {
    /** The span. */
    span: TraceSpan;
    /** Its depth below the root. */
    depth: number;
    /** The trace's start, in Unix microseconds. */
    start: number;
    /** The trace's length, in microseconds. */
    duration: number;
}) {
    // place the bar by the span's start and length within the trace
    const share = (value: number) => `${(100 * value) / Math.max(properties.duration, 1)}%`;
    const left = () => share(properties.span.time - properties.start);
    const width = () => share(properties.span.duration);

    return (
        <li data-slot="span" data-depth={properties.depth} {...style.attrs(spanStyles.span)}>
            <span {...style.attrs(spanStyles.indent(properties.depth))}>
                {properties.span.name} · {properties.span.duration / MICROSECONDS} ms
            </span>
            <span {...style.attrs(spanStyles.track)}>
                <span
                    {...style.attrs(
                        spanStyles.bar(left(), width()),
                        properties.span.status === "error" && spanStyles.failed,
                    )}
                />
            </span>
        </li>
    );
}

/** Read a span event as the time line shows it. */
function spanOf(event: Event): TraceSpan {
    const keys = kinds.span.parseKeys(event.keys);

    return {
        span: kinds.span.parseData(event.data).span,
        parent: keys.parent ?? undefined,
        name: keys.name,
        time: event.time,
        duration: keys.duration,
        status: keys.status,
        trace: keys.trace,
    };
}

/** Order spans depth first below their parents in start order, each with its depth, roots and orphans at the top. */
function tree(spans: readonly TraceSpan[]): { span: TraceSpan; depth: number }[] {
    // index the children of each span in start order
    const ids = new Set(spans.map((span) => span.span));
    const started = spans.toSorted((left, right) => left.time - right.time);
    const children = Map.groupBy(started, (span) =>
        span.parent !== undefined && ids.has(span.parent) ? span.parent : "",
    );

    // walk from the roots
    const ordered: { span: TraceSpan; depth: number }[] = [];
    const visit = (parent: string, depth: number) => {
        for (const span of children.get(parent) ?? []) {
            ordered.push({ span, depth });
            visit(span.span, depth + 1);
        }
    };
    visit("", 0);

    return ordered;
}
