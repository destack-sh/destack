import kanbanIcon from "@destack/icon/phosphor/kanban";
import rowsIcon from "@destack/icon/phosphor/rows";
import slidersIcon from "@destack/icon/phosphor/sliders-horizontal";
import * as style from "@destack/style";
import { color, space } from "@destack/theme/tokens.stylex";
import { Icon } from "@destack/ui/icon";
import { createSignal, handleOnPage, type JSX } from "@destack/view";
import { Button } from "../../button/index.ts";
import { Field, FieldGroup, FieldLabel } from "../../field/index.ts";
import { Popover, PopoverContent, PopoverTrigger } from "../../popover/index.ts";
import {
    Select,
    SelectContent,
    SelectItem,
    SelectTrigger,
    SelectValue,
} from "../../select/index.ts";
import { Sort, type SortFields } from "../../sort/index.ts";
import { Switch } from "../../switch/index.ts";
import { ToggleGroup, ToggleGroupItem } from "../../toggle-group/index.ts";
import { showDisplay, toggleLayout } from "./actions.example.ts";
import type { Studio, TaskField, TaskGroup } from "./state.ts";

/** The styles of the display options. */
const styles = style.create({
    content: {
        width: "20rem",
        maxWidth: `calc(100vw - 2 * ${space[4]})`,
    },
    fields: {
        gap: space[3],
    },
    wide: {
        width: "100%",
    },
    layout: {
        flexGrow: 1,
    },
    label: {
        color: color.mutedForeground,
    },
    wrap: {
        flexWrap: "wrap",
    },
});

/** The properties tasks are grouped by. */
const GROUPS: readonly (readonly [TaskGroup, string])[] = [
    ["status", "Status"],
    ["priority", "Priority"],
    ["assignee", "Assignee"],
    ["project", "Project"],
];

/** The fields a person sorts tasks by. */
const SORTS: SortFields = {
    priority: { label: "Priority" },
    due: { label: "Due", type: { kind: "time" } },
    created: { label: "Created", type: { kind: "time" } },
    title: { label: "Title", type: { kind: "text" } },
};

/** The properties a row shows. */
const FIELDS: readonly (readonly [TaskField, string])[] = [
    ["id", "ID"],
    ["priority", "Priority"],
    ["labels", "Labels"],
    ["project", "Project"],
    ["due", "Due"],
    ["created", "Created"],
    ["assignee", "Assignee"],
];

/** Render the options of how a view looks at its tasks. */
export function Display(properties: {
    /** The studio. */
    readonly studio: Studio;
    /** The view's id. */
    readonly view: string;
}): JSX.Element {
    // open by its key, and change the layout by its own
    const { studio } = properties;
    const view = () => studio.view(properties.view);
    const [isOpen, setOpen] = createSignal(false, { ownedWrite: true });
    const arrange = (patch: Parameters<Studio["arrange"]>[1]) =>
        studio.arrange(properties.view, patch);
    handleOnPage(showDisplay, { run: () => void setOpen((is) => !is) });
    handleOnPage(toggleLayout, {
        isOn: () => view().layout === "board",
        run: () => arrange({ layout: view().layout === "board" ? "list" : "board" }),
    });

    return (
        <Popover open={isOpen()} onOpenChange={(next) => void setOpen(next)}>
            <PopoverTrigger
                render={(attributes) => <Button {...attributes} variant="ghost" size="sm" />}
            >
                <Icon icon={slidersIcon} /> Display
            </PopoverTrigger>
            <PopoverContent align="end" xstyle={styles.content}>
                <FieldGroup xstyle={styles.fields}>
                    <ToggleGroup
                        aria-label="Layout"
                        value={view().layout}
                        disallowEmptySelection
                        onValueChange={(layout) =>
                            arrange({ layout: layout === "board" ? "board" : "list" })
                        }
                        xstyle={styles.wide}
                    >
                        <ToggleGroupItem value="list" xstyle={styles.layout}>
                            <Icon icon={rowsIcon} /> List
                        </ToggleGroupItem>
                        <ToggleGroupItem value="board" xstyle={styles.layout}>
                            <Icon icon={kanbanIcon} /> Board
                        </ToggleGroupItem>
                    </ToggleGroup>
                    <Field orientation="horizontal">
                        <FieldLabel xstyle={styles.label}>Group by</FieldLabel>
                        <Select
                            size="sm"
                            value={view().group}
                            onValueChange={(value) => {
                                const found = GROUPS.find(([group]) => group === value);
                                if (found !== undefined) {
                                    arrange({ group: found[0], subgroup: null, collapsed: [] });
                                }
                            }}
                        >
                            <SelectTrigger>
                                <SelectValue />
                            </SelectTrigger>
                            <SelectContent>
                                {GROUPS.map(([value, label]) => (
                                    <SelectItem value={value}>{label}</SelectItem>
                                ))}
                            </SelectContent>
                        </Select>
                    </Field>
                    <Field orientation="horizontal">
                        <FieldLabel xstyle={styles.label}>Then by</FieldLabel>
                        <Select
                            size="sm"
                            value={view().subgroup ?? "none"}
                            onValueChange={(value) => {
                                const found = GROUPS.find(([group]) => group === value);
                                arrange({ subgroup: found?.[0] ?? null, collapsed: [] });
                            }}
                        >
                            <SelectTrigger>
                                <SelectValue />
                            </SelectTrigger>
                            <SelectContent>
                                <SelectItem value="none">Nothing</SelectItem>
                                {GROUPS.filter(([value]) => value !== view().group).map(
                                    ([value, label]) => (
                                        <SelectItem value={value}>{label}</SelectItem>
                                    ),
                                )}
                            </SelectContent>
                        </Select>
                    </Field>
                    <Field orientation="horizontal">
                        <FieldLabel xstyle={styles.label}>Order</FieldLabel>
                        <Sort
                            fields={SORTS}
                            value={view().orderBy}
                            onValueChange={(orderBy) => arrange({ orderBy })}
                        />
                    </Field>
                    <Field orientation="horizontal">
                        <FieldLabel xstyle={styles.label}>Empty groups</FieldLabel>
                        <Switch
                            checked={view().hasEmptyGroups}
                            onCheckedChange={(hasEmptyGroups) => arrange({ hasEmptyGroups })}
                        />
                    </Field>
                    <Field orientation="horizontal">
                        <FieldLabel xstyle={styles.label}>Subtasks</FieldLabel>
                        <Switch
                            checked={view().hasSubtasks}
                            onCheckedChange={(hasSubtasks) => arrange({ hasSubtasks })}
                        />
                    </Field>
                    <Field>
                        <FieldLabel xstyle={styles.label}>Properties</FieldLabel>
                        <ToggleGroup
                            multiple
                            size="sm"
                            variant="outline"
                            aria-label="Properties"
                            value={view().fields}
                            onValueChange={(values) =>
                                arrange({
                                    fields: FIELDS.map(([field]) => field).filter((field) =>
                                        values.includes(field),
                                    ),
                                })
                            }
                            xstyle={styles.wrap}
                        >
                            {FIELDS.map(([value, label]) => (
                                <ToggleGroupItem value={value}>{label}</ToggleGroupItem>
                            ))}
                        </ToggleGroup>
                    </Field>
                </FieldGroup>
            </PopoverContent>
        </Popover>
    );
}
