import { Snapshot } from "@destack/db/log";
import type { Directory, Zone } from "@destack/directory";
import type { Call, ObjectType } from "@destack/object";
import { Scope } from "@destack/sync";
import { space, SpaceCell } from "../object/index.ts";

/** Serve spaces, placing each new space's zone in this cell before creating it. */
export function serveSpaces(options: { readonly cell: SpaceCell; readonly directory: Directory }) {
    const { directory, cell } = options;

    return space.handle({
        create: {
            authorize: requireCreation(space),
            prepare: async (call) => {
                // place the new space's zone in this cell at the first epoch
                const zone = zoneOf(call, cell);
                await directory.place(zone);

                return zone;
            },
            settle: async (call, _prepared, isCommitted) => {
                // withdraw the zone of a failed creation
                if (!isCommitted) {
                    await directory.withdraw(zoneOf(call, cell));
                }
            },
        },
    });
}

/** Require the permission to create an object in the call's scope before the creation's external work. */
export function requireCreation(created: ObjectType) {
    return async (call: Call) => {
        const scope = await Scope.object(Snapshot.live(call.database), call.scope);
        await call.authorization!.require(created.permission("create"), scope);
    };
}

/** Build the zone a space creation delegates to this cell at the first epoch. */
function zoneOf(call: Call, cell: SpaceCell): Zone {
    return { id: call.id!, scope: call.scope, cell: SpaceCell.id(cell), epoch: 1 };
}
