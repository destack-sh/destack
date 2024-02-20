<script lang="ts" setup>
import PanelStatusNotice from "@/components/panels/PanelStatusNotice.vue";
import PanelHeader from "@/components/panels/PanelHeader.vue";
import InlineStatement from "@/components/statements/InlineStatement.vue";
import StatementAddArea from "@/components/panels/StatementAddArea.vue";
import TitleBanner from "@/components/panels/TitleBanner.vue";
import { graphql, useFragment } from "@/gql";
import { StatementType } from "@/gql/graphql";
import { useAppearance } from "@/state/appearance";
import {
  EditFilePanel,
  useBenchState,
  type PanelContext,
  type FileAction,
  type NavElement,
  type StatementHeader,
} from "@/state/bench";
import { provideFileState, type FileState, type NavigationContext } from "@/state/file";
import { FileHeaderType, StatementContentType } from "@/state/fragments";
import { useCurrentModule, type Statement, mergeNodePaths, newNodeIdentity, getNodeIdFromCk } from "@/state/module";
import { useOperations } from "@/state/operations";
import { syncProperty } from "@/utils/sync";
import { ArrowUturnRightIcon, DocumentDuplicateIcon, PencilSquareIcon, TrashIcon } from "@heroicons/vue/24/outline";
import { useQuery } from "@vue/apollo-composable";
import { useMouse, useWindowSize, whenever } from "@vueuse/core";
import { computed, nextTick, onBeforeUnmount, ref, watch, type Ref, watchEffect } from "vue";
import BusySpinnerIcon from "@/components/basic/BusySpinnerIcon.vue";
import { useCurrentClients } from "@/state/client";
import UserAvatar from "@/components/basic/UserAvatar.vue";
import { useNotifications } from "@/state/notifications";
import { INTEGER_ZERO, generateKeyBetween } from "@/utils/fractional";
import TerminalPopover from "@/components/interfaces/TerminalPopover.vue";

const props = defineProps<{ panel: PanelContext<EditFilePanel>; focused: boolean }>();
const emit = defineEmits<{ (e: "close"): void }>();
const bench = useBenchState();
const module = useCurrentModule();
const appearance = useAppearance();
const panel = computed(() => props.panel.panel.value);
const scroll = computed(() => props.panel.scroll.value);
const ops = useOperations();
const window = useWindowSize();

// file state
const isPendingCreate = computed(() => ops.state.hasInflightLike({ types: ["file.create"] }));

const {
  result: file,
  loading: fileLoading,
  error: fileError,
} = useQuery(
  graphql(/* GraphQL */ `
    query fileContentById($fileId: GlobalID!) {
      file(id: $fileId) {
        # :fileContentById
        id
        ck
        ...FileHeader
        statements(filters: { isVisible: true }) {
          ...StatementContent
        }
        issues {
          ...IssueContent
        }
      }
    }
  `),
  () => ({
    fileId: getNodeIdFromCk(bench.projectVersionId as string, panel.value.fileCk, "File"),
  }),
  {
    enabled: computed(() => !isPendingCreate.value), // cheeky hack to avoid race condition if file is slow in backend
  }
);
const fileHeader = computed(() => useFragment(FileHeaderType, file.value?.file) ?? undefined);
const isLoading = computed(() => fileLoading.value || (isPendingCreate.value && file.value == null));
const isDeleted = computed(() => fileHeader.value?.deletedAt != null);
const isOtherVersion = computed(
  () =>
    !isPendingCreate.value &&
    bench.projectVersionId != null &&
    fileHeader.value != null &&
    fileHeader.value?.projectVersion?.id != bench.projectVersionId
);
const effectiveReadonly = computed(() => bench.readonly || isDeleted.value || isOtherVersion.value);
const name: Ref<string> = ref(fileHeader.value?.name ?? "");
const titleRef: Ref<InstanceType<typeof TitleBanner> | null> = ref(null);

const nameSync = syncProperty({
  read: () => (name.value = fileHeader.value?.name ?? ""),
  write: () => {
    ops.file.rename(null, fileHeader.value?.id, fileHeader.value?.name ?? "", name.value ?? "");
  },
  enabled: computed(() => fileHeader.value != null && !isDeleted.value),
});

// sync name/path into editor
watch(
  () => [name.value, fileHeader.value?.name, module.idx.value],
  () => {
    if (fileHeader.value == null || module.idx.value == null) return;
    panel.value.updatePath({ ...fileHeader.value, name: name.value }, module.idx.value);
  }
);

const statements = computed(() => {
  return (
    file.value?.file?.statements
      .map((statement) => useFragment(StatementContentType, statement))
      .filter((statement) => statement.deletedAt == null) || []
  );
}, {});
const statementsComponents = ref<Record<string, InstanceType<typeof InlineStatement>>>({});
const fileState: Ref<FileState | null> = computed(() => {
  if (fileHeader.value == null) {
    return null;
  }
  return {
    panel: props.panel.panel.value,
    focused: props.focused,
    editing: panel.value.editing || titleRef.value?.editing,
    file: fileHeader.value as any,
    statementsUnordered: statements.value as any,
    statementsComponents: statementsComponents.value,
    navigateUp: focusTitle,
    navigateDown: () => ({}), // no-op?
  } as FileState;
});
const context: Ref<NavigationContext | null> = provideFileState(fileState);

// navigation

function focusTitle() {
  titleRef.value?.focus();
  panel.value.activeStatementCk = undefined;
}

function registerStatementRef(id: string, component: InstanceType<typeof InlineStatement> | undefined) {
  if (component == null) {
    delete statementsComponents.value[id];
  } else if (statementsComponents.value[id] !== component) {
    statementsComponents.value[id] = component;
  }
}

// auto-focus on load
const hasFocused = ref(false);
watch(
  () => [statements.value, props.focused],
  () => {
    if (!props.focused || file.value == null) {
      hasFocused.value = false;
      return;
    }
    if (hasFocused.value) return;
    // reset active statement if it's no longer visible
    if (panel.value.activeStatementCk != null && !statements.value.some((s) => s.ck == panel.value.activeStatementCk)) {
      panel.value.blurElement();
    }
    // focus title or first statement
    if (statements.value.length == 0) {
      titleRef.value?.focus("last");
      nextTick(() => titleRef.value?.focus("last")); // required to focus if just loaded
    } else if (panel.value.activeStatementCk == null) {
      panel.value.focusElement(statements.value[0]);
    }
    hasFocused.value = true;
  },
  { immediate: true }
);

function restore() {
  ops.file.restore(null, fileHeader.value?.id);
}

// navigation

async function insertStatementStart() {
  if (fileHeader.value == null) return;
  bench.focusFile(fileHeader.value);
  const firstRootOk = context.value?.statementsByParentId[fileHeader.value.id]?.[0]?.orderKey ?? INTEGER_ZERO;
  const newStatement = { __typename: "Statement", ...newNodeIdentity(bench.projectVersionId as string, "Statement") };
  const orderKey = generateKeyBetween(null, firstRootOk);
  ops.statement.create(null, newStatement.id, newStatement.ck, fileHeader.value.id, null, orderKey);
  nextTick(() => context.value?.statementsComponents[newStatement.id]?.focus());
}

async function insertStatementEnd() {
  if (fileHeader.value == null) return;
  bench.focusFile(fileHeader.value);
  const roots = context.value?.statementsByParentId[fileHeader.value.id];
  const lastRootOk = roots?.[roots?.length - 1]?.orderKey ?? INTEGER_ZERO;
  const newStatement = { __typename: "Statement", ...newNodeIdentity(bench.projectVersionId as string, "Statement") };
  const orderKey = generateKeyBetween(lastRootOk, null);
  ops.statement.create(null, newStatement.id, newStatement.ck, fileHeader.value.id, null, orderKey);
  nextTick(() => context.value?.statementsComponents[newStatement.id]?.focus());
}

async function insertOrFocusStatementStart() {
  if (fileHeader.value == null) return;
  bench.focusFile(fileHeader.value);
  if (context.value?.positionedStatements.length == 0) {
    insertStatementStart();
  } else {
    panel.value.editElement(context.value?.positionedStatements[0].statement as NavElement);
  }
}

async function insertOrFocusStatementEnd() {
  if (fileHeader.value == null) return;
  bench.focusFile(fileHeader.value);
  // focus last statement if it's a blank
  const lastStatement = context.value?.positionedStatements[context.value.positionedStatements.length - 1];
  if (lastStatement?.statement.type == StatementType.Blank) {
    panel.value.editElement(lastStatement.statement as NavElement);
    return;
  } else {
    insertStatementEnd();
  }
}

// whenever editing -> clears selection
whenever(
  computed(() => panel.value.editing),
  () => panel.value?.clearSelection()
);

function goToContent() {
  titleRef.value?.blur();
  insertStatementStart();
}

// actions
const notifications = useNotifications();
const duplicating = ref(false);
const fileActions: Ref<FileAction[] & { hideInline?: boolean }> = computed(() => [
  {
    label: "Rename",
    icon: PencilSquareIcon,
    action: () => {
      titleRef.value?.focus();
      titleRef.value?.selectAll();
    },
    hideInline: true,
    disabled: effectiveReadonly.value,
  },
  {
    label: "Duplicate",
    icon: DocumentDuplicateIcon,
    active: duplicating.value,
    disabled: true,
    action: async () => {
      // TODO @Feature: re-implement duplicate with new edit system :BE-114
    },
  },
  // we don't have a proper 'duplicate to' action yet (if you're read-only, or just generally)
  // so we provide a generic copy for now that you can paste yourself
  {
    label: "Copy",
    icon: DocumentDuplicateIcon,
    hideInline: !effectiveReadonly.value,
    action: async () => {
      if (fileHeader.value == null) return;
      notifications.show({
        kind: "success",
        type: "file.copied",
        message: "Copied file to clipboard",
        description: "Paste it anywhere into your own file.",
      });
      context.value?.copy(fileState.value?.statementsUnordered as StatementHeader[]);
    },
  },
  {
    label: "Move",
    icon: ArrowUturnRightIcon,
    action: () => {
      // not implemented yet
    },
    disabled: true && effectiveReadonly.value,
  },
  {
    label: "Delete",
    icon: TrashIcon,
    action: () => {
      if (fileHeader.value == null) return;
      ops.file.softDelete(null, fileHeader.value?.id);
      emit("close");
    },
    disabled: effectiveReadonly.value,
    hideInline: true,
  },
]);

const filePath = computed(() => module.nodePathOf(panel.value.fileCk));
const focusPath = computed(() =>
  panel.value.activeStatementCk == null ? null : module.nodePathOf(panel.value.activeStatementCk)
);
const completePath = computed(() => {
  if (filePath.value == null) return null;
  if (focusPath.value == null) return filePath.value;
  return mergeNodePaths(filePath.value, focusPath.value);
});

// statement add areas (computed absolutely because I'm so tired of flex)
const statementAddAreaPositionX = computed(() => {
  const panelsize = props.panel.size.value;
  if (panelsize.width > panel.value.contentWidthWithMargin) {
    const marginX = (panelsize.width - panel.value.contentWidth) / 2;
    return {
      width: panel.value.contentWidth - 4 + "px",
      marginLeft: marginX - 4 + "px", // no, not sure where the 4 comes from
      marginRight: marginX + "px",
    };
  } else {
    return {
      width: panelsize.width - panel.value.contentMarginX * 2 + "px",
      marginLeft: panel.value.contentMarginX + "px",
      marginRight: panel.value.contentMarginX + "px",
    };
  }
});

// drag select area
// TODO @UX: drag should also work for sub-statement elements like fields/records/etc.
// and of course this should also be refactored out into a composable
const mainContentRef = ref<HTMLElement | null>(null);
const statementAddAreaEndRef = ref<InstanceType<typeof StatementAddArea> | null>(null);
const { x: mouseX, y: mouseY } = useMouse();
const dragSelectStart = ref<{ x: number; y: number } | null>(null);

function isMouseInsideStatements(e: MouseEvent): boolean {
  if (e.target == mainContentRef.value || e.target == statementAddAreaEndRef.value?.$el) return false;

  // find next element with 'group/statement' class up (if exists)
  let el = e.target as HTMLElement;
  const path = [el];
  while (el != null && !el.classList.contains("group/statement")) {
    el = el.parentElement as HTMLElement;
    path.push(el);
  }
  if (el == null) return false;

  // only start drag select if not within statement content (i.e. inside left/right margins)
  const insideContent =
    e.clientX > el.getBoundingClientRect().left + appearance.contentMarginX &&
    e.clientX < el.getBoundingClientRect().right - appearance.contentMarginX;
  return insideContent || path.some((el) => el.classList.contains("absolute") || el.classList.contains("fixed"));
}

function startDragSelectMaybe(e: MouseEvent) {
  if (e.button != 0 || e.altKey || e.shiftKey) return;
  // must be clicking outside statement components or in their margin
  if (isMouseInsideStatements(e)) return;
  // clear selection on first click, blur active statement on second
  if (panel.value.hasSelection) {
    panel.value.clearSelection();
    panel.value.stopEditingElement();
  } else {
    panel.value.blurElement();
  }
  dragSelectStart.value = { x: e.clientX + scroll.value.x, y: e.clientY + scroll.value.y };
  e.stopPropagation();
}

function updateDragSelectMaybe(e: { clientX: number; clientY: number }) {
  if (dragSelectStart.value == null) return;
  // select all statements components intersecting with the drag select area
  const selectedElementIds: string[] = [];
  for (const statement of context.value?.positionedStatements ?? []) {
    const bounding = statementsComponents.value[statement.statement.id]?.bounding;
    if (bounding == null) continue;
    if (
      bounding.left.value < Math.max(e.clientX, dragSelectStart.value.x - scroll.value.x) &&
      bounding.right.value > Math.min(e.clientX, dragSelectStart.value.x - scroll.value.x) &&
      bounding.top.value < Math.max(e.clientY, dragSelectStart.value.y - scroll.value.y) &&
      bounding.bottom.value > Math.min(e.clientY, dragSelectStart.value.y - scroll.value.y)
    ) {
      selectedElementIds.push(statement.statement.id);
    }
  }
  // update selected if changed
  if (
    selectedElementIds.some((s, i) => panel.value.selectedElementIds[i] != s) ||
    selectedElementIds.length != panel.value.selectedElementIds.length
  ) {
    panel.value.selectedElementIds = selectedElementIds;
    panel.value.selectedElementType = "Statement";
  }

  // smooth scroll up/down if near top/bottom
  const scrollMargin = 100;
  if (e.clientY - appearance.panelHeaderHeight < scrollMargin) {
    props.panel.container.value?.scrollTo({ left: scroll.value.x, top: scroll.value.y - 10 });
  } else if (e.clientY > window.height.value - scrollMargin) {
    props.panel.container.value?.scrollTo({ left: scroll.value.y, top: scroll.value.y + 10 });
  }
}

const dragSelectArea: Ref<{ left: number; top: number; width: number; height: number } | null> = computed(() => {
  if (dragSelectStart.value == null) return null;
  let top = Math.min(mouseY.value, dragSelectStart.value.y - scroll.value.y);
  let height = Math.abs(mouseY.value - (dragSelectStart.value.y - scroll.value.y));
  let left = Math.min(mouseX.value, dragSelectStart.value.x - scroll.value.x);
  let width = Math.abs(mouseX.value - (dragSelectStart.value.x - scroll.value.x));

  // clip to panel
  const panelPos = { top: props.panel.pos.value.top + appearance.panelHeaderHeight, left: props.panel.pos.value.left };
  const panelSize = props.panel.size.value;
  if (top < panelPos.top) {
    height -= panelPos.top - top;
    top = panelPos.top;
  }
  if (top + height > panelPos.top + panelSize.height) {
    height -= top + height - (panelPos.top + panelSize.height);
  }
  if (left < panelPos.left) {
    width -= panelPos.left - left;
    left = panelPos.left;
  }
  if (left + width > panelPos.left + panelSize.width) {
    width -= left + width - (panelPos.left + panelSize.width);
  }

  return { left, top, width, height };
});

const updateDragInterval = setInterval(() => {
  if (dragSelectStart.value == null) return;
  updateDragSelectMaybe({ clientX: mouseX.value, clientY: mouseY.value });
}, 10);
onBeforeUnmount(() => clearInterval(updateDragInterval));

function stopDragSelect() {
  dragSelectStart.value = null;
}

// assist
const terminalPopoverRef: Ref<InstanceType<typeof TerminalPopover> | null> = ref(null);
function launchAssist(text: string, selection: StatementHeader[] | undefined, from: Statement) {
  terminalPopoverRef.value?.open(text, selection, from);
}

// other clients
const clients = useCurrentClients();
const localClients = computed(() =>
  clients.activeClientsWithoutSelf.value.filter((c) => c.fileId == fileHeader.value?.id && c.statementId != null)
);

function getStatementBounding(statementId: string): { top: number; right: number } {
  const statement = statementsComponents.value[statementId];
  if (statement == null) return { top: -100, right: -100 };
  const panel = props.panel;
  return {
    right: Math.round((statement.bounding.right.value + scroll.value.x - panel.pos.value.left) * 100) / 100,
    top: Math.round((statement.bounding.top.value + scroll.value.y - panel.pos.value.top) * 100) / 100,
  };
}

defineExpose({
  statementsComponents,
});
</script>

<template>
  <!-- File container -->
  <!-- Only source files have a white background :PanelBackground -->
  <div class="relative overflow-x-hidden bg-white">
    <PanelHeader
      class="border-b border-orange-900/[12%] bg-white"
      :thing="file"
      :actions="fileActions"
      :editing="panel.editing"
      :path="completePath ?? []"
      :self="completePath?.findIndex((n) => n.id == panel.fileCk) ?? -1"
      :subpath="context?.statementsById[panel.activeStatementCk ?? '']?.name"
      @focus="panel.activeStatementCk = $event.id"
    />
    <!-- Loading / status -->
    <div
      v-if="isLoading"
      class="flex h-full w-full flex-col items-center justify-center"
      :style="{
        width: props.panel.size.value?.width + 'px',
        height: props.panel.size.value?.height + 'px',
      }"
    >
      <BusySpinnerIcon class="mx-auto h-8 w-8 animate-spin text-gray-700" />
    </div>
    <PanelStatusNotice :thing="fileHeader" name="file" :loading="isLoading" :error="fileError" @restore="restore" />
    <!-- File main content -->
    <!-- (bottom padding is in last StatementAddArea) -->
    <div
      ref="mainContentRef"
      class="relative flex flex-col bg-white"
      v-if="!isLoading && fileHeader"
      @mousedown="startDragSelectMaybe"
      @mousemove="updateDragSelectMaybe"
      @mouseup="stopDragSelect"
      @keydown.escape="stopDragSelect"
      :class="[dragSelectStart ? 'select-none' : '']"
    >
      <!-- Drag select area (clipped to panel boundary) -->
      <div
        v-if="dragSelectArea"
        class="fixed z-50 bg-orange-200 opacity-30"
        :style="{
          left: dragSelectArea.left + 'px',
          top: dragSelectArea.top + 'px',
          width: dragSelectArea.width + 'px',
          height: dragSelectArea.height + 'px',
        }"
      />
      <!-- Title & inline actions -->
      <TitleBanner
        ref="titleRef"
        class="relative mx-auto w-full justify-between pt-14"
        :class="appearance.baseClass"
        :style="{
          'max-width': panel.contentWidth + panel.contentMarginX * 2 + 'px',
          paddingLeft: `${panel.contentMarginX + 8}px`, // + for :StatementPadding
          paddingRight: `${panel.contentMarginX + 8}px`,
        }"
        v-model="name"
        @update:model-value="nameSync.onLocalWrite"
        @enter="goToContent"
        @navigate-down="insertOrFocusStatementStart"
        :readonly="bench.readonly || isDeleted || isOtherVersion"
        :actions="fileActions.filter((f) => !f.hideInline)"
        :thing="fileHeader"
        :fatActions="effectiveReadonly"
      />
      <!-- Add statement to start -->
      <StatementAddArea
        class="mx-auto"
        :style="statementAddAreaPositionX"
        position="start"
        @click="bench.readonly || insertOrFocusStatementStart()"
        v-if="statements?.length > 0"
      />
      <!-- File's statements -->
      <div
        v-for="positioned in context?.positionedStatements"
        :key="positioned.statement.id"
        class="mx-auto w-full"
        :style="{ 'max-width': panel.contentWidth + panel.contentMarginX * 2 + 'px' }"
      >
        <InlineStatement
          :ref="(el: any) => registerStatementRef(positioned.statement.id, el)"
          :key="positioned.statement.id"
          :file="(fileHeader as any)"
          :statement="(positioned.statement as any)"
          :readonly="isDeleted || isOtherVersion || bench.readonly"
          :depth="positioned.depth"
          :rendered-depth="positioned.renderedDepth"
          :ancestors="positioned.ancestors.map((ancestorId) => (context?.statementsById[ancestorId] as Statement))"
          :standalone="false"
          :is-group-start="positioned.isGroupStart"
          :is-group-middle="positioned.isGroupMiddle"
          :is-group-end="positioned.isGroupEnd"
          @launch-assist="(text, selection) => launchAssist(text, selection, positioned.statement as Statement)"
          class="w-full"
        />
      </div>
      <!-- Add statement to end -->
      <StatementAddArea
        ref="statementAddAreaEndRef"
        class="flex-1 pb-96"
        :style="statementAddAreaPositionX"
        position="end"
        @click="bench.readonly || insertOrFocusStatementEnd()"
      />
    </div>
    <!-- Terminal popover -->
    <TerminalPopover
      v-if="fileHeader != null"
      ref="terminalPopoverRef"
      class="fixed z-50"
      :file-ck="fileHeader?.ck"
      :current-selection="context?.getSelectedRoots()"
      :style="{
        right: window.width.value - (props.panel.pos.value?.left + props.panel.size.value?.width) + 24 + 'px',
        bottom: window.height.value - (props.panel.pos.value?.top + props.panel.size.value?.height) + 32 + 'px',
      }"
    />

    <!-- Client indicators next to statements -->
    <!-- TODO @UX: cleanup client presence indicators
      (they're jumpy, ugly, move smoothly as the other client moves but instantly as we scroll...) -->
    <div
      v-for="client in localClients"
      :key="client.id"
      class="absolute transition-all duration-150"
      :style="{
        left: getStatementBounding(client.statementId).right + 32 + 'px',
        top: getStatementBounding(client.statementId).top + 4 + 'px',
      }"
    >
      <UserAvatar :client-id="client.id" :user="client.user" class="h-5 w-5 text-xs" />
    </div>
  </div>
</template>
