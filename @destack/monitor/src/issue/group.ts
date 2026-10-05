import { and, count, eq, inArray, sql, type DatabaseConnection } from "@destack/db";
import { SystemCall } from "@destack/object";
import type { ObjectServer } from "@destack/object/server";
import { BuildCache } from "@destack/package/manifest";
import { present, schema, type Digest, type Identifier } from "@destack/schema";
import { installation, installationRevision } from "@destack/space/object";
import type { OpenBuild } from "@destack/space/server";
import type { Entry } from "../entry/index.ts";
import { ISSUE_LEVELS, issue, type Issue } from "../object/index.ts";
import { issuePerson } from "../stack/db.ts";
import { Exception } from "./exception.ts";
import { fingerprint } from "./fingerprint.ts";
import { parseStack, type StackFrame } from "./frame.ts";
import { Symbolicator } from "./symbolicate.ts";

/** The attribute naming the person a signal acted for, from the OpenTelemetry semantic conventions. */
const PERSON_ATTRIBUTE = "enduser.id";

/** The attribute naming the issue an exception is grouped into, and the issue an issue's metric counts. */
export const ISSUE_ATTRIBUTE = "destack.issue";

/** The metric counting each issue's exceptions, which series conditions read for an issue's frequency. */
export const ISSUE_EVENTS_METRIC = "issue.events";

/** The monitor as the source of the entries it derives. */
const SOURCE = { name: "@destack/monitor", version: "1" };

/** One exception grouped into an issue, after Sentry's events: its entry, frames, fingerprint and the revision that ran it. */
interface IssueEvent {
    /** The exception's entry. */
    readonly entry: Entry;
    /** The exception. */
    readonly exception: Exception;
    /** Its frames, resolved to the build's graph when the revision is known. */
    readonly frames: readonly StackFrame[];
    /** Its grouping. */
    readonly fingerprint: string;
    /** The installation revision that ran it, absent when unknown. */
    readonly revision: Revision | undefined;
}

/** An installation revision an exception ran: its identifier and when it was made, which orders an installation's revisions. */
interface Revision {
    /** The revision. */
    readonly id: Identifier<"installation-revision">;
    /** When it was made, in Unix milliseconds. */
    readonly createdAt: number;
}

/** Groups unexpected exceptions into a space's issues: symbolicated, attributed to the graph and fingerprinted. */
export class Grouper {
    /** The object server keeping the issues beside copies of the installations and their revisions. */
    readonly #objects: Pick<ObjectServer, "database" | "executeAsSystem">;
    /** Open an installation revision's build. */
    readonly #open: OpenBuild;
    /** The symbolicators of the builds read recently. */
    readonly #symbolicators = new BuildCache(async (reader) => new Symbolicator(reader));

    /** Group exceptions into the issues an object server keeps, opening the builds of the revisions that ran them. */
    constructor(objects: Pick<ObjectServer, "database" | "executeAsSystem">, open: OpenBuild) {
        this.#objects = objects;
        this.#open = open;
    }

    /**
     * Group a space's unexpected exceptions into its issues, opening, counting, regressing and escalating them.
     *
     * Returns the entries to keep: each exception stamped with its issue, and each issue's count of them as a point.
     */
    async group(spaceId: string, entries: readonly Entry[], now = Date.now()): Promise<Entry[]> {
        // group the exceptions of spaces' installations only, leaving a host's own to its operators
        const parsed = schema.identifier("space").safeParse(spaceId);
        if (!parsed.success) {
            return [...entries];
        }
        const scope = parsed.data;

        // resolve the unexpected exceptions into events, grouped by installation and fingerprint
        const events = new Map<Entry, IssueEvent>();
        for (const entry of entries) {
            const exception = Exception.of(entry);
            if (
                exception !== undefined &&
                !Exception.isExpected(exception) &&
                entry.installation !== undefined
            ) {
                events.set(entry, await this.#event(scope, entry, exception));
            }
        }
        const grouped = Map.groupBy(events.values(), keyOf);
        if (grouped.size === 0) {
            return [...entries];
        }

        // open new issues
        const known = await this.#known(scope, [...events.values()]);
        await this.#objects.executeAsSystem(
            issue,
            "open",
            [...grouped]
                .filter(([key]) => !known.has(key))
                .map(([, group]) => ({ scope, input: openingOf(group) })),
            now,
        );

        // count the events and people into every issue
        const issues = await this.#known(scope, [...events.values()]);
        const counted = [...grouped].flatMap(([key, group]) => {
            const row = issues.get(key);

            return row === undefined ? [] : [{ row, group }];
        });
        const people = await this.#affect(scope, counted);
        await this.#objects.executeAsSystem(
            issue,
            "occur",
            counted.map(({ row, group }) => {
                const latest = latestOf(group);

                return SystemCall.of(row, {
                    level: group.map((event) => event.exception.level).reduce(severest, row.level),
                    lastSeenAt: Math.max(row.lastSeenAt, ...group.map(timeOf)),
                    ...(latest === undefined ? {} : { lastRevisionId: latest.id }),
                    count: row.count + group.length,
                    people: people.get(row.id) ?? row.people,
                });
            }),
            now,
        );

        // regress resolved issues and escalate ignored ones whose condition the events meet
        const revisions = await this.#revisionsOf(counted.map(({ row }) => row));
        const reopened = counted.filter(({ row, group }) =>
            reopens(row, group, row.count + group.length, revisions),
        );
        await this.#transition(
            "regress",
            reopened.filter(({ row }) => row.status === "resolved"),
            now,
        );
        await this.#transition(
            "escalate",
            reopened.filter(({ row }) => row.status === "ignored"),
            now,
        );

        // stamp each exception with its issue, and count each issue's events as a point
        const issueOf = (event: IssueEvent) => issues.get(keyOf(event))?.id;

        return [
            ...entries.map((entry) => {
                const event = events.get(entry);
                const issueId = event === undefined ? undefined : issueOf(event);

                return issueId === undefined
                    ? entry
                    : { ...entry, attributes: { ...entry.attributes, [ISSUE_ATTRIBUTE]: issueId } };
            }),
            ...counted.map(({ row, group }) => pointOf(row, group, now)),
        ];
    }

    /** Resolve one exception into an event: the revision that ran it, its frames through that revision's build, and its fingerprint. */
    async #event(
        scope: Identifier<"space">,
        entry: Entry,
        exception: Exception,
    ): Promise<IssueEvent> {
        // find the revision built from the entry's manifest in the copied revisions
        const found =
            entry.installation === undefined || entry.build === undefined
                ? undefined
                : await this.#revision(scope, entry.installation, entry.build);

        // read the frames, mapped and attributed through the revision's build when known
        const frames = parseStack(exception.stacktrace ?? "");
        const resolved =
            found === undefined
                ? frames
                : await (await this.#symbolicators.read(await found.open())).frames(frames);

        return {
            entry,
            exception,
            frames: resolved,
            fingerprint: await fingerprint(exception, resolved),
            revision: found?.revision,
        };
    }

    /** Find an installation's revision built from a manifest in the copies, with how to open its build, absent once none is. */
    async #revision(
        scope: Identifier<"space">,
        installationId: Identifier<"installation">,
        manifest: Digest,
    ) {
        // read the installation's revisions from the copies
        const rows = await this.#objects.database
            .select({
                id: installationRevision.table.id,
                createdAt: installationRevision.table.createdAt,
                packageId: installation.table.packageId,
                build: installationRevision.table.build,
            })
            .from(installationRevision.table)
            .innerJoin(
                installation.table,
                eq(installation.table.id, installationRevision.table.installationId),
            )
            .where(
                and(eq(installation.table.scope, scope), eq(installation.table.id, installationId)),
            );

        // take the one built from the manifest
        const found = rows.find((row) => row.build.manifest === manifest);

        return (
            found && {
                revision: { id: found.id, createdAt: found.createdAt },
                open: () => this.#open(found.packageId, found.build, scope),
            }
        );
    }

    /** Read the creation times of the revisions issues' conditions name. */
    async #revisionsOf(rows: readonly Issue[]): Promise<Map<string, number>> {
        // collect the revisions the conditions name
        const named = rows.flatMap((row) =>
            row.until?.kind === "revision" ? [row.until.after] : [],
        );
        if (named.length === 0) {
            return new Map();
        }
        const found = await this.#objects.database
            .select({
                id: installationRevision.table.id,
                createdAt: installationRevision.table.createdAt,
            })
            .from(installationRevision.table)
            .where(inArray(installationRevision.table.id, named));

        return new Map(found.map((row) => [row.id, row.createdAt]));
    }

    /** Move issues to a status as the system, at their latest revision. */
    async #transition(
        name: "regress" | "escalate",
        moved: readonly { readonly row: Issue }[],
        now: number,
    ): Promise<void> {
        if (moved.length === 0) {
            return;
        }
        const rows = await this.#objects.database
            .select()
            .from(issue.table)
            .where(
                inArray(
                    issue.table.id,
                    moved.map(({ row }) => row.id),
                ),
            );
        await this.#objects.executeAsSystem(
            issue,
            name,
            rows.map((row) => SystemCall.of(row)),
            now,
        );
    }

    /** Read a space's issues of events by installation and fingerprint. */
    async #known(
        scope: Identifier<"space">,
        events: readonly IssueEvent[],
    ): Promise<Map<string, Issue>> {
        const rows = await this.#objects.database
            .select()
            .from(issue.table)
            .where(
                and(
                    eq(issue.table.scope, scope),
                    inArray(
                        issue.table.fingerprint,
                        events.map((event) => event.fingerprint),
                    ),
                ),
            );

        return new Map(rows.map((row) => [keyOf(row), row]));
    }

    /** Keep the people each issue's events affected, with their latest time, returning each issue's people count. */
    async #affect(
        scope: Identifier<"space">,
        counted: readonly { readonly row: Issue; readonly group: readonly IssueEvent[] }[],
    ): Promise<Map<string, number>> {
        const people = new Map<string, number>();
        await this.#objects.database.transaction(async (transaction) => {
            for (const { row, group } of counted) {
                // keep the people with their latest time
                const affected = peopleOf(group);
                if (affected.size === 0) {
                    continue;
                }
                await transaction
                    .insert(issuePerson)
                    .values(
                        [...affected].map(([person, lastSeenAt]) => ({
                            scope,
                            issueId: row.id,
                            person,
                            lastSeenAt,
                        })),
                    )
                    .onConflictDoUpdate({
                        target: [issuePerson.issueId, issuePerson.person],
                        set: {
                            lastSeenAt: sql`max(${issuePerson.lastSeenAt}, excluded.last_seen_at)`,
                        },
                    });

                // count them
                people.set(row.id, await countPeople(transaction, row.id));
            }
        });

        return people;
    }
}

/** Count the people an issue affected. */
async function countPeople(
    database: DatabaseConnection,
    issueId: Identifier<"issue">,
): Promise<number> {
    const [counted] = await database
        .select({ count: count() })
        .from(issuePerson)
        .where(eq(issuePerson.issueId, issueId));

    return counted?.count ?? 0;
}

/** The input opening an issue for its first events, counted once they are counted into it. */
function openingOf(group: readonly IssueEvent[]) {
    // name the first event's title, its innermost in-app symbol and the declaration an in-app frame belongs to
    const [first] = group;
    if (first === undefined) {
        throw new TypeError("an issue opens for at least one event");
    }
    const inApp = first.frames.filter((frame) => frame.isInApp);
    const culprit = inApp.find((frame) => frame.moniker !== undefined)?.moniker;
    const declaration = inApp.find((frame) => frame.declaration !== undefined)?.declaration;
    const { exception } = first;
    const firstLine = exception.message.split("\n")[0] ?? "";
    const times = group.map(timeOf);
    const revisions = group.flatMap((event) => event.revision ?? []);
    const earliest = revisions.reduce<Revision | undefined>(
        (found, revision) =>
            found === undefined || revision.createdAt < found.createdAt ? revision : found,
        undefined,
    );
    const latest = latestOf(group);

    return {
        installationId: present(first.entry.installation, "an event's installation"),
        fingerprint: first.fingerprint,
        errorType: exception.errorType,
        title: `${exception.type ?? exception.errorType}: ${firstLine}`.slice(0, 500),
        ...(culprit === undefined ? {} : { culprit }),
        ...(declaration === undefined ? {} : { declaration: declaration.moniker }),
        level: group.map((event) => event.exception.level).reduce(severest),
        firstSeenAt: Math.min(...times),
        lastSeenAt: Math.max(...times),
        ...(earliest === undefined ? {} : { firstRevisionId: earliest.id }),
        ...(latest === undefined ? {} : { lastRevisionId: latest.id }),
    };
}

/** Count an issue's events as one point of its metric. */
function pointOf(row: Issue, group: readonly IssueEvent[], now: number): Entry {
    return {
        kind: "point",
        name: ISSUE_EVENTS_METRIC,
        time: now * 1000,
        installation: row.installationId,
        source: SOURCE,
        status: "unset",
        metric: "sum",
        unit: "{event}",
        value: group.length,
        attributes: { [ISSUE_ATTRIBUTE]: row.id },
    };
}

/** Read the latest revision events ran. */
function latestOf(group: readonly IssueEvent[]): Revision | undefined {
    return group
        .flatMap((event) => event.revision ?? [])
        .reduce<Revision | undefined>(
            (found, revision) =>
                found === undefined || revision.createdAt > found.createdAt ? revision : found,
            undefined,
        );
}

/** Read an event's time in Unix milliseconds. */
function timeOf(event: IssueEvent): number {
    return Math.floor(event.entry.time / 1000);
}

/** Read the people events affected, with the latest time each was. */
function peopleOf(group: readonly IssueEvent[]): Map<string, number> {
    const people = new Map<string, number>();
    for (const event of group) {
        const person = event.entry.attributes[PERSON_ATTRIBUTE];
        if (typeof person === "string" && person !== "") {
            people.set(person, Math.max(people.get(person) ?? 0, timeOf(event)));
        }
    }

    return people;
}

/** Key an issue or an event by its installation and fingerprint. */
function keyOf(grouped: Pick<Issue, "installationId" | "fingerprint"> | IssueEvent): string {
    const installationId = "entry" in grouped ? grouped.entry.installation : grouped.installationId;

    return `${installationId ?? ""}\0${grouped.fingerprint}`;
}

/** Report whether a resolved or ignored issue reopens: its condition met by the new events, a total count or the time. */
function reopens(
    row: Issue,
    group: readonly IssueEvent[],
    total: number,
    revisions: ReadonlyMap<string, number>,
): boolean {
    // reopen only resolved and ignored issues, by their condition
    const until = row.until ?? { kind: "event" };
    if (row.status !== "resolved" && row.status !== "ignored") {
        return false;
    }
    switch (until.kind) {
        case "event":
            return true;
        case "revision": {
            const after = revisions.get(until.after);

            return group.some(
                (event) =>
                    event.revision !== undefined &&
                    after !== undefined &&
                    event.revision.createdAt > after,
            );
        }
        case "time":
            return group.some((event) => timeOf(event) >= until.at);
        case "count":
            return total >= until.count;
        case "never":
            return false;
    }
}

/** Read the more severe of two levels. */
function severest(left: Issue["level"], right: Issue["level"]): Issue["level"] {
    return ISSUE_LEVELS.indexOf(left) <= ISSUE_LEVELS.indexOf(right) ? left : right;
}
