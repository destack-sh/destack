import typescript from "@destack/code/grammar/typescript";
import { Markdown } from "@destack/object/markdown";
import { defineExample } from "@destack/package/declare";
import { createSignal, For, type JSX, onCleanup, onSettled, useEnvironment } from "@destack/view";
import * as style from "@destack/style";
import { Bubble, BubbleContent } from "../bubble/index.ts";
import { Button } from "../button/index.ts";
import { Cluster } from "../cluster/index.ts";
import { useMarkdown } from "../markdown/index.ts";
import {
    MessageScroller,
    MessageScrollerButton,
    MessageScrollerContent,
    MessageScrollerItem,
    MessageScrollerViewport,
} from "../message-scroller/index.ts";
import { Stack } from "../stack/index.ts";
import { ToggleGroup, ToggleGroupItem } from "../toggle-group/index.ts";
import { RichText, type RichTextReveal } from "./index.ts";

/** An answer with every block. */
const ANSWER = `## The order form

The form goes to the printer on **Friday**. Three things are _still open_, and the [price list](https://example.com/prices) is ~~wrong~~ fixed:

1. Margins, at least \`5 mm\` on each side
2. The paper
   - 120 g, uncoated
   - the same as last year
3. The date

- [x] Ask the printer
- [ ] Send the proof

> Do not change the size after the proof.

\`\`\`ts
export function margin(width: number): number {
    return Math.max(5, width * 0.05);
}
\`\`\`

| Paper | Sheets | Price |
|:------|-------:|:-----:|
| 120 g | 1,000 | € 84 |
| 150 g | 1,000 | € 97 |

---

![A harbour at dusk](https://example.com/harbour.png)
`;

/** The characters that arrive at once, and after how long, as a network brings a model's. */
const PIECES = [
    { length: 24, milliseconds: 120 },
    { length: 6, milliseconds: 30 },
    { length: 60, milliseconds: 280 },
    { length: 15, milliseconds: 60 },
] as const;

/** What text comes up by. */
const REVEALS = ["line", "word", "character", "none"] as const;

/** How often a long answer has the answer, about ten kilobytes. */
const LENGTH = 14;

/** The styles of the conversation. */
const styles = style.create({
    frame: {
        height: "32rem",
    },
});

/** Color TypeScript. */
function grammarOf(language: string) {
    return language === "ts" ? typescript : undefined;
}

/** An answer with every block, read from Markdown. */
export const richTextAnswer = defineExample({
    of: RichText,
    name: "answer",
    description: "an answer with every block, read from Markdown",
    render: () => <RichText blocks={Markdown.parse(ANSWER)} grammar={grammarOf} />,
});

/** An answer as an agent writes it, again on a button. */
function Streamed(): JSX.Element {
    // bring it up as chosen
    const [reveal, setReveal] = createSignal<RichTextReveal>("line");
    // add pieces of uneven lengths at uneven times, as a network does
    const { clock } = useEnvironment();
    const [text, setText] = createSignal("", { ownedWrite: true });
    const [isWriting, setWriting] = createSignal(false, { ownedWrite: true });
    const blocks = useMarkdown(text, { isStreaming: isWriting });
    let writing = new AbortController();
    const write = async (signal: AbortSignal): Promise<void> => {
        // start empty, and end with the whole answer
        setText("");
        setWriting(true);
        for (let from = 0, turn = 0; from < ANSWER.length; turn += 1) {
            const { length, milliseconds } = PIECES[turn % PIECES.length] ?? PIECES[0];
            await clock.sleep(milliseconds, signal);
            from += length;
            setText(ANSWER.slice(0, from));
        }
        setWriting(false);
    };
    const start = (): void => {
        // stop writing, and write from the start
        writing.abort();
        writing = new AbortController();
        const { signal } = writing;
        write(signal).catch((error: unknown) => {
            if (!signal.aborted) {
                reportError(error);
            }
        });
    };
    onSettled(start);
    onCleanup(() => writing.abort());

    return (
        <Stack space="4">
            <Cluster space="3" align="center">
                <Button variant="outline" size="sm" onClick={start}>
                    Write again
                </Button>
                <ToggleGroup
                    aria-label="Comes up by"
                    size="sm"
                    value={reveal()}
                    onValueChange={(next) => {
                        // write again in the way chosen
                        setReveal(REVEALS.find((each) => each === next) ?? "line");
                        start();
                    }}
                >
                    <For each={REVEALS}>
                        {(each) => <ToggleGroupItem value={each}>{each}</ToggleGroupItem>}
                    </For>
                </ToggleGroup>
            </Cluster>
            <RichText
                blocks={blocks()}
                isStreaming={isWriting()}
                reveal={reveal()}
                grammar={grammarOf}
            />
        </Stack>
    );
}

/** An answer as an agent writes it, its finished blocks staying as they are. */
export const richTextStreamed = defineExample({
    of: RichText,
    name: "streamed",
    description: "an answer as an agent writes it, its finished blocks staying as they are",
    render: () => <Streamed />,
});

/** A long answer in a conversation, which follows it to its end. */
function Conversation(): JSX.Element {
    // write the answer several times over, as a network brings it
    const { clock } = useEnvironment();
    const long = ANSWER.repeat(LENGTH);
    const [text, setText] = createSignal("", { ownedWrite: true });
    const [isWriting, setWriting] = createSignal(true, { ownedWrite: true });
    const blocks = useMarkdown(text, { isStreaming: isWriting });
    const writing = new AbortController();
    const write = async (): Promise<void> => {
        for (let from = 0, turn = 0; from < long.length; turn += 1) {
            const { length, milliseconds } = PIECES[turn % PIECES.length] ?? PIECES[0];
            await clock.sleep(milliseconds, writing.signal);
            from += 2 * length;
            setText(long.slice(0, from));
        }
        setWriting(false);
    };
    onSettled(() => {
        write().catch((error: unknown) => {
            if (!writing.signal.aborted) {
                reportError(error);
            }
        });
    });
    onCleanup(() => writing.abort());

    return (
        <div {...style.attrs(styles.frame)}>
            <MessageScroller autoScroll>
                <MessageScrollerViewport>
                    <MessageScrollerContent>
                        <MessageScrollerItem messageId="question">
                            <Bubble>
                                <BubbleContent>What is open about the order form?</BubbleContent>
                            </Bubble>
                        </MessageScrollerItem>
                        <MessageScrollerItem messageId="answer">
                            <RichText
                                blocks={blocks()}
                                isStreaming={isWriting()}
                                grammar={grammarOf}
                            />
                        </MessageScrollerItem>
                    </MessageScrollerContent>
                </MessageScrollerViewport>
                <MessageScrollerButton />
            </MessageScroller>
        </div>
    );
}

/** A long answer in a conversation, which follows it to its end. */
export const richTextConversation = defineExample({
    of: RichText,
    name: "conversation",
    description: "a long answer in a conversation, which follows it to its end",
    layout: "padded",
    render: () => <Conversation />,
});
