import brain from "@destack/icon/phosphor/brain";
import caretLeft from "@destack/icon/phosphor/caret-left";
import caretRight from "@destack/icon/phosphor/caret-right";
import checkCircle from "@destack/icon/phosphor/check-circle";
import fileText from "@destack/icon/phosphor/file-text";
import globe from "@destack/icon/phosphor/globe";
import prohibit from "@destack/icon/phosphor/prohibit";
import { schema } from "@destack/schema";
import * as style from "@destack/style";
import { text } from "@destack/theme/text";
import { color, space, weight } from "@destack/theme/tokens.stylex";
import { createSignal, For, type JSX, Show, useLocale } from "@destack/view";
import { Badge } from "../../badge/index.ts";
import { Button } from "../../button/index.ts";
import { Card, CardContent, CardFooter } from "../../card/index.ts";
import { Cluster } from "../../cluster/index.ts";
import {
    Collapsible,
    CollapsibleContent,
    CollapsibleIndicator,
    CollapsibleTrigger,
} from "../../collapsible/index.ts";
import { HoverCard, HoverCardContent, HoverCardTrigger } from "../../hover-card/index.ts";
import { Icon } from "../../icon/index.ts";
import { useMarkdown } from "../../markdown/index.ts";
import { Marker, MarkerContent, MarkerDescription, MarkerIcon } from "../../marker/index.ts";
import { RichText, type RichTextMarkProperties } from "../../rich-text/index.ts";
import { Stack } from "../../stack/index.ts";
import type { Answer, Chat, ChatMessage, Source, Step } from "./chat.ts";
import type { Assistant } from "./assistant.ts";

/** The value of a link. */
const Link = schema.looseObject({ href: schema.string() });

/** The icons of what the assistant does. */
const ICONS = { thought: brain, search: globe, read: fileText, ask: prohibit } as const;

/** The styles of an answer. */
const styles = style.create({
    title: {
        fontWeight: weight.medium,
    },
    muted: {
        color: color.mutedForeground,
    },
    source: {
        display: "inline",
        textDecorationLine: "none",
        marginInlineStart: space[1],
    },
    count: {
        fontVariantNumeric: "tabular-nums",
    },
});

/** Render an answer: what the assistant did, what it asks, and what it wrote. */
export function AnswerView(properties: {
    /** The chat. */
    readonly chat: Chat;
    /** The assistant. */
    readonly assistant: Assistant;
    /** The message it belongs to. */
    readonly message: ChatMessage;
    /** The answer. */
    readonly answer: Answer;
}): JSX.Element {
    // draw a link to what it uses as its source
    const answer = () => properties.answer;
    const isWriting = () => answer().status === "writing";
    const blocks = useMarkdown(() => answer().text, { isStreaming: isWriting });
    const link = (mark: RichTextMarkProperties): JSX.Element => {
        const href = (): string => addressOf(mark.value);

        return (
            <Show
                when={answer().sources.find(({ address }) => address === href())}
                fallback={<a href={href()}>{mark.children}</a>}
            >
                {(source) => <SourceLink source={source()} />}
            </Show>
        );
    };
    const asked = () => answer().steps.find(({ status }) => status === "waiting");

    return (
        <Stack space="3">
            <Show when={answer().steps.length > 0 || answer().status === "working"}>
                <Work answer={answer()} />
            </Show>
            <Show when={asked()}>
                {(step) => (
                    <Card size="sm">
                        <CardContent>
                            <Stack space="1">
                                <span {...style.attrs(styles.title)}>{step().title}?</span>
                                <span {...style.attrs(styles.muted)}>{step().text}</span>
                            </Stack>
                        </CardContent>
                        <CardFooter>
                            <Cluster space="2">
                                <Button
                                    size="sm"
                                    onClick={() =>
                                        properties.assistant.decide(
                                            properties.message.id,
                                            answer().id,
                                            true,
                                        )
                                    }
                                >
                                    Send the mail
                                </Button>
                                <Button
                                    size="sm"
                                    variant="ghost"
                                    onClick={() =>
                                        properties.assistant.decide(
                                            properties.message.id,
                                            answer().id,
                                            false,
                                        )
                                    }
                                >
                                    Leave it
                                </Button>
                            </Cluster>
                        </CardFooter>
                    </Card>
                )}
            </Show>
            <Show when={answer().text !== ""}>
                <RichText
                    blocks={blocks()}
                    isStreaming={isWriting()}
                    components={{ marks: { link } }}
                />
            </Show>
            <Show when={answer().status === "stopped"}>
                <Marker>
                    <MarkerContent>Stopped</MarkerContent>
                </Marker>
            </Show>
        </Stack>
    );
}

/** Render what the assistant did, open while it works. */
function Work(properties: {
    /** The answer. */
    readonly answer: Answer;
}): JSX.Element {
    // follow the work until the person opens or closes it
    const locale = useLocale();
    const [chosen, setChosen] = createSignal<boolean>();
    const isWorking = () => properties.answer.status === "working";
    const isAsking = () => properties.answer.steps.some(({ status }) => status === "waiting");
    const title = () => {
        // name what it is at, and afterwards how long it took
        const { steps, began, wrote } = properties.answer;
        const last = steps.at(-1);
        if (isAsking()) {
            return "Waiting for you";
        } else if (isWorking()) {
            return last?.title ?? "Thinking";
        } else if (wrote === undefined) {
            return "Worked";
        }
        const seconds = Math.max(1, Math.round((wrote - began) / 1_000));

        return `Worked for ${locale.number(seconds)} s`;
    };

    return (
        <Collapsible open={chosen() ?? false} onOpenChange={(next) => void setChosen(next)}>
            <Marker
                status={isWorking() && !isAsking() ? "running" : undefined}
                render={(attributes) => <CollapsibleTrigger {...attributes} />}
            >
                <MarkerContent>{title()}</MarkerContent>
                <CollapsibleIndicator />
            </Marker>
            <CollapsibleContent variant="line">
                <Stack space="2">
                    <For each={properties.answer.steps} keyed={(step) => step.id}>
                        {(step) => <StepView step={step()} />}
                    </For>
                </Stack>
            </CollapsibleContent>
        </Collapsible>
    );
}

/** Render one thing the assistant did. */
function StepView(properties: {
    /** The step. */
    readonly step: Step;
}): JSX.Element {
    return (
        <Stack space="1">
            <Marker status={properties.step.status === "running" ? "running" : undefined}>
                <MarkerIcon>
                    <Icon
                        icon={
                            properties.step.kind === "ask" && properties.step.status === "done"
                                ? checkCircle
                                : ICONS[properties.step.kind]
                        }
                    />
                </MarkerIcon>
                <MarkerContent>{properties.step.title}</MarkerContent>
                <Show when={properties.step.status === "denied"}>
                    <MarkerDescription>not allowed</MarkerDescription>
                </Show>
            </Marker>
            <Show when={properties.step.status !== "running"}>
                <span {...style.attrs(text.sm, styles.muted)}>{properties.step.text}</span>
            </Show>
        </Stack>
    );
}

/** Render a source in the text, which says what it is on hover and focus. */
function SourceLink(properties: {
    /** The source. */
    readonly source: Source;
}): JSX.Element {
    return (
        <HoverCard>
            <HoverCardTrigger
                href={properties.source.address}
                target="_blank"
                rel="noreferrer"
                xstyle={styles.source}
            >
                <Badge variant="secondary">{properties.source.site}</Badge>
            </HoverCardTrigger>
            <HoverCardContent>
                <Stack space="1">
                    <span {...style.attrs(text.sm, styles.title)}>{properties.source.title}</span>
                    <span {...style.attrs(text.sm)}>{properties.source.excerpt}</span>
                    <span {...style.attrs(text.xs, styles.muted)}>{properties.source.site}</span>
                </Stack>
            </HoverCardContent>
        </HoverCard>
    );
}

/** Render which of a message's answers shows, and the steps to the others. */
export function AnswerPicker(properties: {
    /** The chat. */
    readonly chat: Chat;
    /** The message. */
    readonly message: ChatMessage;
}): JSX.Element {
    // step through the answers
    const locale = useLocale();
    const { chat } = properties;
    const count = () => properties.message.answers.length;
    const show = (shown: number) => chat.change(properties.message.id, () => ({ shown }));

    return (
        <Show when={count() > 1}>
            <Cluster space="1" align="center" role="group" aria-label="Answers">
                <Button
                    size="icon-sm"
                    variant="ghost"
                    aria-label="Previous answer"
                    disabled={properties.message.shown === 0}
                    onClick={() => show(properties.message.shown - 1)}
                >
                    <Icon icon={caretLeft} />
                </Button>
                <span {...style.attrs(text.sm, styles.count)}>
                    {locale.number(properties.message.shown + 1)} / {locale.number(count())}
                </span>
                <Button
                    size="icon-sm"
                    variant="ghost"
                    aria-label="Next answer"
                    disabled={properties.message.shown === count() - 1}
                    onClick={() => show(properties.message.shown + 1)}
                >
                    <Icon icon={caretRight} />
                </Button>
            </Cluster>
        </Show>
    );
}

/** Read the address of a link's value. */
function addressOf(value: RichTextMarkProperties["value"]): string {
    return Link.safeParse(value).data?.href ?? "";
}
