import { creating } from "@destack/object";
import * as style from "@destack/style";
import { defineExample } from "@destack/package/declare";
import {
    createSignal,
    handleOnPage,
    type JSX,
    redo,
    Show,
    undo,
    useActions,
    useEnvironment,
} from "@destack/view";
import { ActionDialog, ActionMenu, ShortcutDialog, ShortcutHint } from "../../action/index.ts";
import { Toaster } from "../../toast/index.ts";
import { createTask, goToActive, goToAll, goToBacklog, handleTasks } from "./actions.example.ts";
import { taskType } from "./object.example.ts";
import { sampleTasks } from "./sample.ts";
import { type PlaceGroup, Shell } from "./shell.tsx";
import { Search } from "./search.tsx";
import { Studio, type TaskPatch } from "./state.ts";
import { CreateTask, TaskPage } from "./task.tsx";
import { Tasks } from "./tasks.tsx";

/** The styles of the studio. */
const styles = style.create({
    app: {
        display: "contents",
    },
});

/** Render the studio: its tasks, pages and conversations, all in the browser's memory. */
function Harbour(properties: {
    /** The number of tasks. */
    readonly tasks: number;
}): JSX.Element {
    // open the studio at its active tasks
    const { clock } = useEnvironment();
    const actions = useActions();
    const studio = new Studio(sampleTasks(properties.tasks, clock.now()), () => clock.now());
    const [view, setView] = createSignal("active");
    const [opened, setOpened] = createSignal<string | undefined>(undefined);
    const [start, setStart] = createSignal<TaskPatch>({}, { ownedWrite: true });
    const [isCreating, setCreating] = createSignal(false, { ownedWrite: true });
    const task = () => {
        const id = opened();

        return id === undefined ? undefined : studio.find(id);
    };
    const go = (place: string) => {
        setOpened(undefined);
        setView(place);
    };
    handleOnPage(undo, { isEnabled: () => studio.canUndo(), run: () => studio.undo() });
    handleOnPage(redo, { isEnabled: () => studio.canRedo(), run: () => studio.redo() });
    handleOnPage(goToActive, { run: () => go("active") });
    handleOnPage(goToBacklog, { run: () => go("backlog") });
    handleOnPage(goToAll, { run: () => go("all") });
    handleOnPage(createTask, {
        run: ({ element }) => {
            // start with what the group the person is in has
            setStart(creating(taskType).read(actions.places(element)));
            setCreating(true);
        },
    });
    const places = (): readonly PlaceGroup[] => [
        {
            label: "Tasks",
            places: studio.views().map(({ id, name }) => ({ id, name, icon: "list-checks" })),
        },
    ];

    return (
        <Toaster>
            <div ref={handleTasks(studio)} {...style.attrs(styles.app)}>
                <Shell groups={places()} current={view()} onGo={go}>
                    <Show
                        when={task()}
                        fallback={
                            <Tasks
                                studio={studio}
                                view={view()}
                                onGo={go}
                                onOpen={(id) => void setOpened(id)}
                            />
                        }
                    >
                        {(current) => (
                            <TaskPage
                                studio={studio}
                                task={current()}
                                view={studio.view(view()).name}
                                onBack={() => setOpened(undefined)}
                                onOpen={(id) => void setOpened(id)}
                            />
                        )}
                    </Show>
                </Shell>
                <ActionDialog />
                <Search studio={studio} onOpen={(id) => void setOpened(id)} />
                <ActionMenu />
                <ShortcutDialog />
                <ShortcutHint />
                <CreateTask
                    studio={studio}
                    isOpen={isCreating()}
                    start={start()}
                    onOpenChange={(isOpen) => void setCreating(isOpen)}
                />
            </div>
        </Toaster>
    );
}

/** The studio's tasks: rows under their statuses, moved, set and filtered by mouse and keys. */
export const studioTasks = defineExample({
    of: Harbour,
    name: "tasks",
    description: "the studio's tasks: rows under their statuses, moved, set and filtered",
    layout: "fullscreen",
    render: () => <Harbour tasks={48} />,
});

/** The studio with two thousand tasks. */
export const studioLong = defineExample({
    of: Harbour,
    name: "long",
    description: "two thousand tasks, only those in view in the document",
    layout: "fullscreen",
    render: () => <Harbour tasks={2000} />,
});
