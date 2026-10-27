import { and, asc, eq, inArray } from "@destack/db";
import { Event, EventTime } from "@destack/event";
import { installation } from "@destack/object";
import type { ObjectStore } from "@destack/object/server";
import { Digest, Identifier, present } from "@destack/schema";
import { build, installationRevision } from "@destack/stack/object";
import {
    type ExceptionData,
    Issue,
    ISSUE_NAMESPACE,
    ISSUE_TITLE_LENGTH,
    type IssueOpening,
} from "../object/index.ts";
import type { Exception } from "./exception.ts";

/** The kind an exception's event identity derives under, apart from its record's. */
const EXCEPTION_EVENT = "exception";

/** An unexpected exception of an installation that a log record or a span recorded. */
export interface Occurrence {
    /** The identity counting it once: the log record or span carrying it, which retries repeat. */
    readonly identity: string;
    /** The exception. */
    readonly exception: Exception;
    /** The installation that ran it. */
    readonly installation: Identifier<"installation">;
    /** The digest of the build that ran it, null when unknown. */
    readonly build: Digest | null;
    /** The person it happened to, null when unknown. */
    readonly person: string | null;
    /** When it happened, in Unix microseconds. */
    readonly time: number;
}

/** One exception grouped into an issue. */
interface IssueEvent {
    /** The occurrence. */
    readonly occurrence: Occurrence;
    /** The event identity its exception is appended once under. */
    readonly id: string;
    /** Its grouping. */
    readonly fingerprint: Digest;
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

/** An exception as its events append it. */
export interface ExceptionItem {
    /** The event identity, the same across exports of the exception. */
    readonly id: string;
    /** When it happened, in Unix microseconds. */
    readonly time: number;
    /** The issue it is grouped into. */
    readonly issue: Identifier<"issue">;
    /** The installation that ran it. */
    readonly installationId: Identifier<"installation">;
    /** The person it happened to, null when unknown. */
    readonly person: string | null;
    /** Its issue's opening and its revision's time. */
    readonly data: ExceptionData;
}

/** A space's exceptions grouped into issues, as the events appending them. */
export interface Grouping {
    /** The exceptions to append, which the issues' reader counts. */
    readonly exceptions: readonly ExceptionItem[];
    /** The issue of each unexpected occurrence. */
    readonly issues: ReadonlyMap<Occurrence, Identifier<"issue">>;
}

/** Groups unexpected exceptions into a space's issues. */
export class Grouper {
    /** The object store of the issues, beside replicas of the installations and their revisions. */
    readonly #objects: Pick<ObjectStore, "database" | "environment">;

    /** Group exceptions into the issues an object store keeps. */
    constructor(objects: Pick<ObjectStore, "database" | "environment">) {
        this.#objects = objects;
    }

    /** Group a space's unexpected exceptions into issues, recorded by the calls it answers. */
    async group(scope: Identifier<"space">, occurrences: readonly Occurrence[]): Promise<Grouping> {
        // resolve the unexpected exceptions into events at their revisions, grouped by installation and fingerprint
        const unexpected = occurrences.filter((occurrence) => !occurrence.exception.isExpected);
        const built = await this.#revisions(scope, unexpected);
        const { crypto } = this.#objects.environment;
        const events = await Promise.all(
            unexpected.map(async (occurrence) => ({
                occurrence,
                id: await Event.identity(scope, [EXCEPTION_EVENT, occurrence.identity], crypto),
                fingerprint: await occurrence.exception.fingerprint(crypto),
                revision: built.get(`${occurrence.installation} ${occurrence.build}`),
            })),
        );
        const groups = [
            ...Map.groupBy(events, (event) =>
                issueKey(event.occurrence.installation, event.fingerprint),
            ).values(),
        ].map((group) => new IssueGroup(group));
        if (groups.length === 0) {
            return { exceptions: [], issues: new Map() };
        }

        // record each group into the issue its installation and fingerprint derive
        const recorded = await Promise.all(
            groups.map(async (group) => ({ group, id: await this.#identify(scope, group) })),
        );

        return {
            exceptions: recorded.flatMap(({ group, id }) => group.exceptions(id)),
            issues: new Map(
                recorded.flatMap(({ group, id }) =>
                    group.events.map((event): [Occurrence, Identifier<"issue">] => [
                        event.occurrence,
                        id,
                    ]),
                ),
            ),
        };
    }

    /** Derive a group's issue identifier from its space, installation and fingerprint. */
    #identify(scope: Identifier<"space">, group: IssueGroup): Promise<Identifier<"issue">> {
        const name = [scope, group.installation, group.fingerprint].join(" ");

        return Identifier.hash("issue", ISSUE_NAMESPACE, name, this.#objects.environment.crypto);
    }

    /** Read the revisions the occurrences ran, keyed by installation and build manifest. */
    async #revisions(
        scope: Identifier<"space">,
        occurrences: readonly Occurrence[],
    ): Promise<Map<string, Revision>> {
        // read the revisions of the occurrences' installations and builds from the replicas in one query
        const built = occurrences.filter((each) => each.build !== null);
        const installations = [...new Set(built.map((each) => each.installation))];
        const manifests = [...new Set(built.flatMap((each) => each.build ?? []))];
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
                    inArray(build.table.manifest, manifests),
                ),
            )
            .orderBy(asc(installationRevision.table.createdAt), asc(installationRevision.table.id));

        // key each build by its latest revision, as a rollback to a build makes it again
        return new Map(
            rows.map((row) => [
                `${row.installationId} ${row.manifest}`,
                { id: row.id, createdAt: row.createdAt },
            ]),
        );
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

    /** The installation that ran the events. */
    get installation(): Identifier<"installation"> {
        return this.#first.occurrence.installation;
    }

    /** The fingerprint. */
    get fingerprint(): Digest {
        return this.#first.fingerprint;
    }

    /** List the events as the exceptions of their issue, each with the opening it alone gives. */
    exceptions(id: Identifier<"issue">): ExceptionItem[] {
        return this.events.map((event) => ({
            id: event.id,
            time: event.occurrence.time,
            issue: id,
            installationId: event.occurrence.installation,
            person: event.occurrence.person,
            data: {
                opening: new IssueGroup([event]).opening,
                ...(event.revision === undefined ? {} : { revisedAt: event.revision.createdAt }),
            },
        }));
    }

    /** The most severe level of the events. */
    get level(): Issue["level"] {
        return this.events
            .map((event) => event.occurrence.exception.level)
            .reduce((left, right) => Issue.severest(left, right));
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

    /** The fields opening the group's issue at its first events. */
    get opening(): IssueOpening {
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
            title: `${exception.type ?? exception.errorType}: ${firstLine}`.slice(
                0,
                ISSUE_TITLE_LENGTH,
            ),
            ...(culprit === undefined ? {} : { culprit }),
            ...(declaration === undefined ? {} : { declaration: declaration.moniker }),
            level: this.level,
            firstSeenAt: Math.min(...this.events.map(timeOf)),
            lastSeenAt: this.lastSeenAt,
            ...(earliest === undefined ? {} : { firstRevisionId: earliest.id }),
            ...(latest === undefined ? {} : { lastRevisionId: latest.id }),
        };
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
