import { defineExample } from "@destack/package/declare";
import { Icon } from "@destack/ui/icon";

/** A row of actions, each drawn as a labelled icon. */
export const iconActions = defineExample({
    of: Icon,
    name: "actions",
    description: "three actions as icons in the current text color, one bold",
    render: () => (
        <span>
            <Icon name="pencil-simple" label="Edit" />
            <Icon name="copy-simple" label="Copy" />
            <Icon name="trash" weight="bold" label="Delete" />
        </span>
    ),
});
