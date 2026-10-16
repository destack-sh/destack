import { AccessContext, Authorization, Caller } from "@destack/access";
import { account, Space, space, type StackSource } from "@destack/account/object";
import {
    and,
    Change,
    eq,
    type Database,
    type DatabaseConnection,
    type Table,
    Transaction,
} from "@destack/db";
import { placement } from "@destack/account/object";
import type { ObjectStore } from "@destack/object/server";
import { Digest, type Identifier, type JsonObject, present } from "@destack/schema";
import { Provisioned } from "@destack/object";
import type { Controller } from "@destack/runtime/control";
import { Scope } from "@destack/sync";
import {
    database,
    type Installation,
    installation,
    MAIN_DATABASE,
    SPACE_PROVIDER,
} from "../../object/index.ts";
import { BuildStore, type Registry } from "../installation/build.ts";
import type { PackageId } from "@destack/package";
import { declaredStack } from "../installation/stack.ts";
import { RequestId } from "@destack/runtime/request";

/** The name of the installation applying the stack a space carries from its creation. */
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

/** Give birth to each space placed here once its replicated row and parent scope arrive. */
export class PlacementController implements Controller {
    /** The controller's name in reports. */
    readonly name = "placement";
    /** The replicated placements, space rows and scopes, and the installations. */
    readonly watches: readonly Table[] = [
        placement.table,
        space.table,
        installation.table,
        Scope.table,
    ];
    /** The space's store. */
    readonly #store: ObjectStore;
    /** The database of the spaces this machine serves. */
    readonly #database: DatabaseConnection;
    /** This machine, which gives birth to the spaces placed on it. */
    readonly #machine: Identifier<"machine">;
    /** The build store, the spaces' main database and the release registry. */
    readonly #options: PlacementOptions;

    /** Give birth to the spaces placed on a machine through its store. */
    constructor(store: ObjectStore, machine: Identifier<"machine">, options: PlacementOptions) {
        // set the store, its database, the machine and its options
        this.#store = store;
        this.#database = store.database;
        this.#machine = machine;
        this.#options = options;
    }

    /** List the space a change touches, or the account whose scope arrived. */
    keys(change: Change): readonly string[] {
        // take a placement's space
        if (Change.of(change, placement.table)) {
            return [Change.image(change).id];
        }
        // take a replicated space row's space
        else if (Change.of(change, space.table)) {
            return [Change.image(change).id];
        }
        // take the space of an installation
        else if (Change.of(change, installation.table)) {
            return [Change.image(change).scope];
        }
        // take an account whose scope arrived, whose spaces may now be born
        else if (Change.of(change, Scope.table)) {
            const row = Change.image(change);

            return row.type === "account" ? [row.scope] : [];
        }
        // select nothing for any other change
        else {
            return [];
        }
    }

    /** List the spaces placed here awaiting birth or a main database, or whose stack changed. */
    async list(): Promise<readonly string[]> {
        // read the spaces placed here with their stacks and the stacks last submitted
        const rows = await this.#database
            .select({
                id: placement.table.id,
                born: Scope.table.scope,
                main: database.table.id,
                stack: space.table.stack,
                submitted: installation.table.spaceStack,
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
                    eq(installation.table.role, "stack"),
                ),
            )
            .where(eq(placement.table.machineId, this.#machine));

        // select the unborn ones, those without a main database, and those whose stack changed since its last submission
        const listed = await Promise.all(
            rows.map(async (row) =>
                row.born === null ||
                row.main === null ||
                (await Digest.json(row.stack)) !== row.submitted
                    ? [row.id]
                    : [],
            ),
        );

        return listed.flat();
    }

    /** Give birth to a placed space from its replicated row, and install the stack it carries. */
    async reconcile(key: string): Promise<undefined> {
        // reconcile each space placed here of an account whose scope arrived
        if (account.identifies(key)) {
            const placed = await this.#database
                .select({ id: placement.table.id })
                .from(placement.table)
                .where(
                    and(
                        eq(placement.table.scope, account.identifier(key)),
                        eq(placement.table.machineId, this.#machine),
                    ),
                );
            for (const { id } of placed) {
                await this.reconcile(id);
            }

            return undefined;
        }

        // leave a space not placed here, and one whose row or parent scope has not arrived
        const spaceId = space.identifier(key);
        const [placed] = await this.#database
            .select({ born: Scope.table.scope, parent: space.table.scope })
            .from(placement.table)
            .innerJoin(space.table, eq(space.table.id, placement.table.id))
            .leftJoin(Scope.table, eq(Scope.table.scope, space.table.id))
            .where(
                and(eq(placement.table.id, spaceId), eq(placement.table.machineId, this.#machine)),
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

        // submit the stack the space carries once it changed since its last submission
        const [received] = await this.#database
            .select({ stack: space.table.stack, installed: installation.table })
            .from(space.table)
            .leftJoin(
                installation.table,
                and(
                    eq(installation.table.scope, space.table.id),
                    eq(installation.table.role, "stack"),
                ),
            )
            .where(eq(space.table.id, spaceId));
        const { stack, installed } = present(received, `the row of ${spaceId}`);
        const digest = await Digest.json(stack);
        if (digest !== installed?.spaceStack) {
            await this.#install(spaceId, stack, digest, installed);
        }

        return undefined;
    }

    /** Bear a placed space at its home below its ancestors, the space owning itself. */
    async #open(spaceId: Identifier<"space">, ancestors: readonly string[]): Promise<void> {
        // read the account the space lives in
        const [row] = await this.#database
            .select({ scope: space.table.scope })
            .from(space.table)
            .where(eq(space.table.id, spaceId));
        const replicated = present(row, `the row of ${spaceId}`);
        const now = this.#store.clock.now();
        await Transaction.run(this.#database, async (transaction) => {
            // record its scope and write its first access here, its home
            const authorization = new Authorization(this.#store.authorizer, transaction, () =>
                AccessContext.of(Caller.of(this.#store.principal), now),
            );
            await authorization.receive(
                space.reference(replicated.scope, spaceId),
                ancestors,
                Space.creation({ id: spaceId }),
            );
        });
    }

    /** Create a space's main database where its stack places it. */
    async #createMain(spaceId: Identifier<"space">): Promise<void> {
        // describe the main database every space has in its own storage
        const main = {
            declaration: this.#options.main.toJSON(),
            placement: { provider: SPACE_PROVIDER },
            retention: "forever" as const,
            tags: {},
        };
        const now = this.#store.clock.now();
        await Transaction.run(this.#database, async (transaction) => {
            // create the main database as the space
            await this.#store.mutate(
                this.#store.self,
                {
                    id: RequestId.create(now),
                    calls: [
                        database.call("create", {
                            ...database.in(spaceId),
                            ...Provisioned.values(MAIN_DATABASE, main),
                            spec: this.#options.main.spec,
                        }),
                    ],
                },
                { transaction, scope: spaceId },
            );
        });
    }

    /** Copy a stack's pinned build into the build store and read the space its export declares. */
    async #declared(stack: StackSource): Promise<JsonObject> {
        // copy the stack's build and its dependencies' builds from the registry and evaluate the export
        const { builds, registry } = this.#options;
        const open = (packageId: PackageId, manifest: string) =>
            BuildStore.open(builds, registry, packageId, manifest);
        const reader = await open(stack.package.id, stack.manifest);

        return declaredStack(reader, stack.export, stack.settings, open);
    }

    /** Install a space's stack at its pinned build, and submit the space its export declares. */
    async #install(
        spaceId: Identifier<"space">,
        stack: StackSource,
        digest: Digest,
        installed: Installation | null,
    ): Promise<void> {
        // read the space the stack declares at its pinned build
        const { package: stacked, manifest } = stack;
        const build = { kind: "release" as const, version: stacked.version, manifest };
        const definition = await this.#declared(stack);

        // install the stack package as the space's stack once
        const now = this.#store.clock.now();
        const selection = { kind: "release" as const, version: stacked.version };
        const [created] =
            installed === null
                ? await this.#store.mutate(this.#store.self, {
                      id: RequestId.create(now),
                      calls: [
                          installation.call("create", {
                              ...installation.in(spaceId),
                              packageId: stacked.id,
                              role: "stack",
                              selection,
                              export: stack.export,
                              parameters: {},
                              name: STACK_INSTALLATION,
                          }),
                      ],
                  })
                : [installed];

        // submit its pinned build in a mutation of its own, whose preparation reads the created stack
        await this.#store.mutate(this.#store.self, {
            id: RequestId.create(now),
            calls: [
                installation.call("submit", {
                    ...installation.target(present(created, `the stack of ${spaceId}`)),
                    selection,
                    build,
                    evaluation: { export: stack.export, parameters: {}, definition },
                    spaceStack: digest,
                }),
            ],
        });
    }
}
