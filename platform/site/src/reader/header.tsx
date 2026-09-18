import { type JSX, Show } from "@destack/view";

/// Page intent determines hierarchy; optional fields never reserve empty space.
export function PageHeader(props: {
    title: string;
    variant: "article" | "chapter" | "reference";
    description?: string;
    children?: JSX.Element;
}) {
    return (
        <header class="content-header" data-variant={props.variant}>
            <h1>{props.title}</h1>
            <Show when={props.description}>
                <p class="content-description">{props.description}</p>
            </Show>
            {props.children}
        </header>
    );
}
