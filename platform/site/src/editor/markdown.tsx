type MarkdownViewProps = {
    html: string;
    onLink: (href: string) => boolean;
};

export function MarkdownView(props: MarkdownViewProps) {
    // intercept workspace-relative links so they open in the editor
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

    return <article class="editor-markdown" innerHTML={props.html} onClick={clickLink} />;
}
