import { creating } from "@destack/object";
import * as style from "@destack/style";
import { defineExample } from "@destack/package/declare";
import {
    createMemo,
    createSignal,
    For,
    handleOnPage,
    type JSX,
    onCleanup,
    redo,
    Show,
    undo,
    untrack,
    useActions,
    useEnvironment,
} from "@destack/view";
import { ActionDialog, ActionMenu, ShortcutDialog, ShortcutHint } from "../../action/index.ts";
import { Toaster } from "../../toast/index.ts";
import { Assistant } from "./assistant.ts";
import { ConversationPage } from "./conversation.tsx";
import { newChat } from "./chat.example.ts";
import { Chat } from "./chat.ts";
import { sampleConversations } from "./chat.sample.ts";
import { createTask, goToActive, goToAll, goToBacklog, handleTasks } from "./actions.example.ts";
import { taskType } from "./object.example.ts";
import { sampleTasks } from "./sample.ts";
import { type PlaceGroup, Shell } from "./shell.tsx";
import { Search } from "./search.tsx";
import { Studio, type TaskPatch } from "./state.ts";
import { CreateTask, TaskPage } from "./task.tsx";
import { Tasks } from "./tasks.tsx";

/** The icons of the kinds of conversation. */
const ICONS = { channel: "hash", person: "user", assistant: "sparkle" } as const;

/** The styles of the studio. */
const styles = style.create({
    app: {
        display: "contents",
    },
});

/** Render the studio: its tasks, pages and chat, all in the browser's memory. */
function Harbour(properties: {
    /** The number of tasks. */
    readonly tasks: number;
    /** The place it opens at, its active tasks by default. */
    readonly place?: string;
}): JSX.Element {
    // open the studio at its active tasks
    const { clock } = useEnvironment();
    const actions = useActions();
    const studio = new Studio(sampleTasks(properties.tasks, clock.now()), () => clock.now());
    const chat = new Chat(sampleConversations(properties.tasks * 5, clock.now()), () =>
        clock.now(),
    );
    const assistant = new Assistant(chat, clock);
    onCleanup(() => assistant.close());
    const [view, setView] = createSignal(untrack(() => properties.place ?? "active"));
    const conversation = createMemo(() => chat.find(view()));
    const [opened, setOpened] = createSignal<string | undefined>(undefined);
    const [start, setStart] = createSignal<TaskPatch>({}, { ownedWrite: true });
    const [isCreating, setCreating] = createSignal(false, { ownedWrite: true });
    const task = () => {
        const id = opened();

        return id === undefined ? undefined : studio.find(id);
    };
    const go = (place: string) => {
        // read a conversation as it is left
        const left = untrack(conversation);
        if (left !== undefined) {
            chat.read(left.id);
        }
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
    handleOnPage(newChat, { run: () => go(chat.create()) });
    const placesOf = (kind: "channel" | "person" | "assistant") =>
        chat
            .all()
            .filter((each) => each.kind === kind)
            .map(({ id, name }) => ({
                id,
                name,
                icon: ICONS[kind],
                count: chat.unread(id) || undefined,
            }));
    const places = (): readonly PlaceGroup[] => [
        {
            label: "Tasks",
            places: studio.views().map(({ id, name }) => ({ id, name, icon: "list-checks" })),
        },
        { label: "Channels", places: placesOf("channel") },
        { label: "People", places: placesOf("person") },
        { label: "Assistant", places: placesOf("assistant") },
    ];

    return (
        <Toaster>
            <div ref={handleTasks(studio)} {...style.attrs(styles.app)}>
                <Shell groups={places()} current={view()} onGo={go}>
                    <Show
                        when={task()}
                        fallback={
                            <Show
                                when={conversation()}
                                fallback={
                                    <Tasks
                                        studio={studio}
                                        view={view()}
                                        onGo={go}
                                        onOpen={(id) => void setOpened(id)}
                                    />
                                }
                            >
                                {(each) => (
                                    <For each={[each().id]}>
                                        {() => (
                                            <ConversationPage
                                                chat={chat}
                                                assistant={assistant}
                                                conversation={each()}
                                            />
                                        )}
                                    </For>
                                )}
                            </Show>
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

/** The studio at a channel: what its members said, threads, and the assistant among them. */
export const studioChannel = defineExample({
    of: Harbour,
    name: "channel",
    description: "the studio at a channel: what its members said, with threads and the unread",
    layout: "fullscreen",
    render: () => <Harbour tasks={48} place="printing" />,
});

/** The studio at a chat with the assistant, which works, asks and answers. */
export const studioAssistant = defineExample({
    of: Harbour,
    name: "assistant",
    description: "the studio at a chat with the assistant, which works, asks and answers",
    layout: "fullscreen",
    render: () => <Harbour tasks={48} place="new" />,
});

/** The studio at a channel with ten thousand messages. */
export const studioArchive = defineExample({
    of: Harbour,
    name: "archive",
    description: "a channel with ten thousand messages, the last of them drawn",
    layout: "fullscreen",
    render: () => <Harbour tasks={2000} place="archive" />,
});
