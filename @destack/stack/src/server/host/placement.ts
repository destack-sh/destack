import { AccessContext, Authorization, Caller } from "@destack/access";
import { MAIN_DATABASE } from "../../declare/index.ts";
import { space, type StackRelease, placement } from "@destack/account/object";
import {
    and,
    Change,
    eq,
    type Database,
    type DatabaseConnection,
    type Table,
    Transaction,
    isNotNull,
    isNull,
} from "@destack/db";
import { Installation, installation } from "@destack/object";
import type { ObjectStore } from "@destack/object/server";
import { Digest, type Identifier, present } from "@destack/schema";
import type { ReconcileController } from "@destack/db";
import { Scope, RequestId } from "@destack/sync";
import { database, rollout, SPACE_PROVIDER } from "../../object/index.ts";
import { STACK_INSTALLATION } from "../../package.ts";
import { SPACE_BASE_PACKAGES } from "../server.ts";

/** The platform's declaration of every space's main database. */
export interface PlacementOptions {
    /** The platform's declaration of every space's main database. */
    readonly main: Database;
}

/** Give birth to the store's space once placed here, with its row and parent scope arrived. */
export class PlacementController implements ReconcileController {
    /** The controller's name in reports. */
    readonly name = "placement";
    /** The replicated placements, space rows and scopes, and the installations. */
    readonly watches: readonly Table[] = [placement.table, space.table, rollout.table, Scope.table];
    /** The space's store. */
    readonly #store: ObjectStore;
    /** The space's database. */
    readonly #database: DatabaseConnection;
    /** The store's space, which the controller gives birth to. */
    readonly #space: Identifier<"space">;
    /** This machine, which gives birth to the space once placed on it. */
    readonly #machine: Identifier<"machine">;
    /** The spaces' main database. */
    readonly #options: PlacementOptions;

    /** Give birth to a store's space once placed on a machine. */
    constructor(
        store: ObjectStore,
        spaceId: Identifier<"space">,
        machine: Identifier<"machine">,
        options: PlacementOptions,
    ) {
        // set the store, its database, the space, the machine and its options
        this.#store = store;
        this.#database = store.database;
        this.#space = spaceId;
        this.#machine = machine;
        this.#options = options;
    }

    /** List the store's space when a change touches it or an account's scope arrived. */
    keys(change: Change): readonly string[] {
        // take the space's placement
        if (Change.of(change, placement.table)) {
            return this.#touching(Change.image(change).id);
        }
        // take the space's replicated row
        else if (Change.of(change, space.table)) {
            return this.#touching(Change.image(change).id);
        }
        // take the space's rollouts, its stack's among them
        else if (Change.of(change, rollout.table)) {
            return this.#touching(Change.image(change).scope);
        }
        // take an account whose scope arrived, inside which the space may now be born
        else if (Change.of(change, Scope.table)) {
            return Change.image(change).type === "account" ? [this.#space] : [];
        }
        // select nothing for any other change
        else {
            return [];
        }
    }

    /** List the store's space if a change's space is it, else nothing. */
    #touching(changed: string): readonly string[] {
        return changed === this.#space ? [this.#space] : [];
    }

    /** List the space while placed here awaiting birth, a main database, or its changed stack. */
    async list(): Promise<readonly string[]> {
        // read the space while placed here with its stack and the stack its installation last followed
        const rows = await this.#database
            .select({
                id: placement.table.id,
                born: Scope.table.scope,
                main: database.table.id,
                stack: space.table.stack,
                followed: rollout.table.spaceStack,
            })
            .from(placement.table)
            .innerJoin(space.table, eq(space.table.id, placement.table.id))
            .leftJoin(Scope.table, eq(Scope.table.scope, space.table.id))
            .leftJoin(
                database.table,
                and(
                    eq(database.table.scope, placement.table.id),
                    eq(database.table.name, MAIN_DATABASE),
                ),
            )
            .leftJoin(
                installation.table,
                and(
                    eq(installation.table.scope, placement.table.id),
                    isNotNull(installation.table.export),
                ),
            )
            .leftJoin(rollout.table, eq(rollout.table.id, installation.table.id))
            .where(
                and(
                    eq(placement.table.id, this.#space),
                    eq(placement.table.machineId, this.#machine),
                    isNull(space.table.deletedAt),
                ),
            );

        // select it unborn, without a main database, or with a stack changed since its installation followed it
        const { crypto } = this.#database.state.environment;
        const listed = await Promise.all(
            rows.map(async (row) =>
                row.born === null ||
                row.main === null ||
                (await Digest.json(row.stack, crypto)) !== row.followed
                    ? [row.id]
                    : [],
            ),
        );

        return listed.flat();
    }

    /** Give birth to the placed space from its replicated row, and install its stack. */
    async reconcile(key: string): Promise<undefined> {
        // leave a space not placed here, a deleting one, and one whose row or parent scope has not arrived
        const spaceId = space.identifier(key);
        const [placed] = await this.#database
            .select({ born: Scope.table.scope, parent: space.table.scope })
            .from(placement.table)
            .innerJoin(space.table, eq(space.table.id, placement.table.id))
            .leftJoin(Scope.table, eq(Scope.table.scope, space.table.id))
            .where(
                and(
                    eq(placement.table.id, spaceId),
                    eq(placement.table.machineId, this.#machine),
                    isNull(space.table.deletedAt),
                ),
            );
        if (placed === undefined) {
            return undefined;
        }

        // open the space once its parent's scope arrived
        if (placed.born === null) {
            const [parent] = await this.#database
                .select({ ancestors: Scope.table.ancestors })
                .from(Scope.table)
                .where(eq(Scope.table.scope, placed.parent));
            if (parent === undefined) {
                return undefined;
            }
            await this.#open(spaceId, [placed.parent, ...parent.ancestors]);
        }

        // create its main database as the space
        const [main] = await this.#database
            .select({ id: database.table.id })
            .from(database.table)
            .where(and(eq(database.table.scope, spaceId), eq(database.table.name, MAIN_DATABASE)));
        if (main === undefined) {
            await this.#createMain(spaceId);
        }

        // follow the space's stack once it changed, leaving a stack its owner applied since
        const [received] = await this.#database
            .select({
                stack: space.table.stack,
                installed: installation.table.id,
                installedPackage: installation.table.packageId,
                followed: rollout.table.spaceStack,
            })
            .from(space.table)
            .leftJoin(
                installation.table,
                and(
                    eq(installation.table.scope, space.table.id),
                    isNotNull(installation.table.export),
                ),
            )
            .leftJoin(rollout.table, eq(rollout.table.id, installation.table.id))
            .where(eq(space.table.id, spaceId));
        const row = present(received, `the row of ${spaceId}`);
        const digest = await Digest.json(row.stack, this.#database.state.environment.crypto);
        if (digest !== row.followed) {
            await this.#follow(spaceId, row.stack, digest, row.installed, row.installedPackage);
        }

        return undefined;
    }

    /** Receive a placed space at its primary below its ancestors, with its base installations. */
    async #open(spaceId: Identifier<"space">, ancestors: readonly string[]): Promise<void> {
        // read the account the space lives in
        const [row] = await this.#database
            .select({ scope: space.table.scope })
            .from(space.table)
            .where(eq(space.table.id, spaceId));
        const replicated = present(row, `the row of ${spaceId}`);
        const now = this.#store.clock.now();
        // record its scope here, its primary, born with its base installations
        const scope = space.reference(replicated.scope, spaceId);
        await Authorization.run(
            this.#store.authorizer,
            this.#database,
            () => AccessContext.of(Caller.of(this.#store.principal), now),
            this.#store.environment,
            (authorization) =>
                Installation.receive(authorization, scope, ancestors, SPACE_BASE_PACKAGES),
        );
    }

    /** Create a space's main database where its stack places it. */
    async #createMain(spaceId: Identifier<"space">): Promise<void> {
        // describe the main database every space has in its own storage
        const { main } = this.#options;
        await Transaction.run(this.#database, async (transaction) => {
            // create the main database as the space
            await this.#store.mutate(
                this.#store.as(database),
                {
                    id: RequestId.create(this.#store.environment),
                    calls: [
                        database.call("create", {
                            ...database.in(spaceId),
                            name: MAIN_DATABASE,
                            ...main.declare({
                                placement: { provider: SPACE_PROVIDER },
                                retention: "forever",
                            }),
                        }),
                    ],
                },
                { transaction, scope: spaceId },
            );
        });
    }

    /** Create the space's stack installation following its stack, or point it at a changed one. */
    async #follow(
        spaceId: Identifier<"space">,
        stack: StackRelease,
        digest: Digest,
        installed: Identifier<"installation"> | null,
        installedPackage: string | null,
    ): Promise<void> {
        // refuse moving a space to another stack package, which its installation cannot follow
        if (installedPackage !== null && installedPackage !== stack.packageId) {
            throw new TypeError(
                `the stack of ${spaceId} moved from ${installedPackage} to ${stack.packageId}`,
            );
        }

        // install the stack package once, else follow the changed release and export
        const followed = { selection: stack.selection, export: stack.export };
        const [created] = await this.#store.mutate(this.#store.as(rollout), {
            id: RequestId.create(this.#store.environment),
            calls: [
                installed === null
                    ? installation.call("create", {
                          ...installation.in(spaceId),
                          packageId: stack.packageId,
                          ...followed,
                          parameters: {},
                          name: STACK_INSTALLATION,
                      })
                    : installation.call("update", {
                          ...installation.target({ scope: spaceId, id: installed }),
                          ...followed,
                      }),
            ],
        });

        // record the stack followed and drop an owner's earlier apply, so one its owner applies later stays
        const id = installed ?? present(created, `the stack of ${spaceId}`).id;
        await this.#store.mutate(this.#store.as(rollout), {
            id: RequestId.create(this.#store.environment),
            calls: [
                rollout.call("follow", {
                    ...rollout.target({ scope: spaceId, id }),
                    spaceStack: digest,
                }),
            ],
        });
    }
}
