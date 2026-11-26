import { t } from "@destack/locale";
import { defineExample, type Example } from "@destack/package/declare";
import * as style from "@destack/style";
import { text } from "@destack/theme/text";
import { color, radius, space, stroke, weight } from "@destack/theme/tokens.stylex";
import { For, type JSX } from "@destack/view";
import { accordionHelpQuestionsExpanded } from "../accordion/accordion.example.tsx";
import { alertFailedSync } from "../alert/alert.example.tsx";
import { alertDialogDeleteNotebook } from "../alert-dialog/alert-dialog.example.tsx";
import { autocompleteFruit } from "../autocomplete/autocomplete.example.tsx";
import { avatarDefault, avatarGroupDefault } from "../avatar/avatar.example.tsx";
import { badgeNoteState } from "../badge/badge.example.tsx";
import { boardLaunch } from "../board/board.example.tsx";
import { breadcrumbNoteTrail } from "../breadcrumb/breadcrumb.example.tsx";
import { buttonDefault, buttonDelete } from "../button/button.example.tsx";
import { buttonGroupSplitSave } from "../button-group/button-group.example.tsx";
import { calendarTripDays } from "../calendar/calendar.example.tsx";
import { cardStoragePlan } from "../card/card.example.tsx";
import { carouselTripPhotos } from "../carousel/carousel.example.tsx";
import { centerForm } from "../center/center.example.tsx";
import { chartBar } from "../chart/chart.example.tsx";
import {
    checkboxDefault,
    checkboxChecked,
    checkboxInvalid,
} from "../checkbox/checkbox.example.tsx";
import { clusterTags } from "../cluster/cluster.example.tsx";
import { codeBlockView } from "../code-block/code-block.example.tsx";
import { collapsibleNoteTags } from "../collapsible/collapsible.example.tsx";
import { colorSwatchPickerProjectColor } from "../color-swatch-picker/color-swatch-picker.example.tsx";
import { comboboxMoveNote } from "../combobox/combobox.example.tsx";
import { commandNoteCommands } from "../command/command.example.tsx";
import { contentNote } from "../content/content.example.tsx";
import { contextMenuNoteCardActions } from "../context-menu/context-menu.example.tsx";
import { coverWelcome } from "../cover/cover.example.tsx";
import { dataTableDefault } from "../data-table/data-table.example.tsx";
import { dateFieldBirthday } from "../date-field/date-field.example.tsx";
import { datePickerDueDay } from "../date-picker/date-picker.example.tsx";
import { dialogDefault } from "../dialog/dialog.example.tsx";
import { drawerShareNote } from "../drawer/drawer.example.tsx";
import { dropdownMenuDefault } from "../dropdown-menu/dropdown-menu.example.tsx";
import { emojiPickerReaction } from "../emoji-picker/emoji-picker.example.tsx";
import { emptyNoNotebooks } from "../empty/empty.example.tsx";
import { eventCalendarLaunch } from "../event-calendar/event-calendar.example.tsx";
import { feedActivity } from "../feed/feed.example.tsx";
import { fieldNotebookForm } from "../field/field.example.tsx";
import { glyphPageIcons } from "../glyph/glyph.example.tsx";
import { gridApps } from "../grid/grid.example.tsx";
import { gridListGallery } from "../grid-list/grid-list.example.tsx";
import { hoverCardMention } from "../hover-card/hover-card.example.tsx";
import { iconActions } from "../icon/icon.example.tsx";
import { iconPickerProjectIcon } from "../icon-picker/icon-picker.example.tsx";
import { inputDefault, inputInvalid } from "../input/input.example.tsx";
import { inputGroupNoteSearch } from "../input-group/input-group.example.tsx";
import { otpFieldSignInCode } from "../otp-field/otp-field.example.tsx";
import { itemRecentNotes } from "../item/item.example.tsx";
import { kbdCommandShortcut } from "../kbd/kbd.example.tsx";
import { labelTermsCheckbox } from "../label/label.example.tsx";
import { listBoxNotebook } from "../list-box/list-box.example.tsx";
import { loadMoreNotes } from "../load-more/load-more.example.tsx";
import { menubarEditorMenus } from "../menubar/menubar.example.tsx";
import { navigationMenuSiteNavigation } from "../navigation-menu/navigation-menu.example.tsx";
import { objectChipTask } from "../object/object.example.tsx";
import { paginationSecondPage } from "../pagination/pagination.example.tsx";
import { popoverImageSize } from "../popover/popover.example.tsx";
import { progressUpload } from "../progress/progress.example.tsx";
import { proseArticle } from "../prose/prose.example.tsx";
import { radioGroupListDensity } from "../radio-group/radio-group.example.tsx";
import { resizablePanelGroupNotebookSplit } from "../resizable/resizable.example.tsx";
import { scrollAreaNotebookTags } from "../scroll-area/scroll-area.example.tsx";
import { selectDefault } from "../select/select.example.tsx";
import { separatorFooterLinks } from "../separator/separator.example.tsx";
import { sheetListFilters } from "../sheet/sheet.example.tsx";
import { sidebarNotebooks } from "../sidebar/sidebar.example.tsx";
import { skeletonLoadingRow } from "../skeleton/skeleton.example.tsx";
import { sliderFontSize } from "../slider/slider.example.tsx";
import { sortableChecklist } from "../sortable/sortable.example.tsx";
import { spinnerSavingButton } from "../spinner/spinner.example.tsx";
import { stackNote } from "../stack/stack.example.tsx";
import { switchNotifications } from "../switch/switch.example.tsx";
import { switcherPlans } from "../switcher/switcher.example.tsx";
import { tableInvoices } from "../table/table.example.tsx";
import { tabsDefault } from "../tabs/tabs.example.tsx";
import { textareaComment } from "../textarea/textarea.example.tsx";
import { timeFieldReminder } from "../time-field/time-field.example.tsx";
import { timelinePlan } from "../timeline/timeline.example.tsx";
import { toasterDefault } from "../toast/toast.example.tsx";
import { toggleBoldPressed } from "../toggle/toggle.example.tsx";
import { toggleGroupParagraphAlignment } from "../toggle-group/toggle-group.example.tsx";
import { tooltipArchiveButton } from "../tooltip/tooltip.example.tsx";
import { treeWorkExpanded } from "../tree/tree.example.tsx";
import { virtualListLog } from "../virtual-list/virtual-list.example.tsx";
import { visuallyHiddenIconButton } from "../visually-hidden/visually-hidden.example.tsx";

/** An example a scene shows beside others. */
type Shown = Example<object, JSX.Element>;

/** The styles of a scene: a wall of tiles, each one example under its component's name. */
const styles = style.create({
    garage: {
        display: "flex",
        flexDirection: "column",
        gap: space[4],
        padding: space[5],
    },
    wall: {
        columnWidth: "20rem",
        columnGap: space[4],
    },
    wide: {
        display: "flex",
        flexDirection: "column",
        gap: space[4],
    },
    tile: {
        display: "flex",
        flexDirection: "column",
        gap: space[3],
        minWidth: 0,
        marginBlockEnd: space[4],
        padding: space[4],
        borderWidth: stroke.border,
        borderStyle: "solid",
        borderColor: color.border,
        borderRadius: radius[4],
        breakInside: "avoid",
    },
    wideTile: {
        marginBlockEnd: 0,
    },
    name: {
        margin: 0,
        color: color.mutedForeground,
        fontWeight: weight.medium,
    },
    examples: {
        display: "flex",
        flexDirection: "column",
        gap: space[3],
        minWidth: 0,
    },
    view: {
        position: "relative",
        maxHeight: "32rem",
        overflow: "auto",
    },
});

/** Render an example on the wall, refusing one that leaves its rendering to the gallery. */
function renderOf(example: Shown): JSX.Element {
    if (example.render === undefined) {
        throw new TypeError(`scene example '${example.name}' renders nothing of its own`);
    }

    // render with the example's own properties, none where it sets none
    return example.render(example.properties ?? {});
}

/** Render a wall of examples under the name of the component each shows. */
function Wall(properties: {
    /** The groups, each a component's name and its examples. */
    readonly groups: readonly (readonly [string, readonly Shown[]])[];
    /** Whether each group takes the wall's whole width, as a view does. */
    readonly isWide?: boolean;
}): JSX.Element {
    const isWide = (): boolean => properties.isWide === true;
    const isPacked = (): boolean => !isWide();

    return (
        <div {...style.attrs(text.sm, isPacked() && styles.wall, isWide() && styles.wide)}>
            <For each={properties.groups}>
                {([name, examples]) => (
                    <section {...style.attrs(styles.tile, isWide() && styles.wideTile)}>
                        <h2 {...style.attrs(text.xs, styles.name)}>{name}</h2>
                        <div {...style.attrs(styles.examples, isWide() && styles.view)}>
                            <For each={examples}>{(example) => <div>{renderOf(example)}</div>}</For>
                        </div>
                    </section>
                )}
            </For>
        </div>
    );
}

/** The form controls side by side, at rest and in their states. */
export const sceneControls = defineExample({
    of: Wall,
    name: "controls",
    description: t`The form controls side by side, at rest and in their states.`,
    layout: "padded",
    render: () => (
        <Wall
            groups={[
                ["Button", [buttonDefault, buttonDelete]],
                ["Input", [inputDefault, inputInvalid]],
                ["Input group", [inputGroupNoteSearch]],
                ["Textarea", [textareaComment]],
                ["Select", [selectDefault]],
                ["Combobox", [comboboxMoveNote]],
                ["Date picker", [datePickerDueDay]],
                ["Checkbox", [checkboxDefault, checkboxChecked, checkboxInvalid]],
                ["Radio group", [radioGroupListDensity]],
                ["Switch", [switchNotifications]],
                ["Slider", [sliderFontSize]],
                ["Toggle group", [toggleGroupParagraphAlignment]],
                ["Field", [fieldNotebookForm]],
            ]}
        />
    ),
});

/** The components that take a tile sized to them. */
const COMPONENTS: readonly (readonly [string, readonly Shown[]])[] = [
    ["Accordion", [accordionHelpQuestionsExpanded]],
    ["Alert", [alertFailedSync]],
    ["Alert dialog", [alertDialogDeleteNotebook]],
    ["Autocomplete", [autocompleteFruit]],
    ["Avatar", [avatarDefault, avatarGroupDefault]],
    ["Badge", [badgeNoteState]],
    ["Breadcrumb", [breadcrumbNoteTrail]],
    ["Button", [buttonDefault, buttonDelete]],
    ["Button group", [buttonGroupSplitSave]],
    ["Calendar", [calendarTripDays]],
    ["Card", [cardStoragePlan]],
    ["Carousel", [carouselTripPhotos]],
    ["Center", [centerForm]],
    ["Checkbox", [checkboxChecked]],
    ["Cluster", [clusterTags]],
    ["Code block", [codeBlockView]],
    ["Collapsible", [collapsibleNoteTags]],
    ["Color swatch picker", [colorSwatchPickerProjectColor]],
    ["Combobox", [comboboxMoveNote]],
    ["Command", [commandNoteCommands]],
    ["Context menu", [contextMenuNoteCardActions]],
    ["Date field", [dateFieldBirthday]],
    ["Date picker", [datePickerDueDay]],
    ["Dialog", [dialogDefault]],
    ["Drawer", [drawerShareNote]],
    ["Dropdown menu", [dropdownMenuDefault]],
    ["Emoji picker", [emojiPickerReaction]],
    ["Empty", [emptyNoNotebooks]],
    ["Feed", [feedActivity]],
    ["Field", [fieldNotebookForm]],
    ["Glyph", [glyphPageIcons]],
    ["Grid", [gridApps]],
    ["Hover card", [hoverCardMention]],
    ["Icon", [iconActions]],
    ["Icon picker", [iconPickerProjectIcon]],
    ["Input", [inputDefault]],
    ["Input group", [inputGroupNoteSearch]],
    ["Input otp", [otpFieldSignInCode]],
    ["Item", [itemRecentNotes]],
    ["Kbd", [kbdCommandShortcut]],
    ["Label", [labelTermsCheckbox]],
    ["List box", [listBoxNotebook]],
    ["Load more", [loadMoreNotes]],
    ["Menubar", [menubarEditorMenus]],
    ["Navigation menu", [navigationMenuSiteNavigation]],
    ["Object", [objectChipTask]],
    ["Pagination", [paginationSecondPage]],
    ["Popover", [popoverImageSize]],
    ["Progress", [progressUpload]],
    ["Radio group", [radioGroupListDensity]],
    ["Scroll area", [scrollAreaNotebookTags]],
    ["Select", [selectDefault]],
    ["Separator", [separatorFooterLinks]],
    ["Sheet", [sheetListFilters]],
    ["Skeleton", [skeletonLoadingRow]],
    ["Slider", [sliderFontSize]],
    ["Sortable", [sortableChecklist]],
    ["Spinner", [spinnerSavingButton]],
    ["Stack", [stackNote]],
    ["Switch", [switchNotifications]],
    ["Switcher", [switcherPlans]],
    ["Tabs", [tabsDefault]],
    ["Textarea", [textareaComment]],
    ["Time field", [timeFieldReminder]],
    ["Toast", [toasterDefault]],
    ["Toggle", [toggleBoldPressed]],
    ["Toggle group", [toggleGroupParagraphAlignment]],
    ["Tooltip", [tooltipArchiveButton]],
    ["Tree", [treeWorkExpanded]],
    ["Virtual list", [virtualListLog]],
    ["Visually hidden", [visuallyHiddenIconButton]],
];

/** The views and layouts that take the width of the scene. */
const VIEWS: readonly (readonly [string, readonly Shown[]])[] = [
    ["Board", [boardLaunch]],
    ["Event calendar", [eventCalendarLaunch]],
    ["Timeline", [timelinePlan]],
    ["Data table", [dataTableDefault]],
    ["Chart", [chartBar]],
    ["Table", [tableInvoices]],
    ["Grid list", [gridListGallery]],
    ["Sidebar", [sidebarNotebooks]],
    ["Resizable", [resizablePanelGroupNotebookSplit]],
    ["Cover", [coverWelcome]],
    ["Prose", [proseArticle]],
    ["Content", [contentNote]],
];

/** Every component of the kit in one scene: the components packed in columns, then the views. */
export const sceneGarage = defineExample({
    of: Wall,
    name: "garage",
    description: t`Every component of the kit in one scene: the components packed in columns, then the views.`,
    layout: "fullscreen",
    render: () => (
        <div {...style.attrs(styles.garage)}>
            <Wall groups={COMPONENTS} />
            <Wall groups={VIEWS} isWide />
        </div>
    ),
});
