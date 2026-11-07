import { client, space, user } from "@destack/account/object";
import { definePackage, installation } from "@destack/object";
import { declaration, installationRevision, rollout } from "@destack/stack/object";
import { setting } from "@destack/setting/object";

/** The handle clients import to address this package. */
export default definePackage({
    permissions: {
        user: {
            // read and change the person's own settings, and register and bind this browser and its keys as their client
            home: [
                setting.permission("read"),
                setting.permission("write"),
                client.permission("create"),
                client.permission("rotate"),
                client.permission("verify"),
            ],
            // read the person's profile, whose language every page presents in
            universe: [user.permission("read")],
            // list the account's spaces
            account: [space.permission("list")],
            // list and read the space's installations and rollouts, with the commands and views they declare
            space: [
                installation.permission("list"),
                installation.permission("read"),
                rollout.permission("list"),
                rollout.permission("read"),
                installationRevision.permission("read"),
                declaration.permission("read"),
            ],
        },
    },
});
