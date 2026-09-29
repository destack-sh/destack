import { accessRelationship, type Authorizer, principal } from "@destack/access";
import type { DatabaseConnection } from "@destack/db";
import { Snapshot } from "@destack/db/log";
import type { Directory, Zone } from "@destack/directory";
import { ServiceError } from "@destack/service/error";
import { implement, type ServiceContext } from "@destack/service/server";
import { Feed, type Query, type RowChange, Scope } from "@destack/sync";
import { Condition } from "@destack/db/query";
import { identifier } from "@destack/schema";
import { and, eq, inArray, TABLE } from "@destack/db";
import { account } from "../../object/account.ts";
import { user } from "../../object/user.ts";
import { membership } from "../../object/membership.ts";
import { Handle, Profile, replica } from "../../service/access/index.ts";
import { Sender } from "../directory/index.ts";

/** Typed implementations of the access replication procedures. */
const implementation = implement(replica).$context<ServiceContext>();

/** Serve access replication to the cell serving a space. */
export function replicaRouter(
    feed: Feed,
    directory: Directory,
    database: DatabaseConnection,
    authorizer: Authorizer,
) {
    // follow users and their personal accounts as public profiles and handles
    const profiles = new Feed(database, [user.table, account.table]);

    return implementation.router({
        watch: implementation.watch.handler(async function* ({ input, context }) {
            // serve the cell of a given space its account, else a host the account it serves
            const below =
                input.spaceId === undefined
                    ? (await Sender.of(context, database)).host.scope
                    : (await requireCell(context, directory, database, input.spaceId)).scope;

            // serve only that account and the scopes containing it
            const chain = await Scope.chain(Snapshot.live(feed.database), below);
            if (!chain.some((link) => link.object.id === input.scope)) {
                throw new ServiceError("FORBIDDEN", {
                    message: `${input.scope} does not contain ${below}`,
                });
            }

            // stream the follower's copy of the scope, ending at a completed page once drained
            const copy = authorizer.replicaOf(input.scope, input.held, input.copied);
            yield* feed.subscribe(copy.queries, input.after, context.request.signal, {
                drain: context.signal,
            });
        }),
        profiles: implementation.profiles.handler(async function* ({ input, context }) {
            // serve exactly the host or region serving the space, the profiles of its account's users and of users who joined it
            const zone = await requireCell(context, directory, database, input.spaceId);
            const users = await visibleUsers(database, zone, input.users);

            // stream the followed users and personal accounts from the copy's position
            const pages = profiles.subscribe(
                profileQueries(users),
                input.after,
                context.request.signal,
                {
                    drain: context.signal,
                    ...(input.previous === undefined
                        ? {}
                        : {
                              previous: profileQueries(
                                  await visibleUsers(database, zone, input.previous),
                              ),
                          }),
                },
            );
            for await (const page of pages) {
                // project users to profiles and accounts to handles
                const [users, accounts] = [user, account].map((object) =>
                    page.changes.filter((change) => change.table === object.table[TABLE].sqlName),
                ) as [RowChange[], RowChange[]];
                const kept = (changes: RowChange[]) =>
                    changes.filter((change) => change.operation !== "delete");
                const left = (changes: RowChange[]) =>
                    changes.filter((change) => change.operation === "delete");
                yield {
                    reset: page.reset,
                    complete: page.complete,
                    position: page.position,
                    profiles: kept(users).map(({ row }) =>
                        Profile.parse({ id: row.id, name: row.name, image: row.image ?? null }),
                    ),
                    removed: left(users).map(({ row }) => identifier("user").parse(row.id)),
                    handles: kept(accounts).map(({ row }) =>
                        Handle.parse({ accountId: row.id, userId: row.scope, handle: row.handle }),
                    ),
                    unhandled: left(accounts).map(({ row }) => identifier("account").parse(row.id)),
                };
            }
        }),
    });
}

/** Keep the users whose profiles a space's cell may read: those with access in its account, and those who joined the space. */
async function visibleUsers(
    database: DatabaseConnection,
    zone: Zone,
    users: readonly string[],
): Promise<string[]> {
    // read the users related to anything in the space's account
    const related = await database
        .selectDistinct({ id: accessRelationship.subjectId })
        .from(accessRelationship)
        .where(
            and(
                eq(accessRelationship.scope, zone.scope),
                eq(accessRelationship.subjectPackageId, principal.user.definition.packageId),
                eq(accessRelationship.subjectType, principal.user.name),
                inArray(accessRelationship.subjectId, [...users]),
            ),
        );

    // read the users who joined the space themselves
    const joined = await database
        .select({ id: membership.table.scope })
        .from(membership.table)
        .where(
            and(
                eq(membership.table.spaceId, identifier("space").parse(zone.id)),
                inArray(
                    membership.table.scope,
                    users.map((id) => identifier("user").parse(id)),
                ),
            ),
        );

    return [...new Set([...related, ...joined].map((entry) => String(entry.id)))];
}

/** Select some users and their personal accounts. */
function profileQueries(users: readonly string[]): Record<string, Query> {
    return {
        profiles: {
            table: user.table,
            scopes: [Scope.universe.id],
            where: Condition.oneOf("id", [...users]),
        },
        handles: {
            table: account.table,
            scopes: users,
            where: Condition.eq("kind", "personal"),
        },
    };
}

/** Require the caller to act for the cell serving a space's zone. */
async function requireCell(
    context: ServiceContext,
    directory: Directory,
    database: DatabaseConnection,
    spaceId: string,
): Promise<Zone> {
    // locate the space and refuse a caller acting for another cell
    const sender = await Sender.of(context, database);
    const zone = await directory.locate(spaceId);
    if (zone === undefined) {
        throw new ServiceError("NOT_FOUND", { message: `${spaceId} is not found` });
    } else if (!sender.cells.has(zone.cell)) {
        throw new ServiceError("FORBIDDEN", {
            message: `${spaceId} is served by another host or region`,
        });
    }

    return zone;
}
