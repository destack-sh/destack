import { defineExample } from "@destack/package/declare";
import { createSignal } from "@destack/view";
import {
    ListBox,
    ListBoxEmpty,
    ListBoxItem,
    ListBoxSection,
    ListBoxSeparator,
} from "./list-box.tsx";

/** The notebook a note files into, one chosen at a time, with an archived one unavailable. */
export const listBoxNotebook = defineExample({
    of: ListBox,
    name: "notebook",
    description:
        "the notebook a note files into, one chosen at a time, with an archived one unavailable",
    render: () => {
        const [notebook, setNotebook] = createSignal<string>();

        return (
            <>
                <ListBox
                    aria-label="Notebook"
                    selection={{ defaultValue: "work", onValueChange: setNotebook }}
                >
                    <ListBoxSection heading="Active">
                        <ListBoxItem value="trips">Trips</ListBoxItem>
                        <ListBoxItem value="work">Work</ListBoxItem>
                        <ListBoxItem value="recipes">Recipes</ListBoxItem>
                    </ListBoxSection>
                    <ListBoxSeparator />
                    <ListBoxSection heading="Archived">
                        <ListBoxItem value="taxes" disabled>
                            Taxes
                        </ListBoxItem>
                    </ListBoxSection>
                </ListBox>
                <output>{notebook()}</output>
            </>
        );
    },
});

/** A note's tags, several chosen at a time. */
export const listBoxTags = defineExample({
    of: ListBox,
    name: "tags",
    description: "a note's tags, several chosen at a time",
    render: () => (
        <ListBox aria-label="Tags" selection={{ multiple: true, defaultValue: ["travel", "food"] }}>
            <ListBoxItem value="travel">travel</ListBoxItem>
            <ListBoxItem value="food">food</ListBoxItem>
            <ListBoxItem value="family">family</ListBoxItem>
        </ListBox>
    ),
});

/** Six labels in a grid of three columns, which every arrow key moves through. */
export const listBoxLabelGrid = defineExample({
    of: ListBox,
    name: "label-grid",
    description: "six labels in a grid of three columns, which every arrow key moves through",
    render: () => (
        <ListBox
            aria-label="Label"
            layout="grid"
            columns={3}
            selectionBehavior="replace"
            selection={{ defaultValue: "urgent" }}
        >
            <ListBoxItem value="urgent">Urgent</ListBoxItem>
            <ListBoxItem value="later">Later</ListBoxItem>
            <ListBoxItem value="idea">Idea</ListBoxItem>
            <ListBoxItem value="waiting">Waiting</ListBoxItem>
            <ListBoxItem value="done">Done</ListBoxItem>
            <ListBoxItem value="someday">Someday</ListBoxItem>
        </ListBox>
    ),
});

/** A list box without options, showing its empty note. */
export const listBoxEmpty = defineExample({
    of: ListBox,
    name: "empty",
    description: "a list box without options, showing its empty note",
    render: () => (
        <ListBox aria-label="Notebook">
            <ListBoxEmpty>No notebooks</ListBoxEmpty>
        </ListBox>
    ),
});
