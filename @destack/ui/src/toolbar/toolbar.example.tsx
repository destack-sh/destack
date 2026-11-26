import { t } from "@destack/locale";
import boldIcon from "@destack/icon/phosphor/text-b";
import italicIcon from "@destack/icon/phosphor/text-italic";
import undoIcon from "@destack/icon/phosphor/arrow-counter-clockwise";
import redoIcon from "@destack/icon/phosphor/arrow-clockwise";
import { defineExample } from "@destack/package/declare";
import { Icon } from "../icon/index.ts";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "../select/index.ts";
import { Input } from "../input/index.ts";
import { ToggleGroup, ToggleGroupItem } from "../toggle-group/index.ts";
import { Toolbar, ToolbarButton, ToolbarGroup, ToolbarSeparator } from "./toolbar.tsx";

/** The controls of a text. */
export const toolbarText = defineExample({
    of: Toolbar,
    name: "text",
    description: t`The controls of a text.`,
    render: (properties) => (
        <Toolbar aria-label="Text" {...properties}>
            <ToolbarGroup aria-label="History">
                <ToolbarButton aria-label="Undo">
                    <Icon icon={undoIcon} />
                </ToolbarButton>
                <ToolbarButton aria-label="Redo" disabled>
                    <Icon icon={redoIcon} />
                </ToolbarButton>
            </ToolbarGroup>
            <ToolbarSeparator />
            <ToggleGroup multiple defaultValue={["bold"]} aria-label="Style">
                <ToggleGroupItem value="bold" aria-label="Bold">
                    <Icon icon={boldIcon} />
                </ToggleGroupItem>
                <ToggleGroupItem value="italic" aria-label="Italic">
                    <Icon icon={italicIcon} />
                </ToggleGroupItem>
            </ToggleGroup>
            <ToolbarSeparator />
            <Select defaultValue="body" aria-label="Size">
                <SelectTrigger>
                    <SelectValue />
                </SelectTrigger>
                <SelectContent>
                    <SelectItem value="title">Title</SelectItem>
                    <SelectItem value="body">Body</SelectItem>
                </SelectContent>
            </Select>
            <Input aria-label="Find" placeholder="Find" />
        </Toolbar>
    ),
});
