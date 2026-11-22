import { defineExample } from "@destack/package/declare";
import { createSignal, For, omit } from "@destack/view";
import { present } from "@destack/schema";
import { SortableMove } from "../sortable/index.ts";
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
            onItemMove={(move) =>
                setCards((current) => SortableMove.apply(current, move, (card) => card))
            }
            onColumnMove={(move) =>
                setOrder((current) => SortableMove.place(current, move.id, move, (id) => id))
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
