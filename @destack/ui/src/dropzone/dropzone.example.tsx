import paperclip from "@destack/icon/phosphor/paperclip";
import { defineExample } from "@destack/package/declare";
import { createSignal, For } from "@destack/view";
import { FileItem, FileItemGroup } from "../file-item/index.ts";
import { Icon } from "../icon/index.ts";
import {
    InputGroup,
    InputGroupAddon,
    InputGroupButton,
    InputGroupTextarea,
} from "../input-group/index.ts";
import { Stack } from "../stack/index.ts";
import { Dropzone, DropzoneOverlay, DropzoneRefusal, DropzoneTrigger } from "./dropzone.tsx";

/** A trip's photos, dropped or chosen and shown at once. */
export const dropzonePhotos = defineExample({
    of: Dropzone,
    name: "photos",
    description: "a trip's photos, dropped or chosen and shown at once",
    layout: "padded",
    render: () => {
        const [files, setFiles] = createSignal<readonly File[]>([]);

        return (
            <Stack space="3">
                <Dropzone
                    multiple
                    limits={{ accept: ["image/*"], size: 20_000_000 }}
                    onFiles={(added) => setFiles((current) => [...current, ...added])}
                />
                <FileItemGroup layout="card" aria-label="Photos">
                    <For each={files()}>
                        {(file) => (
                            <FileItem
                                layout="card"
                                file={file}
                                source={file}
                                onRemove={() =>
                                    setFiles((current) => current.filter((each) => each !== file))
                                }
                            />
                        )}
                    </For>
                </FileItemGroup>
            </Stack>
        );
    },
});

/** A message's text that takes files dropped anywhere on it, pasted into it or chosen beside it. */
export const dropzoneMessage = defineExample({
    of: Dropzone,
    name: "message",
    description:
        "a message's text that takes files dropped anywhere on it, pasted into it or chosen beside it",
    layout: "padded",
    render: () => {
        const [files, setFiles] = createSignal<readonly File[]>([]);

        return (
            <Dropzone
                variant="plain"
                multiple
                onFiles={(added) => setFiles((current) => [...current, ...added])}
            >
                <Stack space="2">
                    <InputGroup>
                        <InputGroupTextarea aria-label="Message" placeholder="Write to Ada" />
                        <InputGroupAddon align="block-end">
                            <DropzoneTrigger
                                render={(attributes) => (
                                    <InputGroupButton
                                        {...attributes}
                                        size="icon-sm"
                                        aria-label="Add files"
                                    >
                                        <Icon icon={paperclip} />
                                    </InputGroupButton>
                                )}
                            />
                        </InputGroupAddon>
                    </InputGroup>
                    <FileItemGroup aria-label="Files">
                        <For each={files()}>
                            {(file) => <FileItem size="sm" file={file} source={file} />}
                        </For>
                    </FileItemGroup>
                </Stack>
                <DropzoneRefusal />
                <DropzoneOverlay />
            </Dropzone>
        );
    },
});

/** A ticket in one row, which takes a PDF or a picture. */
export const dropzoneTicket = defineExample({
    of: Dropzone,
    name: "ticket",
    description: "a ticket in one row, which takes a PDF or a picture",
    layout: "padded",
    render: () => (
        <Dropzone
            size="sm"
            limits={{ accept: ["application/pdf", "image/*"], size: 5_000_000 }}
            onFiles={() => {}}
        />
    ),
});

/** Files of any kind, and a zone that takes none now. */
export const dropzoneFiles = defineExample({
    of: Dropzone,
    name: "files",
    description: "files of any kind, and a zone that takes none now",
    layout: "padded",
    render: () => (
        <Stack space="3">
            <Dropzone multiple onFiles={() => {}} />
            <Dropzone disabled size="sm" onFiles={() => {}} />
        </Stack>
    ),
});
