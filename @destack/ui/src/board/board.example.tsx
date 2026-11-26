import { t } from "@destack/locale";
import { ActionMenu } from "../action/index.ts";
import * as style from "@destack/style";
import { defineExample } from "@destack/package/declare";
import { createSignal, For, omit } from "@destack/view";
import { present } from "@destack/schema";
import { SortableMove } from "../sortable/index.ts";
import {
    Board,
    BoardColumn,
    BoardComposer,
    BoardColumnMenu,
    BoardColumnAdd,
    BoardHiddenList,
    BoardColumnContent,
    BoardColumnCount,
    BoardColumnHandle,
    BoardColumnHeader,
    BoardColumnList,
    BoardItem,
    BoardLane,
    BoardLaneCell,
    BoardLaneRow,
    BoardLaneCollapse,
    BoardLaneHeader,
    type BoardCell,
    BoardMove,
    type BoardProperties,
} from "./board.tsx";

/** The height a long column's card takes, with its gap, in pixels. */
const CARD_HEIGHT = 44;

/** The styles of a long column. */
const styles = style.create({
    long: {
        height: "24rem",
    },
});

/** The backlog of a long board: five hundred cards. */
const BACKLOG = Array.from({ length: 500 }, (_, index) => `Ticket ${String(index + 1)}`);

/** A launch board's columns, their limits and their cards. */
const COLUMNS = [
    {
        id: "todo",
        label: "To do",
        limit: undefined,
        cards: ["Write the launch post", "Record the demo", "Draft the waitlist email"],
    },
    { id: "doing", label: "Doing", limit: 2, cards: ["Price the plans"] },
    { id: "done", label: "Done", limit: undefined, cards: ["Pick the launch date"] },
];

/** The people a team board runs a lane for. */
const PEOPLE = [
    { id: "ada", label: "Ada" },
    { id: "grace", label: "Grace" },
];

/** A team board's cards by person and column. */
const TEAM_CARDS: Readonly<Record<string, readonly string[]>> = {
    [JSON.stringify(["ada", "todo"])]: ["Write the launch post", "Record the demo"],
    [JSON.stringify(["ada", "doing"])]: ["Price the plans"],
    [JSON.stringify(["ada", "done"])]: [],
    [JSON.stringify(["grace", "todo"])]: ["Draft the waitlist email"],
    [JSON.stringify(["grace", "doing"])]: ["Fix the signup bug", "Review the pricing page"],
    [JSON.stringify(["grace", "done"])]: ["Pick the launch date"],
};

/** Read the key a board's owner files a cell's cards under: the person's and the column's. */
function cellKey(cell: BoardCell): string {
    return cell.lane === undefined ? cell.column : JSON.stringify([cell.lane, cell.column]);
}

/** Move a card between the owner's lists of cards, keyed by cell. */
function moved(
    lists: Readonly<Record<string, readonly string[]>>,
    move: BoardMove,
): Record<string, readonly string[]> {
    return BoardMove.apply(lists, move, cellKey, (card) => card);
}

/** Render the launch board, its cards and columns moving where people leave them. */
function LaunchBoard(properties: Pick<BoardProperties, "label"> & { readonly isFixed: boolean }) {
    // follow the cards of each column and the order of the columns
    const [cards, setCards] = createSignal<Record<string, readonly string[]>>(
        Object.fromEntries(COLUMNS.map((column) => [column.id, column.cards])),
    );
    const [order, setOrder] = createSignal<readonly string[]>(COLUMNS.map((column) => column.id));
    const columns = () =>
        order().map((id) =>
            present(
                COLUMNS.find((column) => column.id === id),
                "column",
            ),
        );

    return (
        <Board
            {...omit(properties, "isFixed")}
            onItemMove={(move) => setCards((current) => moved(current, move))}
            onColumnMove={(move) =>
                setOrder((current) => SortableMove.place(current, move.id, move, (id) => id))
            }
            onItemRemove={(keys) =>
                setCards((current) =>
                    Object.fromEntries(
                        Object.entries(current).map(([id, list]) => [
                            id,
                            list.filter((card) => !keys.includes(card)),
                        ]),
                    ),
                )
            }
            onCreate={(draft) =>
                setCards((current) => ({
                    ...current,
                    [draft.column]: [...(current[draft.column] ?? []), draft.title],
                }))
            }
        >
            <BoardColumnList>
                <For each={columns()}>
                    {(column) => (
                        <BoardColumn
                            id={column.id}
                            label={column.label}
                            {...(column.limit === undefined ? {} : { limit: column.limit })}
                        >
                            <BoardColumnHeader>
                                <BoardColumnHandle />
                                {column.label}
                                <BoardColumnCount />
                                <BoardColumnMenu />
                                <BoardColumnAdd />
                            </BoardColumnHeader>
                            <BoardComposer position="start" />
                            <BoardColumnContent>
                                <For each={cards()[column.id]}>
                                    {(card) => (
                                        <BoardItem id={card} disabled={properties.isFixed}>
                                            {card}
                                        </BoardItem>
                                    )}
                                </For>
                            </BoardColumnContent>
                            <BoardComposer />
                        </BoardColumn>
                    )}
                </For>
                <BoardHiddenList />
            </BoardColumnList>
            <ActionMenu />
        </Board>
    );
}

/** Render a team's board: a lane per person, the columns' headers above them. */
function TeamBoard() {
    // follow each person's cards in each column
    const [cards, setCards] = createSignal<Record<string, readonly string[]>>({ ...TEAM_CARDS });

    return (
        <Board
            label="Team"
            onItemMove={(move) => setCards((current) => moved(current, move))}
            onCreate={(draft) =>
                setCards((current) => {
                    const key = JSON.stringify([draft.lane, draft.column]);

                    return { ...current, [key]: [...(current[key] ?? []), draft.title] };
                })
            }
        >
            {/* The columns' headers */}
            <BoardColumnList>
                <For each={COLUMNS}>
                    {(column) => (
                        <BoardColumn
                            id={column.id}
                            label={column.label}
                            {...(column.limit === undefined ? {} : { limit: column.limit })}
                        >
                            <BoardColumnHeader>
                                {column.label}
                                <BoardColumnCount />
                                <BoardColumnMenu />
                                <BoardColumnAdd />
                            </BoardColumnHeader>
                        </BoardColumn>
                    )}
                </For>
                <BoardHiddenList />
            </BoardColumnList>

            {/* A lane per person */}
            <For each={PEOPLE}>
                {(person) => (
                    <BoardLane id={person.id} label={person.label}>
                        <BoardLaneHeader>
                            <BoardLaneCollapse />
                            {person.label}
                        </BoardLaneHeader>
                        <BoardLaneRow>
                            <For each={COLUMNS}>
                                {(column) => (
                                    <BoardLaneCell column={column.id}>
                                        <BoardColumnContent>
                                            <For
                                                each={
                                                    cards()[JSON.stringify([person.id, column.id])]
                                                }
                                            >
                                                {(card) => <BoardItem id={card}>{card}</BoardItem>}
                                            </For>
                                        </BoardColumnContent>
                                        <BoardComposer />
                                    </BoardLaneCell>
                                )}
                            </For>
                        </BoardLaneRow>
                    </BoardLane>
                )}
            </For>
            <ActionMenu />
        </Board>
    );
}

/** A launch board whose cards and columns move by pointer, touch and keys. */
export const boardLaunch = defineExample({
    of: Board,
    name: "launch",
    description: t`A launch plan in three columns, cards and columns moving by pointer, touch and keys.`,
    properties: {
        label: "Launch",
    },
    render: (properties) => <LaunchBoard {...properties} isFixed={false} />,
});

/** A launch plan whose cards stay where they are. */
export const boardLaunchFixedCards = defineExample({
    of: Board,
    name: "launchFixedCards",
    description: t`A launch plan whose cards stay where they are.`,
    render: () => <LaunchBoard label="Launch" isFixed />,
});

/** A team's board, a lane per person under the columns. */
export const boardTeam = defineExample({
    of: Board,
    name: "team",
    description: t`A team's board, a lane per person under the shared columns.`,
    render: () => <TeamBoard />,
});

/** Render a board whose backlog of five hundred cards mounts only the cards in view. */
function LongBoard() {
    // follow the backlog and the column beside it
    const [cards, setCards] = createSignal<Record<string, readonly string[]>>({
        backlog: BACKLOG,
        next: [],
    });

    return (
        <Board
            label="Backlog"
            onItemMove={(move) =>
                setCards((current) =>
                    BoardMove.apply(
                        current,
                        move,
                        (cell) => cell.column,
                        (card) => card,
                    ),
                )
            }
        >
            <BoardColumnList>
                <BoardColumn id="backlog" label="Backlog">
                    <BoardColumnHeader>
                        Backlog
                        <BoardColumnCount />
                    </BoardColumnHeader>
                    <BoardColumnContent
                        each={cards()["backlog"] ?? []}
                        estimateSize={() => CARD_HEIGHT}
                        itemKey={(card) => card}
                        textOf={(card) => card}
                        xstyle={styles.long}
                    >
                        {(card) => <BoardItem id={card}>{card}</BoardItem>}
                    </BoardColumnContent>
                </BoardColumn>
                <BoardColumn id="next" label="Next">
                    <BoardColumnHeader>
                        Next
                        <BoardColumnCount />
                    </BoardColumnHeader>
                    <BoardColumnContent>
                        <For each={cards()["next"]}>
                            {(card) => <BoardItem id={card}>{card}</BoardItem>}
                        </For>
                    </BoardColumnContent>
                </BoardColumn>
            </BoardColumnList>
        </Board>
    );
}

/** A backlog of five hundred cards that mounts only the cards in view. */
export const boardLongBacklog = defineExample({
    of: Board,
    name: "longBacklog",
    description: t`A backlog of five hundred cards that mounts only the cards in view.`,
    render: () => <LongBoard />,
});
