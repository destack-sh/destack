import { Icon } from "@destack/icon";
import caretDown from "@destack/icon/phosphor/caret-down";
import { ButtonGroup } from "@destack/ui/button-group";
import { CopyButton } from "../layout/copy";
import {
    DropdownMenu,
    DropdownMenuContent,
    DropdownMenuItem,
    DropdownMenuSeparator,
    DropdownMenuTrigger,
} from "@destack/ui/dropdown-menu";
import { createSignal, onSettled } from "@destack/view";

import { commandEvents } from "../layout/command";
import { origin } from "../content/site";
import type { PageSource } from "../content/source";
import { captureException } from "../telemetry.ts";
import { createEventListener } from "@destack/view/primitives/event-listener";

/** Properties for the page source controls. */
type SourceActionsProperties = {
    /** The shared page source commands. */
    commands: PageSourceCommands;
};

/** One portable source format. */
type SourceKind = "md" | "txt";

/** The source commands shared by responsive reader controls. */
export type PageSourceCommands = {
    /** Copy one source format. */
    copy: (kind: SourceKind) => void;

    /** The current page source files. */
    source: PageSource;
};

/** Create source commands shared by responsive article controls. */
export function createPageSourceCommands(source: PageSource): PageSourceCommands {
    // copy one source format to the clipboard
    const write = async (kind: SourceKind) => {
        // load the requested source format
        const route = kind === "md" ? source.markdownRoute : source.textRoute;
        const response = await fetch(route);

        // fail on unavailable generated sources
        if (!response.ok) {
            throw new Error(`failed to load ${route}: ${response.status}`);
        }

        // publish the requested source
        await navigator.clipboard.writeText(await response.text());
    };

    // copy in the background, logging failures
    const copy = (kind: SourceKind) => {
        // report clipboard failures
        void write(kind).catch((error: unknown) => {
            captureException(error, { tags: { feature: "source" } });
        });
    };

    // bind the palette copy commands while the page shows
    createEventListener(
        () => document,
        commandEvents.copyMarkdown,
        () => copy("md"),
    );
    createEventListener(
        () => document,
        commandEvents.copyText,
        () => copy("txt"),
    );

    return { copy, source };
}

/** Offer the page as Markdown: copy it, view it, or open it in an assistant. */
export function SourceActions(properties: SourceActionsProperties) {
    // load the page's Markdown once the page settles, so the copy happens inside the click
    const commands = properties.commands;
    const [markdown, setMarkdown] = createSignal<string>();
    onSettled(() => {
        void fetch(commands.source.markdownRoute)
            .then(async (response) => {
                if (!response.ok) {
                    throw new Error(
                        `failed to load ${commands.source.markdownRoute}: ${response.status}`,
                    );
                }
                setMarkdown(await response.text());
            })
            .catch((error: unknown) => captureException(error, { tags: { feature: "source" } }));
    });

    // ask an assistant to read the page's Markdown
    const prompt = () => {
        const address = new URL(commands.source.markdownRoute, origin).href;

        return encodeURIComponent(`Read ${address} so I can ask questions about it.`);
    };

    return (
        <ButtonGroup aria-label="Page formats">
            <CopyButton
                value={markdown() ?? ""}
                disabled={markdown() === undefined}
                variant="outline"
                size="sm"
                onFailure={(error) => captureException(error, { tags: { feature: "source" } })}
            >
                Copy page
            </CopyButton>
            <DropdownMenu>
                <DropdownMenuTrigger
                    variant="outline"
                    size="icon-sm"
                    aria-label="More page formats"
                >
                    <Icon icon={caretDown} />
                </DropdownMenuTrigger>
                <DropdownMenuContent align="end">
                    <DropdownMenuItem
                        render={(item) => (
                            <a
                                {...item}
                                href={commands.source.markdownRoute}
                                type="text/markdown"
                                target="_blank"
                                rel="alternate noopener"
                            />
                        )}
                    >
                        View as Markdown
                    </DropdownMenuItem>
                    <DropdownMenuItem
                        render={(item) => (
                            <a
                                {...item}
                                href={commands.source.textRoute}
                                type="text/plain"
                                target="_blank"
                                rel="alternate noopener"
                            />
                        )}
                    >
                        View as plain text
                    </DropdownMenuItem>
                    <DropdownMenuSeparator />
                    <DropdownMenuItem
                        render={(item) => (
                            <a
                                {...item}
                                href={`https://claude.ai/new?q=${prompt()}`}
                                target="_blank"
                                rel="noopener"
                            />
                        )}
                    >
                        Open in Claude
                    </DropdownMenuItem>
                    <DropdownMenuItem
                        render={(item) => (
                            <a
                                {...item}
                                href={`https://chatgpt.com/?q=${prompt()}`}
                                target="_blank"
                                rel="noopener"
                            />
                        )}
                    >
                        Open in ChatGPT
                    </DropdownMenuItem>
                </DropdownMenuContent>
            </DropdownMenu>
        </ButtonGroup>
    );
}
