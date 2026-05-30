import { For } from "solid-js";

type CodePanelProps = {
    html: string;
};

export function CodePanel(props: CodePanelProps) {
    const lines = () => props.html.split("\n");

    return (
        <pre class="code-block numbered-code-block">
            <code>
                <For each={lines()}>
                    {(line, index) => (
                        <span class="numbered-code-line">
                            <span class="numbered-code-gutter">{index() + 1}</span>
                            <span
                                class="numbered-code-text"
                                innerHTML={line === "" ? " " : line}
                            />
                        </span>
                    )}
                </For>
            </code>
        </pre>
    );
}
