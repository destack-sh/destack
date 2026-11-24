import paperclip from "@destack/icon/phosphor/paperclip";
import { defineExample } from "@destack/package/declare";
import { createSignal, For } from "@destack/view";
import { Button } from "../button/index.ts";
import { Cluster } from "../cluster/index.ts";
import { Field, FieldDescription, FieldLabel } from "../field/index.ts";
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

/** An application's documents, two to four of them, which its form submits. */
export const dropzoneForm = defineExample({
    of: Dropzone,
    name: "form",
    description: "an application's documents, two to four of them, which its form submits",
    layout: "padded",
    render: () => {
        const [files, setFiles] = createSignal<readonly File[]>([]);
        const [sent, setSent] = createSignal("");

        return (
            <form
                onSubmit={(event) => {
                    event.preventDefault();
                    const { length } = new FormData(event.currentTarget).getAll("documents");
                    setSent(`Sent ${String(length)}`);
                }}
            >
                <Stack space="3">
                    <Field>
                        <FieldLabel>Documents</FieldLabel>
                        <Dropzone
                            multiple
                            required
                            name="documents"
                            limits={{ accept: ["application/pdf", "image/*"], count: 4 }}
                            validate={({ name }) =>
                                name.startsWith("~") ? `${name} is a draft` : undefined
                            }
                            files={files()}
                            onFiles={(added) => setFiles((current) => [...current, ...added])}
                            onReset={() => setFiles([])}
                        />
                        <FieldDescription>Your passport and your ticket.</FieldDescription>
                    </Field>
                    <FileItemGroup aria-label="Documents">
                        <For each={files()}>
                            {(file) => (
                                <FileItem
                                    file={file}
                                    source={file}
                                    onRemove={() =>
                                        setFiles((current) =>
                                            current.filter((each) => each !== file),
                                        )
                                    }
                                />
                            )}
                        </For>
                    </FileItemGroup>
                    <Cluster space="2">
                        <Button type="submit">Send</Button>
                        <Button type="reset" variant="outline">
                            Start over
                        </Button>
                        <output>{sent()}</output>
                    </Cluster>
                </Stack>
            </form>
        );
    },
});

/** A page that takes a folder dropped anywhere on it. */
export const dropzonePage = defineExample({
    of: Dropzone,
    name: "page",
    description: "a page that takes a folder dropped anywhere on it",
    layout: "padded",
    render: () => {
        const [paths, setPaths] = createSignal<readonly string[]>([]);

        return (
            <Dropzone
                variant="plain"
                target="page"
                multiple
                directory
                onFiles={(files, pathOf) =>
                    setPaths((current) => [...current, ...files.map(pathOf)])
                }
            >
                <Stack space="3">
                    <DropzoneTrigger
                        render={(attributes) => (
                            <Button {...attributes} variant="outline">
                                Choose a folder
                            </Button>
                        )}
                    />
                    <For each={paths()}>{(path) => <span>{path}</span>}</For>
                </Stack>
                <DropzoneRefusal />
                <DropzoneOverlay target="page" />
            </Dropzone>
        );
    },
});
