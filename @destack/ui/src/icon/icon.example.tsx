import { t } from "@destack/locale";
import { defineExample } from "@destack/package/declare";
import { Icon } from "@destack/ui/icon";
import { Cluster } from "../cluster/index.ts";

/** A row of actions, each drawn as a labelled icon. */
export const iconActions = defineExample({
    of: Icon,
    name: "actions",
    description: t`Three actions as icons in the current text color, one bold.`,
    render: () => (
        <Cluster>
            <Icon name="pencil-simple" label="Edit" size="1.5em" />
            <Icon name="copy-simple" label="Copy" size="1.5em" />
            <Icon name="trash" weight="bold" label="Delete" size="1.5em" />
        </Cluster>
    ),
});
