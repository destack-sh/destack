import { AccessContext, Authorization, Caller } from "@destack/access";
import { declaredStack, MAIN_DATABASE } from "../../declare/index.ts";
import { space, type StackSource, placement } from "@destack/account/object";
import {
    and,
    Change,
    eq,
    type Database,
    type DatabaseConnection,
    type Table,
    Transaction,
    isNull,
} from "@destack/db";
import { Installation } from "@destack/object";
import type { ObjectStore } from "@destack/object/server";
import { SPACE_KERNEL } from "../server.ts";
import { Digest, type Identifier, type JsonObject, present } from "@destack/schema";
import type { ReconcileController } from "@destack/db";
import { Scope, RequestId } from "@destack/sync";
import { database, rollout, type Rollout, SPACE_PROVIDER } from "../../object/index.ts";
import { BuildStore, type Registry } from "../installation/build.ts";
import type { PackageId } from "@destack/package";

/** The name of the installation applying the stack of a space from its creation. */
const STACK_INSTALLATION = "stack";

/** The build store, the spaces' main database, and the registry the stacks' releases copy from. */
export interface PlacementOptions {
    /** The machine's store of every build. */
    readonly builds: BuildStore;
    /** The registry the stacks' releases copy from. */
    readonly registry: Registry;
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
    /** The build store, the spaces' main database and the release registry. */
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
        // take the space's rollout
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
        // read the space while placed here with its stack and the stack last submitted
        const rows = await this.#database
            .select({
                id: placement.table.id,
                born: Scope.table.scope,
                main: database.table.id,
                stack: space.table.stack,
                submitted: rollout.table.spaceStack,
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
                rollout.table,
                and(eq(rollout.table.scope, placement.table.id), eq(rollout.table.kind, "stack")),
            )
            .where(
                and(
                    eq(placement.table.id, this.#space),
                    eq(placement.table.machineId, this.#machine),
                    isNull(space.table.deletedAt),
                ),
            );

        // select it unborn, without a main database, or with a stack changed since its last submission
        const listed = await Promise.all(
            rows.map(async (row) =>
                row.born === null ||
                row.main === null ||
                (await Digest.json(row.stack, this.#database.state.environment.crypto)) !==
                    row.submitted
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

        // submit the space's stack once it changed since its last submission
        const [received] = await this.#database
            .select({ stack: space.table.stack, installed: rollout.table })
            .from(space.table)
            .leftJoin(
                rollout.table,
                and(eq(rollout.table.scope, space.table.id), eq(rollout.table.kind, "stack")),
            )
            .where(eq(space.table.id, spaceId));
        const { stack, installed } = present(received, `the row of ${spaceId}`);
        const digest = await Digest.json(stack, this.#database.state.environment.crypto);
        if (digest !== installed?.spaceStack) {
            await this.#install(spaceId, stack, digest, installed);
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
        await Transaction.run(this.#database, async (transaction) => {
            // record its scope here, its primary, born with its base installations
            const authorization = new Authorization(
                this.#store.authorizer,
                transaction,
                () => AccessContext.of(Caller.of(this.#store.principal), now),
                this.#store.environment,
            );
            const scope = space.reference(replicated.scope, spaceId);
            await Installation.receive(authorization, scope, ancestors, SPACE_KERNEL, this.#store);
        });
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

    /** Copy a stack's pinned build into the build store and read the space its export declares. */
    async #declared(stack: StackSource): Promise<JsonObject> {
        // copy the stack's build, evaluate its export, and read its installations' builds without storing them
        const { builds, registry } = this.#options;
        const reader = await builds.pull(registry, stack.package.id, stack.manifest);
        const read = (packageId: PackageId, manifest: Digest) =>
            builds.pull(registry, packageId, manifest);

        return declaredStack(reader, stack.export, stack.settings, read);
    }

    /** Install a space's stack at its pinned build, and submit the space its export declares. */
    async #install(
        spaceId: Identifier<"space">,
        stack: StackSource,
        digest: Digest,
        installed: Rollout | null,
    ): Promise<void> {
        // read the space the stack declares at its pinned build
        const { package: stacked, manifest } = stack;
        const build = { kind: "release" as const, version: stacked.version, manifest };
        const definition = await this.#declared(stack);

        // install the stack package as the space's stack once
        const selection = { kind: "release" as const, version: stacked.version };
        const [created] =
            installed === null
                ? await this.#store.mutate(this.#store.as(rollout), {
                      id: RequestId.create(this.#store.environment),
                      calls: [
                          rollout.call("create", {
                              ...rollout.in(spaceId),
                              packageId: stacked.id,
                              kind: "stack",
                              selection,
                              export: stack.export,
                              parameters: {},
                              name: STACK_INSTALLATION,
                          }),
                      ],
                  })
                : [installed];

        // submit its pinned build in a mutation of its own, whose preparation reads the created stack
        await this.#store.mutate(this.#store.as(rollout), {
            id: RequestId.create(this.#store.environment),
            calls: [
                rollout.call("submit", {
                    ...rollout.target(present(created, `the stack of ${spaceId}`)),
                    selection,
                    build,
                    evaluation: { export: stack.export, parameters: {}, definition },
                    spaceStack: digest,
                }),
            ],
        });
    }
}
