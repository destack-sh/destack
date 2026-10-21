import type { Call } from "@destack/sync";
import { EventTime } from "@destack/event";
import { and, eq, inArray, Snapshot } from "@destack/db";
import type { ObjectStore } from "@destack/object/server";
import { type Digest, type Identifier, present } from "@destack/schema";
import { build, installation, installationRevision } from "@destack/stack/object";
import { ISSUE_LEVELS, issue, type Issue, issuePerson } from "../object/index.ts";
import type { Exception } from "./exception.ts";
import { RequestId } from "@destack/sync";

/** The component name the grouper records its calls under. */
const GROUPER = "issues";

/** An unexpected exception of an installation that a log record or a span recorded. */
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

/** One exception grouped into an issue. */
interface IssueEvent {
    /** The occurrence. */
    readonly occurrence: Occurrence;
    /** Its grouping. */
    readonly fingerprint: string;
    /** The installation revision that ran it, absent when unknown. */
    readonly revision: Revision | undefined;
}

/** An installation revision an exception ran: its identifier and when it was made. */
interface Revision {
    /** The revision. */
    readonly id: Identifier<"installation-revision">;
    /** When it was made, in Unix milliseconds. */
    readonly createdAt: number;
}

/** Groups unexpected exceptions into a space's issues. */
export class Grouper {
    /** The object store of the issues, beside replicas of the installations and their revisions. */
    readonly #objects: Pick<ObjectStore, "database" | "serving" | "mutate">;

    /** Group exceptions into the issues an object store keeps. */
    constructor(objects: Pick<ObjectStore, "database" | "serving" | "mutate">) {
        this.#objects = objects;
    }

    /** Group a space's unexpected exceptions into its issues. */
    async group(
        scope: Identifier<"space">,
        occurrences: readonly Occurrence[],
        now: number,
    ): Promise<ReadonlyMap<Occurrence, Identifier<"issue">>> {
        // resolve the unexpected exceptions into events at their revisions, grouped by installation and fingerprint
        const unexpected = occurrences.filter((occurrence) => !occurrence.exception.isExpected);
        const built = await this.#revisions(scope, unexpected);
        const events = await Promise.all(
            unexpected.map(async (occurrence) => ({
                occurrence,
                fingerprint: await occurrence.exception.fingerprint(),
                revision: built.get(`${occurrence.installation} ${occurrence.build}`),
            })),
        );
        const groups = [
            ...Map.groupBy(events, (event) =>
                issueKey(event.occurrence.installation, event.fingerprint),
            ).values(),
        ].map((group) => new IssueGroup(group));
        if (groups.length === 0) {
            return new Map();
        }

        // open new issues and count the events and people into every issue
        const { issues, counted } = await this.#count(scope, groups, now);

        // regress resolved issues and escalate ignored ones whose condition the events meet
        const revisions = await this.#revisionsOf(counted.map(({ row }) => row));
        const reopened = counted.filter(({ row, group }) => group.reopens(row, revisions));
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
            groups.flatMap((group) => {
                const found = issues.get(group.key);

                return found === undefined
                    ? []
                    : group.events.map((event): [Occurrence, Identifier<"issue">] => [
                          event.occurrence,
                          found.id,
                      ]);
            }),
        );
    }

    /** Open the issues the grouped events are new to. */
    async #count(scope: Identifier<"space">, groups: readonly IssueGroup[], now: number) {
        // open new issues
        const known = await this.#known(scope, groups);
        await this.#objects.mutate(
            this.#objects.serving(issue),
            {
                id: RequestId.create(now),
                calls: groups
                    .filter((group) => !known.has(group.key))
                    .map((group) => issue.call("open", { ...issue.in(scope), ...group.opening })),
            },
            { component: GROUPER },
        );

        // count the events and people into every issue
        const issues = await this.#known(scope, groups);
        const counted = groups.flatMap((group) => {
            const row = issues.get(group.key);

            return row === undefined ? [] : [{ row, group }];
        });
        const { calls, people } = await this.#affect(counted);
        await this.#objects.mutate(
            this.#objects.serving(issue),
            {
                id: RequestId.create(now),
                calls: [
                    ...calls,
                    ...counted.map(({ row, group }) => {
                        const { latest } = group;

                        return issue.call("occur", {
                            ...issue.target(row),
                            level: severest(group.level, row.level),
                            lastSeenAt: Math.max(row.lastSeenAt, group.lastSeenAt),
                            ...(latest === undefined ? {} : { lastRevisionId: latest.id }),
                            count: row.count + group.events.length,
                            people: people.get(row.id) ?? row.people,
                        });
                    }),
                ],
            },
            { component: GROUPER },
        );

        return { issues, counted };
    }

    /** Read the revisions the occurrences ran, keyed by installation and build manifest. */
    async #revisions(
        scope: Identifier<"space">,
        occurrences: readonly Occurrence[],
    ): Promise<Map<string, Revision>> {
        // read the revisions of the occurrences' installations from the replicas in one query
        const installations = [
            ...new Set(
                occurrences.flatMap((each) =>
                    each.build === undefined ? [] : [each.installation],
                ),
            ),
        ];
        if (installations.length === 0) {
            return new Map();
        }
        const rows = await this.#objects.database
            .select({
                id: installationRevision.table.id,
                createdAt: installationRevision.table.createdAt,
                installationId: installationRevision.table.installationId,
                manifest: build.table.manifest,
            })
            .from(installationRevision.table)
            .innerJoin(
                installation.table,
                eq(installation.table.id, installationRevision.table.installationId),
            )
            .innerJoin(build.table, eq(build.table.id, installationRevision.table.buildId))
            .where(
                and(
                    eq(installation.table.scope, scope),
                    inArray(installation.table.id, installations),
                ),
            );

        return new Map(
            rows.map((row) => [
                `${row.installationId} ${row.manifest}`,
                { id: row.id, createdAt: row.createdAt },
            ]),
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
        await this.#objects.mutate(
            this.#objects.serving(issue),
            {
                id: RequestId.create(now),
                calls: rows.map((row) => issue.call(name, issue.target(row))),
            },
            { component: GROUPER },
        );
    }

    /** Read a space's issues of events by installation and fingerprint. */
    async #known(
        scope: Identifier<"space">,
        groups: readonly IssueGroup[],
    ): Promise<Map<string, Issue>> {
        const rows = await this.#objects.database
            .select()
            .from(issue.table)
            .where(
                and(
                    eq(issue.table.scope, scope),
                    inArray(
                        issue.table.fingerprint,
                        groups.map((group) => group.fingerprint),
                    ),
                ),
            );

        return new Map(rows.map((row) => [issueKey(row.installationId, row.fingerprint), row]));
    }

    /** Plan the people each issue's events affected: the calls keeping them, and their counts. */
    async #affect(
        counted: readonly { readonly row: Issue; readonly group: IssueGroup }[],
    ): Promise<{ calls: Call[]; people: Map<string, number> }> {
        // read the people the issues affected before
        const known = await Snapshot.live(this.#objects.database)
            .read(issuePerson.table)
            .findMany({
                where: {
                    parentId: { in: counted.map(({ row }) => row.id) },
                },
            });
        const byIssue = Map.groupBy(known, (row) => row.parentId);

        // keep each person new to an issue, and the later time of each known one
        const calls: Call[] = [];
        const people = new Map<string, number>();
        for (const { row, group } of counted) {
            const kept = new Map((byIssue.get(row.id) ?? []).map((each) => [each.person, each]));
            const affected = group.people;
            for (const [person, lastSeenAt] of affected) {
                const found = kept.get(person);
                if (found === undefined) {
                    calls.push(
                        issuePerson.call("create", {
                            ...issuePerson.in(row.scope),
                            parentId: row.id,
                            person,
                            lastSeenAt,
                        }),
                    );
                } else if (lastSeenAt > found.lastSeenAt) {
                    calls.push(
                        issuePerson.call("update", { ...issuePerson.target(found), lastSeenAt }),
                    );
                }
            }

            // count the people known and new
            const added = [...affected.keys()].filter((person) => !kept.has(person)).length;
            people.set(row.id, kept.size + added);
        }

        return { calls, people };
    }
}

/** The events of one installation and fingerprint, which one issue counts. */
class IssueGroup {
    /** The events, at least one. */
    readonly events: readonly IssueEvent[];
    /** The first event. */
    readonly #first: IssueEvent;

    /** Group events of one installation and fingerprint. */
    constructor(events: readonly IssueEvent[]) {
        this.events = events;
        this.#first = present(events[0], "an issue group's first event");
    }

    /** The key of the group's issue: its installation and fingerprint. */
    get key(): string {
        return issueKey(this.#first.occurrence.installation, this.#first.fingerprint);
    }

    /** The fingerprint. */
    get fingerprint(): string {
        return this.#first.fingerprint;
    }

    /** The most severe level of the events. */
    get level(): Issue["level"] {
        return this.events.map((event) => event.occurrence.exception.level).reduce(severest);
    }

    /** When the latest event happened, in Unix milliseconds. */
    get lastSeenAt(): number {
        return Math.max(...this.events.map(timeOf));
    }

    /** The latest revision the events ran. */
    get latest(): Revision | undefined {
        return this.#revisions().reduce<Revision | undefined>(
            (found, revision) =>
                found === undefined || revision.createdAt > found.createdAt ? revision : found,
            undefined,
        );
    }

    /** The people the events affected, with the latest time each was. */
    get people(): Map<string, number> {
        const people = new Map<string, number>();
        for (const event of this.events) {
            const { person } = event.occurrence;
            if (person !== null && person !== "") {
                people.set(person, Math.max(people.get(person) ?? 0, timeOf(event)));
            }
        }

        return people;
    }

    /** The input opening the group's issue, which the events are counted into after. */
    get opening() {
        // read the first event's title, its innermost in-app symbol and an in-app frame's declaration
        const { exception, installation: installationId } = this.#first.occurrence;
        const inApp = exception.frames.filter((frame) => frame.isInApp);
        const culprit = inApp.find((frame) => frame.moniker !== undefined)?.moniker;
        const declaration = inApp.find((frame) => frame.declaration !== undefined)?.declaration;
        const firstLine = exception.message.split("\n")[0] ?? "";

        // span the events' times and revisions
        const earliest = this.#revisions().reduce<Revision | undefined>(
            (found, revision) =>
                found === undefined || revision.createdAt < found.createdAt ? revision : found,
            undefined,
        );
        const { latest } = this;

        return {
            installationId,
            fingerprint: this.#first.fingerprint,
            errorType: exception.errorType,
            title: `${exception.type ?? exception.errorType}: ${firstLine}`.slice(0, 500),
            ...(culprit === undefined ? {} : { culprit }),
            ...(declaration === undefined ? {} : { declaration: declaration.moniker }),
            level: this.level,
            firstSeenAt: Math.min(...this.events.map(timeOf)),
            lastSeenAt: this.lastSeenAt,
            ...(earliest === undefined ? {} : { firstRevisionId: earliest.id }),
            ...(latest === undefined ? {} : { lastRevisionId: latest.id }),
        };
    }

    /** Report whether the group reopens a resolved or ignored issue, by the issue's condition. */
    reopens(row: Issue, revisions: ReadonlyMap<string, number>): boolean {
        // reopen only resolved and ignored issues
        const { until } = row;
        if (row.status !== "resolved" && row.status !== "ignored") {
            return false;
        }

        // reopen by the condition: any event, a later revision, a time, or a count
        switch (until.kind) {
            case "event":
                return true;
            case "revision": {
                const after = revisions.get(until.after);

                return this.events.some(
                    (event) =>
                        event.revision !== undefined &&
                        after !== undefined &&
                        event.revision.createdAt > after,
                );
            }
            case "time":
                return this.events.some((event) => timeOf(event) >= until.at);
            case "count":
                return row.count + this.events.length >= until.count;
            case "never":
                return false;
        }
    }

    /** List the revisions the events ran. */
    #revisions(): Revision[] {
        return this.events.flatMap((event) => event.revision ?? []);
    }
}

/** Read an event's time in Unix milliseconds. */
function timeOf(event: IssueEvent): number {
    return EventTime.milliseconds(event.occurrence.time);
}

/** Key an issue by its installation and fingerprint. */
function issueKey(installationId: string, fingerprint: string): string {
    return `${installationId}\0${fingerprint}`;
}

/** Read the more severe of two levels. */
function severest(left: Issue["level"], right: Issue["level"]): Issue["level"] {
    return ISSUE_LEVELS.indexOf(left) <= ISSUE_LEVELS.indexOf(right) ? left : right;
}
