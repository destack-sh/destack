import { t } from "@destack/locale";
import { defineExample } from "@destack/package/declare";
import { Icon } from "@destack/ui/icon";
import { Badge } from "../badge/index.ts";
import { Button } from "../button/index.ts";
import { VisuallyHidden } from "./visually-hidden.tsx";

/** An icon button named by text only assistive technology reads. */
export const visuallyHiddenIconButton = defineExample({
    of: VisuallyHidden,
    name: "iconButton",
    description: t`An icon button named by text only assistive technology reads.`,
    render: () => (
        <Button isIconOnly variant="outline">
            <Icon name="trash" />
            <VisuallyHidden>Delete note</VisuallyHidden>
        </Button>
    ),
});

/** An unread count whose unit only assistive technology reads. */
export const visuallyHiddenUnreadCount = defineExample({
    of: VisuallyHidden,
    name: "unreadCount",
    description: t`An unread count whose unit only assistive technology reads.`,
    render: () => (
        <p>
            Inbox{" "}
            <Badge>
                4<VisuallyHidden> unread messages</VisuallyHidden>
            </Badge>
        </p>
    ),
});
