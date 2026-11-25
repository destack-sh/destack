import fileIcon from "@destack/icon/phosphor/file";
import plusIcon from "@destack/icon/phosphor/plus";
import { defineExample } from "@destack/package/declare";
import * as style from "@destack/style";
import { text } from "@destack/theme/text";
import { color, radius, space, stroke } from "@destack/theme/tokens.stylex";
import { createSignal, type JSX } from "@destack/view";
import { Button } from "../button/index.ts";
import { Empty, EmptyDescription, EmptyHeader, EmptyTitle } from "../empty/index.ts";
import { Icon } from "../icon/index.ts";
import { TabsClose, TabsTrigger } from "../tabs/index.ts";
import { PaneBar, PaneContent, PaneGroup, PaneLayout, PaneTab, PaneTabs, Pane } from "./index.ts";

/** The styles of the examples' windows. */
const styles = style.create({
    window: {
        height: "28rem",
        borderWidth: stroke.border,
        borderColor: color.border,
        borderRadius: radius[4],
    },
    editor: {
        display: "block",
        width: "100%",
        height: "100%",
        padding: space[4],
        borderWidth: 0,
        resize: "none",
        backgroundColor: "transparent",
        color: color.foreground,
        outlineStyle: "none",
    },
    bar: {
        padding: space[2],
        paddingInline: space[3],
        borderBlockEndWidth: stroke.border,
        borderBlockEndColor: color.border,
        cursor: "grab",
    },
    tile: {
        padding: space[4],
    },
});

/** Files in panes, which split, merge and keep what is typed in them. */
export const paneEditor = defineExample({
    of: PaneGroup,
    name: "editor",
    description: "files in panes, which split, merge and keep what is typed in them",
    render: () => <Editor />,
});

/** Render files in panes. */
function Editor(): JSX.Element {
    const [layout, setLayout] = createSignal(
        PaneLayout.of(["plan.md", "tabs.tsx", "pane.tsx"], ["README.md"]),
    );

    return (
        <PaneGroup
            layout={layout()}
            onLayoutChange={setLayout}
            xstyle={styles.window}
            content={(item) => (
                <textarea
                    aria-label={item().value}
                    {...style.attributes([text.sm, styles.editor])}
                >{`// ${item().value}`}</textarea>
            )}
            fallback={
                <Empty>
                    <EmptyHeader>
                        <EmptyTitle>No file open</EmptyTitle>
                        <EmptyDescription>Open one from the sidebar.</EmptyDescription>
                    </EmptyHeader>
                </Empty>
            }
        />
    );
}

/** The views of a dashboard in panes without tabs. */
export const paneTiles = defineExample({
    of: PaneGroup,
    name: "tiles",
    description: "a dashboard's views in panes without tabs, which move by their bars",
    render: () => <Tiles />,
});

/** Render a dashboard's views. */
function Tiles(): JSX.Element {
    const [layout, setLayout] = createSignal(
        PaneLayout.open(PaneLayout.of(["Sales"], ["Visits"]), "Orders", {
            kind: "side",
            side: "bottom",
        }),
    );

    return (
        <PaneGroup
            layout={layout()}
            onLayoutChange={setLayout}
            xstyle={styles.window}
            content={(item) => <div {...style.attrs(styles.tile)}>{item().value} today</div>}
        >
            {(pane) => (
                <Pane>
                    <PaneBar xstyle={[text.sm, styles.bar]}>
                        {PaneLayout.selected(pane())?.value}
                    </PaneBar>
                    <PaneContent />
                </Pane>
            )}
        </PaneGroup>
    );
}

/** Panes with tabs of one's own. */
export const paneCustom = defineExample({
    of: PaneGroup,
    name: "custom",
    description: "panes with tabs, a look and a button of one's own",
    render: () => <Custom />,
});

/** Render panes with tabs of one's own. */
function Custom(): JSX.Element {
    const [layout, setLayout] = createSignal(PaneLayout.of(["one", "two"], ["three"]));
    const open = (pane: string): void => {
        setLayout((all) =>
            PaneLayout.open(
                all,
                { id: PaneLayout.freeItem(all, "new"), value: "new" },
                { kind: "item", pane },
            ),
        );
    };

    return (
        <PaneGroup
            layout={layout()}
            onLayoutChange={setLayout}
            xstyle={styles.window}
            content={(item) => <div {...style.attrs(styles.tile)}>{item().value}</div>}
        >
            {(pane) => (
                <Pane>
                    <PaneTabs
                        variant="subtle"
                        end={
                            <Button
                                variant="ghost"
                                size="icon"
                                aria-label="New"
                                onClick={() => open(pane().id)}
                            >
                                <Icon icon={plusIcon} />
                            </Button>
                        }
                    >
                        {(item) => (
                            <PaneTab item={item()}>
                                <TabsTrigger>
                                    <Icon icon={fileIcon} /> {item().value}
                                </TabsTrigger>
                                <TabsClose />
                            </PaneTab>
                        )}
                    </PaneTabs>
                    <PaneContent />
                </Pane>
            )}
        </PaneGroup>
    );
}
