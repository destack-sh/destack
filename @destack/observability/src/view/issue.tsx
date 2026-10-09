import type { JsonCondition } from "@destack/db";
import type { Event } from "@destack/event";
import { Badge } from "@destack/ui/badge";
import { Button } from "@destack/ui/button";
import { ItemContent, ItemDescription, ItemTitle, itemVariants } from "@destack/ui/item";
import type { Identifier } from "@destack/schema";
import * as style from "@destack/style";
import { text } from "@destack/theme/text";
import { createSignal, For, Loading, Show, useQuery, useSpace, useView } from "@destack/view";
import { parseStack } from "../issue/frame.ts";
import { log } from "../event/index.ts";
import { issue, type Issue } from "../object/index.ts";
import { useEvents } from "./feed.ts";
import {
    FilterField,
    MoreButton,
    OpenButton,
    PAGE_ROWS,
    styles,
    timeOf,
    useFilter,
} from "./page.tsx";

/** The statuses the page lists issues of, unresolved first. */
const STATUSES = ["unresolved", "regressed", "ignored", "resolved"] as const;

/** The microseconds in a millisecond, as events time themselves in microseconds. */
const MICROSECONDS = 1000;

/** The lifecycle methods a person calls on an issue. */
type LifecycleMethod = "resolve" | "ignore" | "reopen";

/** List a space's issues of a status the typed filter selects, most recently seen first, opening the one chosen. */
export default function Issues() {
    // hold the filter, the status and the open issue
    const filter = useFilter(new Set(Object.keys(issue.fields)));
    const [status, setStatus] = createSignal<Issue["status"]>("unresolved");
    const [open, setOpen] = createSignal<Identifier<"issue">>();

    return (
        <main {...style.attrs(text.footnote, styles.page)} aria-label="Issues">
            {/* Statuses and filter */}
            <header {...style.attrs(styles.header)}>
                <For each={STATUSES}>
                    {(each) => (
                        <Button
                            variant={status() === each ? "default" : "ghost"}
                            size="sm"
                            onClick={() => setStatus(each)}
                        >
                            {each}
                        </Button>
                    )}
                </For>
                <FilterField filter={filter} placeholder="level = fatal AND people > 10" />
            </header>

            {/* Issues beside the open one */}
            <div {...style.attrs(styles.split)}>
                <Loading fallback={<p>Loading issues</p>}>
                    <Show when={filter.condition()}>
                        {(condition) => (
                            <IssueList
                                status={status()}
                                condition={condition()}
                                open={open()}
                                choose={setOpen}
                            />
                        )}
                    </Show>
                </Loading>
                <Show when={open()}>{(id) => <IssueDetail id={id()} />}</Show>
            </div>
        </main>
    );
}

/** List the issues of a status a condition selects, most recently seen first, opening the one chosen. */
function IssueList(properties: {
    /** The status listed. */
    status: Issue["status"];
    /** The condition over the issues' fields. */
    condition: JsonCondition;
    /** The open issue. */
    open: Identifier<"issue"> | undefined;
    /** Open an issue. */
    choose: (id: Identifier<"issue">) => void;
}) {
    // follow the issues of the status the condition selects
    const objects = useSpace({ issue });
    const issues = useQuery(() =>
        objects.query.issue.findMany({
            where: { AND: [{ status: properties.status }, properties.condition] },
            orderBy: { lastSeenAt: "desc" },
            limit: PAGE_ROWS,
        }),
    );

    return (
        <ol {...style.attrs(styles.list)} aria-label="Issues">
            <For each={issues()}>
                {(row) => (
                    <li>
                        <OpenButton
                            slot="issue"
                            isOpen={properties.open === row.id}
                            open={() => properties.choose(row.id)}
                        >
                            <ItemContent>
                                <ItemTitle>
                                    {row.title}
                                    <Badge
                                        variant={
                                            row.level === "fatal" || row.level === "error"
                                                ? "destructive"
                                                : "secondary"
                                        }
                                    >
                                        {row.level}
                                    </Badge>
                                </ItemTitle>
                                <ItemDescription>
                                    {row.count} events · {row.people} people ·{" "}
                                    {timeOf(row.lastSeenAt)}
                                </ItemDescription>
                            </ItemContent>
                        </OpenButton>
                    </li>
                )}
            </For>
        </ol>
    );
}

/** Show one issue with where it fails and its events, and resolve, ignore or reopen it. */
function IssueDetail(properties: {
    /** The open issue. */
    id: Identifier<"issue">;
}) {
    // follow the issue
    const objects = useSpace({ issue });
    const current = useQuery(() => objects.query.issue.findFirst({ where: { id: properties.id } }));

    /** Call a lifecycle method of the issue as one mutation, setting what reopens it first when given. */
    async function change(method: LifecycleMethod, until?: Issue["until"]): Promise<void> {
        const id = properties.id;
        await objects.mutation(async (mutation) => {
            // set what reopens the issue, then move it
            if (until !== undefined) {
                await mutation.call(issue).update({ id, until });
            }
            await mutation.call(issue)[method]({ id });
        }).confirmed;
    }

    return (
        <Loading fallback={<p>Loading the issue</p>}>
            <Show when={current()}>
                {(read) => (
                    <article {...style.attrs(styles.detail)} aria-label="Issue">
                        {/* Title and facts */}
                        <h2>{read().title}</h2>
                        <dl {...style.attrs(styles.facts)}>
                            <dt>Status</dt>
                            <dd data-slot="status">{read().status}</dd>
                            <dt>Declaration</dt>
                            <dd>{read().declaration ?? "undeclared code"}</dd>
                            <dt>Culprit</dt>
                            <dd>{read().culprit ?? "outside the package"}</dd>
                            <dt>Seen</dt>
                            <dd>
                                {timeOf(read().firstSeenAt)} to {timeOf(read().lastSeenAt)}
                            </dd>
                        </dl>

                        {/* Lifecycle */}
                        <Lifecycle issue={read()} change={change} />

                        {/* Events */}
                        <h3>Events</h3>
                        <IssueEvents issue={read()} />
                    </article>
                )}
            </Show>
        </Loading>
    );
}

/** Offer the lifecycle moves an issue's status allows: resolve or ignore an open issue, reopen a settled one. */
function Lifecycle(properties: {
    /** The issue. */
    issue: Pick<Issue, "status" | "lastRevisionId">;
    /** Move the issue, setting what reopens it first when given. */
    change: (method: LifecycleMethod, until?: Issue["until"]) => Promise<void>;
}) {
    const isSettled = () =>
        properties.issue.status === "resolved" || properties.issue.status === "ignored";

    return (
        <div {...style.attrs(styles.actions)}>
            <Show
                when={isSettled()}
                fallback={
                    <>
                        {/* Resolve on the next event */}
                        <Button
                            onClick={() => void properties.change("resolve", { kind: "event" })}
                        >
                            Resolve
                        </Button>

                        {/* Resolve in the next revision */}
                        <Show when={properties.issue.lastRevisionId}>
                            {(after) => (
                                <Button
                                    variant="outline"
                                    onClick={() =>
                                        void properties.change("resolve", {
                                            kind: "revision",
                                            after: after(),
                                        })
                                    }
                                >
                                    Resolve in the next revision
                                </Button>
                            )}
                        </Show>

                        {/* Ignore */}
                        <Button
                            variant="ghost"
                            onClick={() => void properties.change("ignore", { kind: "never" })}
                        >
                            Ignore
                        </Button>
                    </>
                }
            >
                <Button variant="outline" onClick={() => void properties.change("reopen")}>
                    Reopen
                </Button>
            </Show>
        </div>
    );
}

/** List an issue's exception records newest first, following new ones as they commit. */
function IssueEvents(properties: {
    /** The issue whose events to list. */
    issue: Pick<Issue, "id" | "installationId" | "firstSeenAt">;
}) {
    // follow the exception records grouped into the issue since its first one
    const view = useView();
    const events = useEvents(
        () => ({
            kind: "log",
            scope: view.space,
            object: properties.issue.installationId,
            where: `issue = "${properties.issue.id}"`,
            from: properties.issue.firstSeenAt * MICROSECONDS,
        }),
        () => true,
    );

    return (
        <>
            <ol {...style.attrs(styles.list)} aria-label="Events">
                <For each={events().events}>{(event) => <IssueEvent event={event} />}</For>
            </ol>
            <MoreButton more={events().more} />
        </>
    );
}

/** Show one exception record: when it happened, its message and its innermost in-app frame. */
function IssueEvent(properties: {
    /** The log event. */
    event: Event;
}) {
    // read the record's message and its innermost frame in the package's own code
    const keys = () => log.parseKeys(properties.event.keys);
    const title = () => {
        const message = keys().attributes["exception.message"];

        return typeof message === "string" ? message : keys().name;
    };
    const frame = () => {
        const stack = keys().attributes["exception.stacktrace"];

        return parseStack(typeof stack === "string" ? stack : "").find((each) => each.isInApp);
    };

    return (
        <li data-slot="event" {...style.attrs(itemVariants({ variant: "outline", size: "sm" }))}>
            <ItemContent>
                <ItemTitle>{title()}</ItemTitle>
                <ItemDescription>
                    {timeOf(properties.event.time / MICROSECONDS)}
                    <Show when={frame()}>
                        {(found) =>
                            ` · ${found().function ?? "anonymous"} at ${found().file}:${found().line}`
                        }
                    </Show>
                </ItemDescription>
            </ItemContent>
        </li>
    );
}
