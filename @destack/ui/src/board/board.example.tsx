import { defineExample } from "@destack/package/declare";
import { createSignal, For, omit } from "@destack/view";
import { present } from "@destack/schema";
import type { SortableMove } from "../sortable/index.ts";
import {
    Board,
    BoardColumn,
    BoardColumnContent,
    BoardColumnHandle,
    BoardColumnHeader,
    BoardItem,
    type BoardProperties,
} from "./board.tsx";

/** A launch board's columns and their cards. */
const COLUMNS = [
    {
        id: "todo",
        label: "To do",
        cards: ["Write the launch post", "Record the demo", "Draft the waitlist email"],
    },
    { id: "doing", label: "Doing", cards: ["Price the plans"] },
    { id: "done", label: "Done", cards: ["Pick the launch date"] },
];

/** Place an item before its next neighbour, at the end without one, as a board's move lands it. */
function placed<Item>(
    items: readonly Item[],
    item: Item,
    next: string | undefined,
    idOf: (item: Item) => string,
): Item[] {
    const at = next === undefined ? items.length : items.findIndex((each) => idOf(each) === next);

    return items.toSpliced(at, 0, item);
}

/** Move an item between lists, before its next neighbour in the list it lands in. */
function moved<Item>(
    lists: Readonly<Record<string, readonly Item[]>>,
    move: SortableMove,
    idOf: (item: Item) => string,
): Record<string, readonly Item[]> {
    // take the item out of its list
    const item = present(
        Object.values(lists)
            .flat()
            .find((each) => idOf(each) === move.id),
        `the moved item ${move.id}`,
    );
    const rest = Object.fromEntries(
        Object.entries(lists).map(([id, items]) => [id, items.filter((each) => each !== item)]),
    );

    // place it in the list it lands in
    const target = present(rest[move.container], `the list ${move.container}`);

    return { ...rest, [move.container]: placed(target, item, move.next, idOf) };
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
            onItemMove={(move) => setCards((current) => moved(current, move, (card) => card))}
            onColumnMove={(move) =>
                setOrder((current) =>
                    placed(
                        current.filter((id) => id !== move.id),
                        move.id,
                        move.next,
                        (id) => id,
                    ),
                )
            }
        >
            <For each={columns()}>
                {(column) => (
                    <BoardColumn id={column.id} label={column.label}>
                        <BoardColumnHeader>
                            <BoardColumnHandle />
                            {column.label} · {cards()[column.id]?.length}
                        </BoardColumnHeader>
                        <BoardColumnContent>
                            <For each={cards()[column.id]}>
                                {(card) => (
                                    <BoardItem id={card} disabled={properties.isFixed}>
                                        {card}
                                    </BoardItem>
                                )}
                            </For>
                        </BoardColumnContent>
                    </BoardColumn>
                )}
            </For>
        </Board>
    );
}

/** A launch board whose cards and columns move by pointer, touch and keys. */
export const boardLaunch = defineExample({
    of: Board,
    name: "launch",
    description:
        "a launch plan in three columns, cards and columns moving by pointer, touch and keys",
    properties: {
        label: "Launch",
    },
    render: (properties) => <LaunchBoard {...properties} isFixed={false} />,
});

/** A launch plan whose cards stay where they are. */
export const boardLaunchFixedCards = defineExample({
    of: Board,
    name: "launch-fixed-cards",
    description: "a launch plan whose cards stay where they are",
    render: () => <LaunchBoard label="Launch" isFixed />,
});
