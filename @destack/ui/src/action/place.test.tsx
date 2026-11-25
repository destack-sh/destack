import { schema } from "@destack/schema";
import type { ObjectReference } from "@destack/sync";
import { expect, test } from "@destack/test";
import { ActionRegistry, flush, type JSX } from "@destack/view";
import { render, stubPopovers } from "@destack/view/test";
import { FileItem } from "../file-item/index.ts";
import { Gallery, GalleryItem, GridLayout } from "../gallery/index.ts";
import { GridList, GridListControl, GridListProvider } from "../grid-list/index.ts";
import { PaneGroup, PaneLayout } from "../pane/index.ts";
import { Tabs, TabsList, TabsTrigger } from "../tabs/index.ts";
import { Tree, TreeItem } from "../tree/index.ts";

/** Reference the note an item stands for. */
function noteOf(id: string): ObjectReference {
    return {
        packageId: schema
            .identifier("package")
            .parse("package-01996ab0-0000-7000-8000-000000000001"),
        type: "note",
        scope: "space",
        id,
    };
}

/** Render a grid list of one note. */
function Notes(): JSX.Element {
    const list = new GridListControl({
        sections: () => [{ key: "notes", items: ["a"] }],
        key: (note) => note,
        text: (note) => note,
        reference: noteOf,
    });

    return (
        <GridListProvider control={list}>
            <GridList control={list} aria-label="Notes" label={(note) => note} />
        </GridListProvider>
    );
}

test.for<readonly [name: string, selector: string, draw: () => JSX.Element]>([
    ["a grid list", "[data-item=a]", () => <Notes />],
    [
        "a tree",
        "[data-value=a]",
        () => (
            <Tree aria-label="Notes" reference={noteOf}>
                <TreeItem value="a" label="A" />
            </Tree>
        ),
    ],
    [
        "tabs",
        "[role=tab]",
        () => (
            <Tabs defaultValue="a" reference={noteOf}>
                <TabsList>
                    <TabsTrigger value="a">A</TabsTrigger>
                </TabsList>
            </Tabs>
        ),
    ],
    [
        "panes",
        "[role=tab]",
        () => (
            <PaneGroup
                layout={PaneLayout.of(["a"])}
                reference={noteOf}
                content={(item) => item().value}
            />
        ),
    ],
    [
        "a gallery",
        "[data-slot=gallery-open]",
        () => (
            <Gallery each={["a"]} layout={new GridLayout({ minimum: 100 })}>
                {(note) => (
                    <GalleryItem
                        label={note()}
                        reference={noteOf(note())}
                        onOpen={() => undefined}
                    />
                )}
            </Gallery>
        ),
    ],
    [
        "a file",
        "[data-slot=file-item]",
        () => (
            <FileItem
                file={{ name: "a.txt", type: "text/plain", size: 1 }}
                reference={noteOf("a")}
            />
        ),
    ],
])("give the object an item of %s stands for", ([, selector, draw]) => {
    // read what is selected at the item
    stubPopovers();
    const actions = new ActionRegistry();
    const { container } = render(draw, { actions });
    flush();

    expect(actions.selection(container.querySelector(selector))).toEqual([noteOf("a")]);
});
