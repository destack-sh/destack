import { t } from "@destack/locale";
import { Button } from "../button/index.ts";
import plusIcon from "@destack/icon/phosphor/plus";
import arrowElbowUpLeftIcon from "@destack/icon/phosphor/arrow-elbow-up-left";
import { defineExample } from "@destack/package/declare";
import { createSignal, For, Show, useEnvironment } from "@destack/view";
import { Dropzone, DropzoneOverlay, DropzoneTrigger } from "../dropzone/index.ts";
import { FileItem, FileItemGroup } from "../file-item/index.ts";
import { Icon } from "../icon/index.ts";
import { Stack } from "../stack/index.ts";
import {
    Composer,
    ComposerAddon,
    ComposerFooter,
    ComposerFrame,
    ComposerHeader,
    ComposerInput,
    ComposerSubmit,
} from "./composer.tsx";
import { Quote, QuoteContent, QuoteDismiss, QuoteIcon } from "../quote/index.ts";

/** How long the assistant of the examples answers, in milliseconds. */
const ANSWER = 4_000;

/** A question for an assistant, which a press stops answering. */
export const composerAssistant = defineExample({
    of: Composer,
    name: "assistant",
    description: t`A question for an assistant, which a press stops answering.`,
    layout: "padded",
    render: () => {
        // answer for a while
        const { clock } = useEnvironment();
        const [words, setWords] = createSignal("");
        const [timer, setTimer] = createSignal<number | undefined>(undefined);

        return (
            <Composer
                busy={timer() !== undefined}
                onSubmit={() => {
                    setWords("");
                    setTimer(clock.setTimeout(() => setTimer(undefined), ANSWER));
                }}
                onStop={() => {
                    clock.clearTimeout(timer());
                    setTimer(undefined);
                }}
            >
                <ComposerFrame>
                    <ComposerInput
                        placeholder="Ask anything"
                        value={words()}
                        onInput={(event) => setWords(event.currentTarget.value)}
                    />
                    <ComposerFooter>
                        <ComposerSubmit disabled={timer() === undefined && words().trim() === ""} />
                    </ComposerFooter>
                </ComposerFrame>
            </Composer>
        );
    },
});

/** An answer to a message, with files dropped on it. */
export const composerReply = defineExample({
    of: Composer,
    name: "reply",
    description: t`An answer to a message, which takes files dropped on it or chosen.`,
    layout: "padded",
    render: () => {
        const [files, setFiles] = createSignal<readonly File[]>([
            new File([new Uint8Array(48_120)], "quotes.pdf", { type: "application/pdf" }),
        ]);
        const [isQuoted, setQuoted] = createSignal(true);

        return (
            <Dropzone
                variant="plain"
                multiple
                onFiles={(added) => setFiles((current) => [...current, ...added])}
            >
                <Composer onSubmit={() => setFiles([])}>
                    <ComposerFrame>
                        <ComposerHeader>
                            <Show when={isQuoted()}>
                                <Quote>
                                    <QuoteIcon>
                                        <Icon icon={arrowElbowUpLeftIcon} />
                                    </QuoteIcon>
                                    <QuoteContent>
                                        <strong>Grace Hopper</strong>
                                        <div>
                                            Can you send me the quotes again? The client asked twice
                                            today.
                                        </div>
                                    </QuoteContent>
                                    <QuoteDismiss onClick={() => setQuoted(false)} />
                                </Quote>
                            </Show>
                            <Show when={files().length > 0}>
                                <FileItemGroup aria-label="Files">
                                    <For each={files()}>
                                        {(file) => (
                                            <FileItem
                                                file={file}
                                                source={file}
                                                onRemove={() =>
                                                    setFiles((all) =>
                                                        all.filter((each) => each !== file),
                                                    )
                                                }
                                            />
                                        )}
                                    </For>
                                </FileItemGroup>
                            </Show>
                        </ComposerHeader>
                        <ComposerInput placeholder="Write to Grace" />
                        <ComposerFooter>
                            <DropzoneTrigger
                                render={(attributes) => (
                                    <Button
                                        isIconOnly
                                        variant="ghost"

                                        shape="round"
                                        {...attributes}
                                        aria-label="Add files"
                                    >
                                        <Icon icon={plusIcon} />
                                    </Button>
                                )}
                            />
                            <ComposerSubmit />
                        </ComposerFooter>
                    </ComposerFrame>
                </Composer>
                <DropzoneOverlay />
            </Dropzone>
        );
    },
});

/** A composer that is off, and one that waits for the command key. */
export const composerDisabled = defineExample({
    of: Composer,
    name: "disabled",
    description: t`A composer that takes nothing, and one that sends by the command key with Enter.`,
    layout: "padded",
    render: () => (
        <Stack space="4">
            <Composer disabled>
                <ComposerFrame>
                    <ComposerInput placeholder="This channel is archived" />
                    <ComposerFooter>
                        <ComposerSubmit />
                    </ComposerFooter>
                </ComposerFrame>
            </Composer>
            <Composer submitMode="ctrlEnter" onSubmit={() => {}}>
                <ComposerFrame>
                    <ComposerInput placeholder="Comment" />
                    <ComposerFooter>
                        <ComposerSubmit />
                    </ComposerFooter>
                </ComposerFrame>
            </Composer>
        </Stack>
    ),
});

/** The surfaces of a frame, and one row with its buttons beside the text. */
export const composerSurfaces = defineExample({
    of: Composer,
    name: "surfaces",
    description: t`A frame raised, filled and drawn by its edge, its buttons beside the text.`,
    layout: "padded",
    render: () => (
        <Stack space="4">
            <For each={["raised", "muted", "outline"] as const}>
                {(variant) => (
                    <Composer onSubmit={() => {}}>
                        <ComposerFrame variant={variant}>
                            <ComposerAddon align="inline-start">
                                <Button
                                    isIconOnly
                                    variant="ghost"

                                    shape="round"
                                    aria-label="Add files"
                                >
                                    <Icon icon={plusIcon} />
                                </Button>
                            </ComposerAddon>
                            <ComposerInput placeholder="Ask anything" />
                            <ComposerAddon align="inline-end">
                                <ComposerSubmit />
                            </ComposerAddon>
                        </ComposerFrame>
                    </Composer>
                )}
            </For>
        </Stack>
    ),
});
