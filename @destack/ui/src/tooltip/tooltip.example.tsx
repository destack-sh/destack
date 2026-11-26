import { t } from "@destack/locale";
import { Button } from "../button/index.ts";
import { defineExample } from "@destack/package/declare";
import { Icon } from "@destack/ui/icon";
import { Tooltip, TooltipContent, TooltipTrigger } from "./tooltip.tsx";

/** What an icon-only archive button does. */
export const tooltipArchiveButton = defineExample({
    of: Tooltip,
    name: "archiveButton",
    description: t`What an icon-only archive button does.`,
    render: () => (
        <Tooltip>
            <TooltipTrigger
                render={(attributes) => <Button isIconOnly {...attributes} variant="ghost" />}
                aria-label="Archive"
            >
                <Icon name="archive" />
            </TooltipTrigger>
            <TooltipContent>Archive note</TooltipContent>
        </Tooltip>
    ),
});

/** The archive button's tooltip shown above it. */
export const tooltipArchiveButtonOpen = defineExample({
    of: Tooltip,
    name: "archiveButtonOpen",
    description: t`The archive button's tooltip shown above it.`,
    properties: {
        defaultOpen: true,
    },
    render: (properties) => (
        <Tooltip {...properties}>
            <TooltipTrigger
                render={(attributes) => <Button isIconOnly {...attributes} variant="ghost" />}
                aria-label="Archive"
            >
                <Icon name="archive" />
            </TooltipTrigger>
            <TooltipContent>Archive note</TooltipContent>
        </Tooltip>
    ),
});
