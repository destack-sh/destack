import { account, endpoint, key, machine } from "@destack/account/object";
import { Change, type ReconcileController, type Table } from "@destack/db";
import { schema } from "@destack/schema";
import type { Relay } from "./relay.ts";

/** Tell each machine tunnelled here its name, or end its tunnel, once its replica changes. */
export class NameController implements ReconcileController {
    /** The controller's name in reports. */
    readonly name = "name";
    /** The replicated machines, their endpoints and keys, and the accounts names derive from. */
    readonly watches: readonly Table[] = [machine.table, endpoint.table, key.table, account.table];
    /** The relay routing to the tunnels. */
    readonly #relay: Relay;

    /** Tell the machines tunnelled to a relay their names. */
    constructor(relay: Relay) {
        this.#relay = relay;
    }

    /** List a machine tunnelled here whose endpoint, name, keys or standing changed. */
    async keys(change: Change): Promise<readonly string[]> {
        // select a machine whose endpoint came here or left, not one renewing its tunnel here
        if (Change.of(change, endpoint.table)) {
            const [before, after] = [Change.before(change), Change.after(change)];
            const wasHere = before !== null && this.#relay.isHere(before);
            const isHere = after !== null && this.#relay.isHere(after);

            return wasHere !== isHere ? [Change.image(change).id] : [];
        }
        // select a renamed machine tunnelled here, its revoke selected by its endpoint's deletion
        else if (Change.of(change, machine.table)) {
            const isRenamed = Change.before(change)?.name !== Change.after(change)?.name;
            const { id } = Change.image(change);

            return isRenamed && (await this.#relay.isTunnelled(id)) ? [id] : [];
        }
        // select the machine tunnelled here of a key that was revoked, suspended, restored or removed
        else if (Change.of(change, key.table)) {
            const before = Change.before(change);
            const after = Change.after(change);
            const image = Change.image(change);
            const isStandingChanged =
                before?.revokedAt !== after?.revokedAt ||
                before?.suspendedAt !== after?.suspendedAt;
            const isMachine = image.parentType === machine.name && image.parentId !== null;
            if (!isMachine || !isStandingChanged) {
                return [];
            }
            const machineId = schema.identifier("machine").parse(image.parentId);

            return (await this.#relay.isTunnelled(machineId)) ? [machineId] : [];
        }
        // select the machines of an account whose handle changed
        else if (Change.of(change, account.table)) {
            const isRenamed = Change.before(change)?.handle !== Change.after(change)?.handle;

            const machines = isRenamed ? await this.#relay.machines(Change.image(change).id) : [];
            const tunnelled = await Promise.all(machines.map((id) => this.#relay.isTunnelled(id)));

            return machines.filter((_, index) => tunnelled[index] === true);
        }
        // select nothing for other tables
        else {
            return [];
        }
    }

    /** List the machines tunnelled here, whose names or standing may have changed meanwhile. */
    list(): Promise<readonly string[]> {
        return this.#relay.tunnelled();
    }

    /** Tell a tunnelled machine its name, or end the tunnel of a revoked or keyless one. */
    async reconcile(machineId: string): Promise<undefined> {
        await this.#relay.reconcile(schema.identifier("machine").parse(machineId));

        return undefined;
    }
}
