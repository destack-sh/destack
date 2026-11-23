import { defineExample } from "@destack/package/declare";
import * as style from "@destack/style";
import { text } from "@destack/theme/text";
import { color, radius, space, swatch, width } from "@destack/theme/tokens.stylex";
import { For } from "@destack/view";
import { Button } from "../button/index.ts";
import { Progress } from "../progress/index.ts";
import {
    Card,
    CardAction,
    CardContent,
    CardDescription,
    CardFooter,
    CardHeader,
    CardTitle,
} from "./card.tsx";

/** What uses a space's storage, each in a chart series' mark. */
const USES = [
    { label: "Notes", size: "1.2 GB", mark: swatch.series1Mark },
    { label: "Files", size: "0.6 GB", mark: swatch.series2Mark },
    { label: "Photos", size: "0.2 GB", mark: swatch.series3Mark },
] as const;

/** The styles of the storage card: its width, its bar and its legend. */
const storage = style.create({
    card: {
        width: width.popover,
        maxWidth: "100%",
    },
    content: {
        display: "flex",
        flexDirection: "column",
        gap: space[3],
    },
    legend: {
        display: "flex",
        flexDirection: "column",
        gap: space[1],
        margin: 0,
        padding: 0,
        listStyle: "none",
    },
    use: {
        display: "flex",
        alignItems: "center",
        gap: space[2],
    },
    dot: (mark: string) => ({
        width: space[2],
        height: space[2],
        borderRadius: radius.full,
        backgroundColor: mark,
    }),
    size: {
        marginInlineStart: "auto",
        color: color.mutedForeground,
        fontVariantNumeric: "tabular-nums",
    },
});

/** A storage plan's usage bar, what uses the space, an upgrade and a way to manage it. */
export const cardStoragePlan = defineExample({
    of: Card,
    name: "storage-plan",
    description:
        "a storage plan's usage bar, what uses the space, an upgrade and a way to manage it",
    render: () => (
        <Card xstyle={storage.card}>
            <CardHeader>
                <CardTitle>Storage</CardTitle>
                <CardDescription>2 of 5 GB used</CardDescription>
                <CardAction>
                    <Button size="sm">Upgrade</Button>
                </CardAction>
            </CardHeader>
            <CardContent xstyle={storage.content}>
                <Progress value={40} aria-label="Storage used" />
                <ul {...style.attrs(text.xs, storage.legend)}>
                    <For each={USES}>
                        {(use) => (
                            <li {...style.attrs(storage.use)}>
                                <span {...style.attrs(storage.dot(use.mark))} />
                                {use.label}
                                <span {...style.attrs(storage.size)}>{use.size}</span>
                            </li>
                        )}
                    </For>
                </ul>
            </CardContent>
            <CardFooter>
                <Button variant="outline" size="sm">
                    Manage storage
                </Button>
            </CardFooter>
        </Card>
    ),
});

/** A note's card with its text and two actions in the footer. */
export const cardNotePreview = defineExample({
    of: Card,
    name: "note-preview",
    description: "a note's card with its text and two actions in the footer",
    render: () => (
        <Card>
            <CardHeader>
                <CardTitle>Trip to Lisbon</CardTitle>
                <CardDescription>Edited yesterday</CardDescription>
            </CardHeader>
            <CardContent>Tram 28 in the morning, pastries in Belém after lunch.</CardContent>
            <CardFooter>
                <Button>Open</Button>
                <Button variant="ghost">Archive</Button>
            </CardFooter>
        </Card>
    ),
});

/** The grid of the usage figures, three to a row. */
const styles = style.create({
    figures: {
        display: "grid",
        gridTemplateColumns: "repeat(3, minmax(0, 1fr))",
        gap: space[2],
        width: "100%",
    },
});

/** A space's usage as three small cards, each a figure and its label. */
export const cardUsageFigures = defineExample({
    of: Card,
    name: "usage-figures",
    description: "a space's usage as three small cards, each a figure and its label",
    layout: "padded",
    properties: {
        size: "sm",
    },
    render: (properties) => (
        <div {...style.attrs(styles.figures)}>
            <Card {...properties}>
                <CardHeader>
                    <CardDescription>Storage</CardDescription>
                    <CardTitle>2.4 GB</CardTitle>
                </CardHeader>
            </Card>
            <Card {...properties}>
                <CardHeader>
                    <CardDescription>Objects</CardDescription>
                    <CardTitle>18,204</CardTitle>
                </CardHeader>
            </Card>
            <Card {...properties}>
                <CardHeader>
                    <CardDescription>Requests today</CardDescription>
                    <CardTitle>3,117</CardTitle>
                </CardHeader>
            </Card>
        </div>
    ),
});
