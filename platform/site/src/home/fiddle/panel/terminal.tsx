type TerminalPanelProps = {
    html: string;
};

export function TerminalPanel(props: TerminalPanelProps) {
    return (
        <pre class="code-block terminal-block">
            <code innerHTML={props.html} />
        </pre>
    );
}
