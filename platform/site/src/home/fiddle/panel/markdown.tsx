type MarkdownPanelProps = {
    html: string;
    onLink: (href: string) => boolean;
};

export function MarkdownPanel(props: MarkdownPanelProps) {
    const clickLink = (event: MouseEvent) => {
        if (!(event.target instanceof Element)) {
            return;
        }

        const link = event.target.closest("a[href]");
        if (!(link instanceof HTMLAnchorElement)) {
            return;
        }

        const isHandled = props.onLink(link.getAttribute("href") ?? "");
        if (isHandled) {
            event.preventDefault();
        }
    };

    return <article class="markdown-block" innerHTML={props.html} onClick={clickLink} />;
}
