import { defineExample } from "@destack/package/declare";
import { Icon } from "@destack/icon";
import { Button } from "../button/index.ts";
import { VisuallyHidden } from "./visually-hidden.tsx";

/** An icon button named by text only assistive technology reads. */
export const visuallyHiddenIconButton = defineExample({
    of: VisuallyHidden,
    name: "icon-button",
    description: "an icon button named by text only assistive technology reads",
    render: () => (
        <Button variant="outline" size="icon">
            <Icon name="trash" />
            <VisuallyHidden>Delete note</VisuallyHidden>
        </Button>
    ),
});
