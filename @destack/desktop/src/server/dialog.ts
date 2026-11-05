import { type Localization, t } from "@destack/locale";
import type { GrantPermission } from "@destack/access";
import type { Address } from "@destack/view/declare";
import type { Shell } from "../shell/index.ts";
import { Link } from "../link/index.ts";

/** A question the shell asks the person for a window: may its view act for them as it asks. */
export class Dialog {
    /** The window row asking. */
    readonly id: string;
    /** The view or object the window opens. */
    readonly address: Address;

    /** Ask for a window row about the view it opens. */
    constructor(id: string, address: Address) {
        this.id = id;
        this.address = address;
    }

    /** Ask the person whether the view may use the permissions it asks for. */
    async ask(
        shell: Shell,
        localization: Localization,
        permissions: readonly GrantPermission[],
    ): Promise<void> {
        // list each permission as its verb and type under the view's title
        const name = Link.title(this.address);
        const listed = permissions.map((permission) => `${permission.name} ${permission.type}`);

        // ask in the host's language
        await shell.send({
            action: "dialog.ask",
            id: this.id,
            title: localization.render(t`Allow ${name} to act for you?`),
            body: [...new Set(listed)].join("\n"),
            allow: localization.render(t`Allow`),
            deny: localization.render(t`Don’t Allow`),
        });
    }
}
