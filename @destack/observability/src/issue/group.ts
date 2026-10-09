import { EventTime } from "@destack/event";
import { and, count, eq, inArray, sql, type DatabaseConnection } from "@destack/db";
import { ServerCall } from "@destack/object";
import type { ObjectServer } from "@destack/object/server";
import { BuildCache } from "@destack/package/manifest";
import type { Digest, Identifier } from "@destack/schema";
import { installation, installationRevision } from "@destack/space/object";
import type { OpenBuild } from "@destack/space/server";
import { ISSUE_LEVELS, issue, type Issue } from "../object/index.ts";
import { issuePerson } from "../stack/db.ts";
import { Exception } from "./exception.ts";
import { fingerprint } from "./fingerprint.ts";
import { parseStack, type StackFrame } from "./frame.ts";
import { Symbolicator } from "./symbolicate.ts";

/** The component name the grouper records its calls under. */
const GROUPER = "issues";

/** An unexpected exception of an installation a log record or a span recorded, as the grouper reads it. */
export interface Occurrence {
    /** The exception. */
    readonly exception: Exception;
    /** The installation that ran it. */
    readonly installation: Identifier<"installation">;
    /** The digest of the build that ran it, absent when unknown. */
    readonly build: Digest | undefined;
    /** The person it happened to, absent when unknown. */
    readonly person: string | null;
    /** When it happened, in Unix microseconds. */
    readonly time: number;
}

/** One exception grouped into an issue: its occurrence, frames, fingerprint and the revision that ran it. */
interface IssueEvent {
    /** The occurrence. */
    readonly occurrence: Occurrence;
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
    readonly #objects: Pick<ObjectServer, "database" | "principal" | "execute">;
    /** Open an installation revision's build. */
    readonly #open: OpenBuild;
    /** The symbolicators of the builds read recently. */
    readonly #symbolicators = new BuildCache(async (reader) => new Symbolicator(reader));

    /** Group exceptions into the issues an object server keeps, opening the builds of the revisions that ran them. */
    constructor(
        objects: Pick<ObjectServer, "database" | "principal" | "execute">,
        open: OpenBuild,
    ) {
        this.#objects = objects;
        this.#open = open;
    }

    /** Group a space's unexpected exceptions into its issues, opening, counting, regressing and escalating them, and answer each occurrence's issue. */
    async group(
        scope: Identifier<"space">,
        occurrences: readonly Occurrence[],
        now: number,
    ): Promise<ReadonlyMap<Occurrence, Identifier<"issue">>> {
        // resolve the unexpected exceptions into events, grouped by installation and fingerprint
        const events = await Promise.all(
            occurrences
                .filter((occurrence) => !Exception.isExpected(occurrence.exception))
                .map((occurrence) => this.#event(scope, occurrence)),
        );
        const grouped = Map.groupBy(events, keyOf);
        if (grouped.size === 0) {
            return new Map();
        }

        // open new issues and count the events and people into every issue
        const { issues, counted } = await this.#count(scope, grouped, now);

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

        // answer each occurrence's issue
        return new Map(
            events.flatMap((event): [Occurrence, Identifier<"issue">][] => {
                const found = issues.get(keyOf(event));

                return found === undefined ? [] : [[event.occurrence, found.id]];
            }),
        );
    }

    /** Open the issues the grouped events are new to, then count each group's events and people into its issue. */
    async #count(scope: Identifier<"space">, grouped: Map<string, IssueEvent[]>, now: number) {
        // open new issues
        const events = [...grouped.values()].flat();
        const known = await this.#known(scope, events);
        await this.#objects.execute(
            this.#objects.principal,
            issue,
            "open",
            [...grouped]
                .filter(([key]) => !known.has(key))
                .map(([, group]) => ({ scope, input: openingOf(group) })),
            now,
            GROUPER,
        );

        // count the events and people into every issue
        const issues = await this.#known(scope, events);
        const counted = [...grouped].flatMap(([key, group]) => {
            const row = issues.get(key);

            return row === undefined ? [] : [{ row, group }];
        });
        const people = await this.#affect(scope, counted);
        await this.#objects.execute(
            this.#objects.principal,
            issue,
            "occur",
            counted.map(({ row, group }) => {
                const latest = latestOf(group);

                return ServerCall.of(row, {
                    level: group.map((event) => event.exception.level).reduce(severest, row.level),
                    lastSeenAt: Math.max(row.lastSeenAt, ...group.map(timeOf)),
                    ...(latest === undefined ? {} : { lastRevisionId: latest.id }),
                    count: row.count + group.length,
                    people: people.get(row.id) ?? row.people,
                });
            }),
            now,
            GROUPER,
        );

        return { issues, counted };
    }

    /** Resolve one occurrence into an event: the revision that ran it, its frames through that revision's build, and its fingerprint. */
    async #event(scope: Identifier<"space">, occurrence: Occurrence): Promise<IssueEvent> {
        // find the revision built from the occurrence's manifest in the copied revisions
        const { exception } = occurrence;
        const found =
            occurrence.build === undefined
                ? undefined
                : await this.#revision(scope, occurrence.installation, occurrence.build);

        // read the frames, mapped and attributed through the revision's build when known
        const frames = parseStack(exception.stacktrace ?? "");
        const resolved =
            found === undefined
                ? frames
                : await (await this.#symbolicators.read(await found.open())).frames(frames);

        return {
            occurrence,
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
            row.until.kind === "revision" ? [row.until.after] : [],
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
        await this.#objects.execute(
            this.#objects.principal,
            issue,
            name,
            rows.map((row) => ServerCall.of(row)),
            now,
            GROUPER,
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
        installationId: first.occurrence.installation,
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
    return EventTime.milliseconds(event.occurrence.time);
}

/** Read the people events affected, with the latest time each was. */
function peopleOf(group: readonly IssueEvent[]): Map<string, number> {
    const people = new Map<string, number>();
    for (const event of group) {
        const { person } = event.occurrence;
        if (person !== null && person !== "") {
            people.set(person, Math.max(people.get(person) ?? 0, timeOf(event)));
        }
    }

    return people;
}

/** Key an issue or an event by its installation and fingerprint. */
function keyOf(grouped: Pick<Issue, "installationId" | "fingerprint"> | IssueEvent): string {
    const installationId =
        "occurrence" in grouped ? grouped.occurrence.installation : grouped.installationId;

    return `${installationId}\0${grouped.fingerprint}`;
}

/** Report whether a resolved or ignored issue reopens: its condition met by the new events, a total count or the time. */
function reopens(
    row: Issue,
    group: readonly IssueEvent[],
    total: number,
    revisions: ReadonlyMap<string, number>,
): boolean {
    // reopen only resolved and ignored issues, by their condition
    const { until } = row;
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
