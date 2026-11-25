import { defineExample } from "@destack/package/declare";
import * as style from "@destack/style";
import { text } from "@destack/theme/text";
import { color, space, weight } from "@destack/theme/tokens.stylex";
import { createSignal, type JSX } from "@destack/view";
import { Avatar, AvatarFallback, AvatarImage } from "../avatar/index.ts";
import { Button } from "../button/index.ts";
import { Checkbox, CheckboxGroup } from "../checkbox/index.ts";
import {
    ColorArea,
    ColorEyeDropper,
    ColorField,
    ColorPicker,
    ColorSlider,
    ColorSwatch,
} from "../color-picker/index.ts";
import {
    Cropper,
    CropperFlip,
    CropperImage,
    CropperRotate,
    CropperSelection,
    CropperViewport,
    CropperZoom,
    useCropper,
} from "../cropper/index.ts";
import {
    Dialog,
    DialogContent,
    DialogFooter,
    DialogHeader,
    DialogTitle,
    DialogTrigger,
} from "../dialog/index.ts";
import { Editable } from "../editable/index.ts";
import { PICTURES } from "../example/picture.ts";
import { Field, FieldDescription, FieldGroup, FieldLabel } from "../field/index.ts";
import { Label } from "../label/index.ts";
import { Meter, MeterLabel, MeterTrack, MeterValue } from "../meter/index.ts";
import { Popover, PopoverContent, PopoverTrigger } from "../popover/index.ts";
import { Slider } from "../slider/index.ts";
import { Stack } from "../stack/index.ts";
import { Switch } from "../switch/index.ts";
import { TagField, TagFieldInput, TagFieldList } from "../tag-field/index.ts";

/** The styles of the settings. */
const styles = style.create({
    page: {
        width: "100%",
        maxWidth: "36rem",
        marginInline: "auto",
    },
    header: {
        display: "flex",
        alignItems: "center",
        gap: space[4],
    },
    name: {
        margin: 0,
        fontWeight: weight.bold,
    },
    muted: {
        color: color.mutedForeground,
    },
    row: {
        display: "flex",
        alignItems: "center",
        justifyContent: "space-between",
        gap: space[3],
    },
    tools: {
        display: "flex",
        alignItems: "center",
        gap: space[2],
    },
    content: {
        width: "auto",
    },
});

/** Render the button that takes the part a person cropped. */
function Use(properties: { readonly onUse: (source: string) => void }): JSX.Element {
    const cropper = useCropper();
    const use = async (): Promise<void> =>
        properties.onUse(URL.createObjectURL(await cropper.toBlob({ width: 192 })));

    return <Button onClick={() => void use()}>Use picture</Button>;
}

/** A person's profile and how their account works. */
export const sceneSettings = defineExample({
    of: Cropper,
    name: "settings",
    description: "a profile: its picture cropped, its name in place, a color, tags and notices",
    layout: "padded",
    render: () => {
        // keep the picture and the color
        const [picture, setPicture] = createSignal<string>(PICTURES.ada);
        const [isOpen, setOpen] = createSignal(false);
        const [accent, setAccent] = createSignal("#f76b15");

        return (
            <Stack space="6" xstyle={styles.page}>
                {/* Picture and name */}
                <div {...style.attrs(styles.header)}>
                    <Dialog open={isOpen()} onOpenChange={(next) => void setOpen(next)}>
                        <DialogTrigger
                            render={(attributes) => (
                                <button type="button" aria-label="Change picture" {...attributes}>
                                    <Avatar size="lg">
                                        <AvatarImage src={picture()} alt="" />
                                        <AvatarFallback>AL</AvatarFallback>
                                    </Avatar>
                                </button>
                            )}
                        />
                        <DialogContent>
                            <DialogHeader>
                                <DialogTitle>Profile picture</DialogTitle>
                            </DialogHeader>
                            <Cropper fixedCropArea aspectRatio={1}>
                                <CropperViewport>
                                    <CropperImage src={PICTURES.alfama} alt="Roofs of Alfama" />
                                    <CropperSelection cropShape="circle" />
                                </CropperViewport>
                                <div {...style.attrs(styles.tools)}>
                                    <CropperZoom />
                                    <CropperRotate />
                                    <CropperFlip />
                                </div>
                                <DialogFooter>
                                    <Use
                                        onUse={(source) => {
                                            setPicture(source);
                                            setOpen(false);
                                        }}
                                    />
                                </DialogFooter>
                            </Cropper>
                        </DialogContent>
                    </Dialog>
                    <div>
                        <h1 {...style.attrs(text["2xl"], styles.name)}>
                            <Editable required defaultValue="Ada Lovelace" />
                        </h1>
                        <Editable
                            xstyle={[text.sm, styles.muted]}
                            placeholder="Add a line about yourself"
                        />
                    </div>
                </div>

                {/* Fields */}
                <FieldGroup>
                    <Field>
                        <FieldLabel>Topics</FieldLabel>
                        <TagField defaultValue={["engines", "poetry"]} max={8}>
                            <TagFieldList />
                            <TagFieldInput placeholder="Add a topic" />
                        </TagField>
                        <FieldDescription>People find you by these.</FieldDescription>
                    </Field>
                    <div {...style.attrs(styles.row)}>
                        <span {...style.attrs(text.sm)}>Accent</span>
                        <Popover>
                            <PopoverTrigger
                                render={(attributes) => (
                                    <Button {...attributes} variant="outline">
                                        <ColorSwatch color={accent()} />
                                        <bdi>{accent()}</bdi>
                                    </Button>
                                )}
                            />
                            <PopoverContent xstyle={styles.content} aria-label="Accent">
                                <ColorPicker
                                    value={accent()}
                                    onValueChange={(next) => void setAccent(next)}
                                >
                                    <ColorArea />
                                    <ColorSlider channel="hue" />
                                    <div {...style.attrs(styles.tools)}>
                                        <ColorField />
                                        <ColorEyeDropper />
                                    </div>
                                </ColorPicker>
                            </PopoverContent>
                        </Popover>
                    </div>
                    <div {...style.attrs(styles.row)}>
                        <Label for="settings-sound">Sounds</Label>
                        <Switch id="settings-sound" size="sm" defaultChecked />
                    </div>
                    <Slider aria-label="Volume" defaultValue={[60]} />
                    <CheckboxGroup defaultValue={["mentions"]}>
                        <Stack space="3">
                            <Label>
                                <Checkbox parent /> Every notice
                            </Label>
                            <Label>
                                <Checkbox value="mentions" /> Mentions
                            </Label>
                            <Label>
                                <Checkbox value="replies" /> Replies
                            </Label>
                        </Stack>
                    </CheckboxGroup>
                    <Meter
                        value={41}
                        max={50}
                        high={40}
                        optimum={0}
                        format={{ style: "unit", unit: "gigabyte" }}
                    >
                        <MeterLabel>Storage</MeterLabel>
                        <MeterValue>{(written) => `${written} of 50 GB`}</MeterValue>
                        <MeterTrack />
                    </Meter>
                </FieldGroup>
            </Stack>
        );
    },
});
