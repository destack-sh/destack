import { defineExample } from "@destack/package/declare";
import { Icon } from "@destack/ui/icon";
import { Badge } from "../badge/index.ts";
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

/** An unread count whose unit only assistive technology reads. */
export const visuallyHiddenUnreadCount = defineExample({
    of: VisuallyHidden,
    name: "unread-count",
    description: "an unread count whose unit only assistive technology reads",
    render: () => (
        <p>
            Inbox{" "}
            <Badge>
                4<VisuallyHidden> unread messages</VisuallyHidden>
            </Badge>
        </p>
    ),
});
