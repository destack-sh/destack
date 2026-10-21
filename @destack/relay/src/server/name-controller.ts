import { account, key, machine } from "@destack/account/object";
import { Change, type Table } from "@destack/db";
import { schema } from "@destack/schema";
import type { Controller } from "@destack/db";
import type { Relay } from "./relay.ts";

/** Tell each connected machine its name once its own or its account's replica changes. */
export class NameController implements Controller {
    /** The controller's name in reports. */
    readonly name = "name";
    /** The replicated machines, their keys and the accounts names derive from. */
    readonly watches: readonly Table[] = [machine.table, key.table, account.table];
    /** The relay routing to the tunnels. */
    readonly #relay: Relay;

    /** Tell the machines connected to a relay their names. */
    constructor(relay: Relay) {
        this.#relay = relay;
    }

    /** List the machine whose name or revocation changed. */
    async keys(change: Change): Promise<readonly string[]> {
        // select a machine that was renamed or revoked or created or deleted
        if (Change.of(change, machine.table)) {
            const before = Change.before(change);
            const after = Change.after(change);
            const isChanged =
                before?.name !== after?.name || before?.revokedAt !== after?.revokedAt;

            return isChanged ? [Change.image(change).id] : [];
        }
        // select the machine of a key that was revoked, suspended, restored or removed
        else if (Change.of(change, key.table)) {
            const before = Change.before(change);
            const after = Change.after(change);
            const image = Change.image(change);
            const isStanding =
                before?.revokedAt !== after?.revokedAt ||
                before?.suspendedAt !== after?.suspendedAt;

            return image.parentType === machine.name && image.parentId !== null && isStanding
                ? [image.parentId]
                : [];
        }
        // select the machines of an account whose handle changed
        else if (Change.of(change, account.table)) {
            const isRenamed = Change.before(change)?.handle !== Change.after(change)?.handle;

            return isRenamed ? this.#relay.machines(Change.image(change).id) : [];
        }
        // select nothing for other tables
        else {
            return [];
        }
    }

    /** List an empty start, as each tunnel takes its name as it opens. */
    async list(): Promise<readonly string[]> {
        return [];
    }

    /** Tell a connected machine its name, or close its revoked machine's tunnel. */
    async reconcile(machineId: string): Promise<undefined> {
        await this.#relay.reconcile(schema.identifier("machine").parse(machineId));

        return undefined;
    }
}
