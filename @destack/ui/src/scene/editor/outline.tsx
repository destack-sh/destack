import { t } from "@destack/locale";
import * as style from "@destack/style";
import { space } from "@destack/theme/tokens.stylex";
import { createMemo, createSignal, For, type JSX, useLocale } from "@destack/view";
import { Icon } from "@destack/ui/icon";
import { SearchField } from "../../search-field/index.ts";
import { Tree, TreeItem } from "../../tree/index.ts";
import { Pane } from "./pane.tsx";
import { wordsOf } from "./words.ts";

/** The styles of the outline. */
const styles = style.create({
    search: {
        flexGrow: 1,
    },
    tree: {
        padding: space[2],
    },
    scenario: {
        display: "inline-flex",
        alignItems: "center",
        gap: space[1],
    },
});

/** One subject of the outline: what its examples and scenarios show. */
export interface Subject {
    /** The subject's name, such as `tabs`. */
    readonly name: string;
    /** The members of its examples, in order. */
    readonly examples: readonly string[];
    /** The members of its scenarios, in order. */
    readonly scenarios: readonly string[];
}

/** List a package's examples and scenarios by what they show, finding them by their words. */
export function Outline(properties: {
    /** The subjects. */
    readonly subjects: readonly Subject[];
    /** The name of the example or scenario shown. */
    readonly shown: string;
    /** Handle a person choosing an example or scenario by its name. */
    readonly onChoose: (name: string) => void;
}): JSX.Element {
    // write the strings in the person's language
    const localization = useLocale();

    // list only the subjects and members whose words contain what a person types
    const [text, setText] = createSignal("");
    const found = createMemo(() => {
        // match what a person types against the words, ignoring case
        const needle = text().trim().toLowerCase();
        const matches = (words: string) => needle === "" || words.toLowerCase().includes(needle);

        return properties.subjects.flatMap((subject) => {
            // list every member of a matched subject, else the members that match
            const isSubject = matches(wordsOf(subject.name, true));
            const examples = subject.examples.filter(
                (each) => isSubject || matches(wordsOf(each, false)),
            );
            const scenarios = subject.scenarios.filter(
                (each) => isSubject || matches(wordsOf(each, false)),
            );

            return examples.length + scenarios.length === 0
                ? []
                : [[subject.name, { examples, scenarios }] as const];
        });
    });
    const members = createMemo(() => new Map(found()));
    const names = createMemo(() => found().map(([name]) => name));

    return (
        <Pane
            isSide
            header={
                <SearchField
                    aria-label={localization.render(t`Find an example`)}
                    placeholder={localization.render(t`Find an example`)}
                    value={text()}
                    onValueChange={setText}
                    xstyle={styles.search}
                />
            }
        >
            {/* Tree */}
            <div {...style.attrs(styles.tree)}>
                <Tree
                    aria-label={localization.render(t.context("title")`Examples`)}
                    value={properties.shown}
                    onValueChange={(next) => {
                        if (next !== undefined && next.includes(".")) {
                            properties.onChoose(next);
                        }
                    }}
                >
                    <For each={names()}>
                        {(subject) => (
                            <TreeItem
                                value={subject}
                                label={wordsOf(subject, true)}
                                defaultExpanded={
                                    properties.shown.startsWith(`${subject}.`) || text() !== ""
                                }
                            >
                                <For each={members().get(subject)?.examples ?? []}>
                                    {(member) => (
                                        <TreeItem
                                            value={`${subject}.${member}`}
                                            label={wordsOf(member, true)}
                                        />
                                    )}
                                </For>
                                <For each={members().get(subject)?.scenarios ?? []}>
                                    {(member) => (
                                        <TreeItem
                                            value={`${subject}.${member}`}
                                            label={
                                                <span {...style.attrs(styles.scenario)}>
                                                    <Icon
                                                        name="play"
                                                        label={localization.render(
                                                            t.context("option")`Scenario`,
                                                        )}
                                                    />
                                                    {wordsOf(member, true)}
                                                </span>
                                            }
                                        />
                                    )}
                                </For>
                            </TreeItem>
                        )}
                    </For>
                </Tree>
            </div>
        </Pane>
    );
}
