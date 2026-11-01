import { AccessContext, Authorization, Caller } from "@destack/access";
import * as accountObject from "@destack/account/object";
import { Change, type ReconcileController, Snapshot, type Table } from "@destack/db";
import { Installation } from "@destack/object";
import objectPackage from "@destack/object/package";
import type { ObjectStore } from "@destack/object/server";
import type { Identifier } from "@destack/schema";
import { Scope } from "@destack/sync";
import machinePackage from "../../package.ts";

/** The packages a machine's scope is born with, each as its base installation. */
const MACHINE_PACKAGES = [machinePackage, objectPackage];

/** Receive this machine's scope at its primary, the machine's own store, once its row arrives. */
export class ScopeController implements ReconcileController {
    /** The controller's name in reports. */
    readonly name = "machine-scope";
    /** The tables whose changes bring the machine's row and its parent's scope. */
    readonly watches: readonly Table[] = [accountObject.machine.table, Scope.table];
    /** The machine's own store. */
    readonly #store: Pick<
        ObjectStore,
        "database" | "authorizer" | "environment" | "principal" | "objects" | "readServer"
    >;
    /** The machine. */
    readonly #machineId: Identifier<"machine">;

    /** Receive a machine's scope in its own store. */
    constructor(
        store: Pick<
            ObjectStore,
            "database" | "authorizer" | "environment" | "principal" | "objects" | "readServer"
        >,
        machineId: Identifier<"machine">,
    ) {
        this.#store = store;
        this.#machineId = machineId;
    }

    /** Select the machine at each change of a watched table. */
    keys(change: Change): readonly string[] {
        return this.watches.some((table) => Change.of(change, table)) ? [this.#machineId] : [];
    }

    /** List the machine. */
    async list(): Promise<readonly string[]> {
        return [this.#machineId];
    }

    /** Receive the machine's scope below its parent once its row and its parent's scope arrive. */
    async reconcile(): Promise<undefined> {
        // leave a machine whose row has not arrived, or whose scope this store serves
        const { database, authorizer, environment, principal } = this.#store;
        const view = Snapshot.live(database);
        const row = await view
            .read(accountObject.machine.table)
            .findFirst({ where: { id: this.#machineId } });
        const [received] = await Scope.read(view, [this.#machineId]);
        if (row === undefined || received !== undefined) {
            return undefined;
        }

        // receive it once its parent's scope arrived, below the parent's ancestors
        const [parent] = await Scope.read(view, [row.scope]);
        if (parent === undefined) {
            return undefined;
        }
        const now = environment.clock.now();
        // receive the scope as the machine, with its packages' base installations
        const scope = accountObject.machine.reference(row.scope, row.id);
        const ancestors = [row.scope, ...parent.ancestors];
        await Authorization.run(
            authorizer,
            database,
            () => AccessContext.of(Caller.of(principal), now),
            environment,
            (authorization) =>
                Installation.receive(authorization, scope, ancestors, MACHINE_PACKAGES),
        );

        return undefined;
    }
}
