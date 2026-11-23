import type { IconBodies } from "@destack/icon";
import archive from "@destack/icon/phosphor/archive";
import checkSquare from "@destack/icon/phosphor/check-square";
import gear from "@destack/icon/phosphor/gear";
import kanban from "@destack/icon/phosphor/kanban";
import link from "@destack/icon/phosphor/link";
import userPlus from "@destack/icon/phosphor/user-plus";
import { defineExample } from "@destack/package/declare";
import * as style from "@destack/style";
import { color, space, weight } from "@destack/theme/tokens.stylex";
import { createSignal } from "@destack/view";
import { Button } from "../button/index.ts";
import { CommandDetailField, CommandDetailMetadata } from "../command/index.ts";
import type { PaletteItem, PalettePage } from "./control.ts";
import { CommandPalette } from "./palette.tsx";

/** The styles of a task's detail beside the palette's entries. */
const styles = style.create({
    title: {
        margin: 0,
        fontWeight: weight.medium,
    },
    line: {
        marginBlock: space[1],
        color: color.mutedForeground,
    },
});

/** The launch's open tasks, by key. */
const TASKS = [
    {
        key: "LAU-14",
        title: "Record the demo",
        summary: "A two-minute walkthrough of the board, the calendar and the palette.",
        status: "Doing",
        assignee: "Ada Lovelace",
        due: "Thursday",
    },
    {
        key: "LAU-15",
        title: "Pricing page",
        summary: "Plans, the free tier and the questions people ask before paying.",
        status: "Doing",
        assignee: "Jonas Weber",
        due: "Friday",
    },
    {
        key: "LAU-21",
        title: "Write the changelog",
        summary: "Everything that changed since the beta, in the order people notice it.",
        status: "Backlog",
        assignee: "Mia Sørensen",
        due: "Next Monday",
    },
] as const;

/** The people a task can be assigned to. */
const PEOPLE = ["Ada Lovelace", "Jonas Weber", "Mia Sørensen"] as const;

/** The views of the workspace. */
const VIEWS: readonly { readonly id: string; readonly title: string; readonly icon: IconBodies }[] =
    [
        { id: "board", title: "Launch board", icon: kanban },
        { id: "my-tasks", title: "My tasks", icon: checkSquare },
        { id: "settings", title: "Settings", icon: gear },
    ];

/** Build the palette's root page, reporting what each action does. */
function rootOf(report: (summary: string) => void): PalettePage {
    // each task opens, copies its link or archives, and shows its detail
    const tasks = TASKS.map((task): PaletteItem => ({
        id: task.key,
        title: task.title,
        subtitle: task.key,
        icon: checkSquare,
        accessories: [
            { text: task.status, color: task.status === "Doing" ? "amber" : "blue" },
            { text: task.assignee },
        ],
        keywords: [task.key, task.assignee],
        actions: [
            { id: "open", title: "Open task", run: () => report(`Opened ${task.title}`) },
            {
                id: "copy",
                title: "Copy link",
                icon: link,
                shortcut: "mod+shift+c",
                run: () => report(`Copied ${task.key}`),
            },
            {
                id: "archive",
                title: "Archive",
                icon: archive,
                shortcut: "mod+backspace",
                isDestructive: true,
                run: () => report(`Archived ${task.title}`),
            },
        ],
        detail: () => (
            <>
                <h3 {...style.attrs(styles.title)}>{task.title}</h3>
                <p {...style.attrs(styles.line)}>{task.summary}</p>
                <CommandDetailMetadata>
                    <CommandDetailField label="Key">{task.key}</CommandDetailField>
                    <CommandDetailField label="Status">{task.status}</CommandDetailField>
                    <CommandDetailField label="Assignee">{task.assignee}</CommandDetailField>
                    <CommandDetailField label="Due">{task.due}</CommandDetailField>
                </CommandDetailMetadata>
            </>
        ),
    }));

    // assigning opens a page of people
    const assign: PaletteItem = {
        id: "assign",
        title: "Assign to…",
        icon: userPlus,
        actions: [],
        page: () => ({
            id: "assign",
            title: "Assign to",
            placeholder: "Search people",
            groups: [
                {
                    id: "people",
                    title: "People",
                    items: () =>
                        PEOPLE.map((person) => ({
                            id: `assign-${person}`,
                            title: person,
                            actions: [
                                {
                                    id: "assign",
                                    title: "Assign",
                                    run: () => report(`Assigned to ${person}`),
                                },
                            ],
                        })),
                },
            ],
        }),
    };

    return {
        id: "root",
        title: "Launch",
        groups: [
            { id: "tasks", title: "Tasks", items: () => tasks },
            { id: "commands", title: "Commands", items: () => [assign] },
            {
                id: "views",
                title: "Views",
                items: () =>
                    VIEWS.map((view) => ({
                        id: view.id,
                        title: view.title,
                        icon: view.icon,
                        actions: [
                            {
                                id: "open",
                                title: "Open view",
                                run: () => report(`Opened ${view.title}`),
                            },
                        ],
                    })),
            },
        ],
    };
}

/** A workspace's palette: tasks with their actions and detail, a page to assign from, and views. */
export const commandPaletteLaunch = defineExample({
    of: CommandPalette,
    name: "launch",
    description: "a workspace's palette: tasks with their actions and detail, assigning, and views",
    render: () => {
        // open the palette from a button, and show what its last action did
        const [isOpen, setOpen] = createSignal(false);
        const [summary, setSummary] = createSignal("");
        const root = rootOf(setSummary);

        return (
            <>
                <Button variant="outline" onClick={() => setOpen(true)}>
                    Open palette
                </Button>
                <p>{summary()}</p>
                <CommandPalette id="launch" root={root} open={isOpen()} onOpenChange={setOpen} />
            </>
        );
    },
});
