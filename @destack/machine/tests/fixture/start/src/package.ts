import { client, space, user } from "@destack/account/object";
import { definePackage, installation } from "@destack/object";
import { declaration, installationRevision, rollout } from "@destack/stack/object";
import { setting } from "@destack/setting/object";

/** The start page every person's home installs, granted what a start page reads. */
export default definePackage({
    access: {
        permissions: {
            user: {
                // read and change the person's settings, and register and bind the browser as their client
                home: [
                    setting.permission("read"),
                    setting.permission("write"),
                    client.permission("create"),
                    client.permission("rotate"),
                    client.permission("verify"),
                ],
                // read the person's profile
                universe: [user.permission("read")],
                // list the account's spaces
                account: [space.permission("list")],
                // list and read the space's installations and rollouts, with their declarations
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
    },
});
