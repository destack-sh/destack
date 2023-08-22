<script lang="ts" setup>
import PanelStatusNotice from "@/components/editors/PanelStatusNotice.vue";
import PanelHeader from "@/components/editors/PanelHeader.vue";
import StatementComponent from "@/components/editors/Statement.vue";
import StatementAddArea from "@/components/editors/StatementAddArea.vue";
import TitleBanner from "@/components/editors/TitleBanner.vue";
import { graphql, useFragment } from "@/gql";
import { StatementType } from "@/gql/graphql";
import { useActions } from "@/state/actions";
import { useAppearance } from "@/state/appearance";
import { FileEditor, useBenchState, type PanelContext, type FileAction, type StatementHeader } from "@/state/bench";
import { provideFileState, type FileState } from "@/state/file";
import { FileHeaderType, StatementContentType } from "@/state/fragments";
import { useCurrentModule, type Statement, mergeNodePaths } from "@/state/module";
import { useOperations } from "@/state/operations";
import { syncProperty } from "@/utils/sync";
import { ArrowUturnRightIcon, DocumentDuplicateIcon, PencilSquareIcon, TrashIcon } from "@heroicons/vue/24/outline";
import { useQuery } from "@vue/apollo-composable";
import { whenever } from "@vueuse/core";
import { computed, nextTick, onBeforeUnmount, ref, watch, type Ref, watchEffect } from "vue";
import BusySpinnerIcon from "@/components/basic/BusySpinnerIcon.vue";
import { newFileId } from "@/state/operations/file";
import { useCurrentClients } from "@/state/client";
import UserAvatar from "@/components/basic/UserAvatar.vue";

const props = defineProps<{ panel: PanelContext<FileEditor>; focused: boolean }>();
const emit = defineEmits<{ (e: "close"): void }>();
const bench = useBenchState();
const module = useCurrentModule();
const appearance = useAppearance();
const actions = useActions();
const panel = computed(() => props.panel.panel.value);
const ops = useOperations();

// file state

const { result: file, loading: fileLoading } = useQuery(
  graphql(/* GraphQL */ `
    query fileContentById($fileId: GlobalID!) {
      file(id: $fileId) {
        # :fileContentById
        id
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
    fileId: props.panel.panel.value.fileId,
  })
);
const fileHeader = computed(() => useFragment(FileHeaderType, file.value?.file) ?? undefined);
const isDeleted = computed(() => fileHeader.value?.deletedAt != null);
const isOtherVersion = computed(
  () =>
    bench.projectVersionId != null &&
    fileHeader.value != null &&
    fileHeader.value?.projectVersion?.id != bench.projectVersionId
);
const name: Ref<string> = ref(fileHeader.value?.name ?? "");
const titleRef: Ref<InstanceType<typeof TitleBanner> | null> = ref(null);

syncProperty({
  value: name,
  editing: computed(() => titleRef.value?.editing),
  read: () => (name.value = fileHeader.value?.name ?? ""),
  write: () => ops.file.rename(null, fileHeader.value?.id, fileHeader.value?.name ?? "", name.value ?? ""),
});

// sync name/path into editor
watch([name, fileHeader], () => {
  if (fileHeader.value == null || module.idx.value == null) return;
  panel.value.updatePath({ ...fileHeader.value, name: name.value }, module.idx.value);
});

const statements = computed(() => {
  return (
    file.value?.file?.statements
      .map((statement) => useFragment(StatementContentType, statement))
      .filter((statement) => statement.deletedAt == null) || []
  );
}, {});
const statementsComponents = ref<Record<string, InstanceType<typeof StatementComponent>>>({});
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
const context = provideFileState(fileState);

const statementsLoaded = ref(false); // first time that all statements are loaded (subsequent loads are ignored)
watchEffect(() => {
  if (statementsLoaded.value || fileLoading.value) return;
  if (Object.keys(statementsComponents.value).length == statements.value.length) {
    for (const statement of statements.value) {
      if (statementsComponents.value[statement.id].loading) {
        return;
      }
    }
    statementsLoaded.value = true;
    panel.value.stopEditingElement(); // reset editing element on load
    if (panel.value.focused && panel.value.activeStatementId != null) {
      // focus active statement
      statementsComponents.value[panel.value.activeStatementId]?.focus();
      // scroll into view
      nextTick(() => {
        statementsComponents.value[panel.value.activeStatementId as string]?.$el?.parentNode?.scrollIntoView({
          behavior: "instant",
          block: "center",
          inline: "center",
        });
      });
    }
  }
});

// navigation

function focusTitle() {
  titleRef.value?.focus();
  panel.value.activeStatementId = undefined;
}

function registerStatementRef(id: string, component: InstanceType<typeof StatementComponent> | undefined) {
  if (component == null) {
    delete statementsComponents.value[id];
  } else if (statementsComponents.value[id] !== component) {
    statementsComponents.value[id] = component;
  }
}

// auto-focus name once loaded and if contents are empty
watch(
  () => [name.value, props.focused],
  () => {
    if (name.value == null) {
      return;
    }
    if (props.focused && statements.value.length == 0 && name.value == "") {
      titleRef.value?.focus();
      nextTick(() => titleRef.value?.focus()); // required to focus if just loaded
    }
  }
);

function restore() {
  ops.file.restore(null, fileHeader.value?.id);
}

// navigation

async function insertStatementStart() {
  if (fileHeader.value == null) return;
  bench.focusFile(fileHeader.value);
  actions.apply("statement.insertStart");
}

async function insertOrFocusStatementStart() {
  if (fileHeader.value == null) return;
  bench.focusFile(fileHeader.value);
  if (context.value?.positionedStatements.length == 0) {
    insertStatementStart();
  } else {
    focusStatementStart();
  }
}

function focusStatementStart() {
  panel.value.editElement(context.value?.positionedStatements[0].statement as StatementHeader);
}

async function insertOrFocusStatementEnd() {
  if (fileHeader.value == null) return;
  bench.focusFile(fileHeader.value);
  // focus last statement if it's a blank
  const lastStatement = context.value?.positionedStatements[context.value.positionedStatements.length - 1];
  if (lastStatement?.statement.type == StatementType.Blank) {
    panel.value.editElement(lastStatement.statement as StatementHeader);
    return;
  } else {
    actions.apply("statement.insertEnd");
  }
}

// left click anywhere clears editor selection
function clearSelectionIfLeftClick(e: MouseEvent) {
  if (e.button == 0 && !e.altKey && !e.shiftKey) {
    panel.value?.clearSelection();
  }
}
document.addEventListener("click", clearSelectionIfLeftClick);
onBeforeUnmount(() => document.removeEventListener("click", clearSelectionIfLeftClick));
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
  },
  {
    label: "Duplicate",
    icon: DocumentDuplicateIcon,
    active: duplicating.value,
    disabled: duplicating.value,
    action: async () => {
      if (fileHeader.value == null) return;
      duplicating.value = true;
      const targetId = newFileId();
      try {
        const ret = await ops.file.paste(null, fileHeader.value?.id, targetId, module.id.value, null);
        if (ret?.data?.pasteFile.__typename == "File") {
          bench.focusFile({ __typename: "File", id: targetId, name: fileHeader.value?.name ?? "" });
        }
      } finally {
        duplicating.value = false;
      }
    },
  },
  {
    label: "Move",
    icon: ArrowUturnRightIcon,
    action: () => {
      // not implemented yet
    },
    disabled: true,
  },
  {
    label: "Delete",
    icon: TrashIcon,
    action: () => {
      if (fileHeader.value == null) return;
      ops.file.softDelete(null, fileHeader.value?.id);
      emit("close");
    },
    disabled: bench.readonly,
    hideInline: true,
  },
]);

const filePath = computed(() => module.nodePathOf({ id: panel.value.fileId }));
const focusPath = computed(() =>
  panel.value.activeStatementId == null ? null : module.nodePathOf({ id: panel.value.activeStatementId })
);
const completePath = computed(() => {
  if (filePath.value == null) return null;
  if (focusPath.value == null) return filePath.value;
  return mergeNodePaths(filePath.value, focusPath.value);
});

// statement add areas (computed absolutely because I'm so tired of flex)
const statementAddAreaPositionX = computed(() => {
  const editorSize = props.panel.size.value;
  if (editorSize.width > panel.value.contentWidthWithMargin) {
    const marginX = (editorSize.width - panel.value.contentWidth) / 2;
    return {
      width: panel.value.contentWidth - 4 + "px",
      marginLeft: marginX - 4 + "px", // no, not sure where the 4 comes from
      marginRight: marginX + "px",
    };
  } else {
    return {
      width: editorSize.width - panel.value.contentMarginX * 2 + "px",
      marginLeft: panel.value.contentMarginX + "px",
      marginRight: panel.value.contentMarginX + "px",
    };
  }
});

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
    right: Math.round((statement.bounding.right.value + panel.scroll.value.x - panel.pos.value.left) * 100) / 100,
    top: Math.round((statement.bounding.top.value + panel.scroll.value.y - panel.pos.value.top) * 100) / 100,
  };
}
</script>

<template>
  <!-- File container -->
  <!-- Only files have a white background :FileBackground -->
  <div class="relative overflow-x-hidden bg-white">
    <PanelHeader
      :thing="file"
      :actions="fileActions"
      :editing="panel.editing"
      :path="completePath ?? []"
      :self="completePath?.findIndex((n) => n.id == panel.fileId) ?? -1"
      :subpath="context?.statementsById[panel.activeStatementId ?? '']?.name"
      @focus="panel.activeStatementId = $event.id"
    />
    <!-- Loading / status -->
    <div
      v-if="fileLoading || !statementsLoaded"
      class="flex h-full w-full flex-col items-center justify-center"
      :style="{
        width: props.panel.size.value?.width + 'px',
        height: props.panel.size.value?.height + 'px',
      }"
    >
      <BusySpinnerIcon class="mx-auto h-8 w-8 animate-spin text-gray-700" />
    </div>
    <PanelStatusNotice :thing="fileHeader" name="file" :is-loading="fileLoading" @restore="restore" />
    <!-- File main content -->
    <!-- (bottom padding is in last StatementAddArea) -->
    <div class="relative flex flex-col bg-white" v-if="!fileLoading && fileHeader" v-show="statementsLoaded">
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
        @enter="goToContent"
        @navigate-down="focusStatementStart"
        :readonly="bench.readonly || isDeleted || isOtherVersion"
        :actions="fileActions.filter((f) => !f.hideInline)"
        :thing="fileHeader"
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
        <StatementComponent
          :ref="(el: any) => registerStatementRef(positioned.statement.id, el)"
          :file="(fileHeader as any)"
          :statement="(positioned.statement as any)"
          :readonly="isDeleted || isOtherVersion"
          :depth="positioned.depth"
          :ancestors="positioned.ancestors.map((ancestorId) => (context?.statementsById[ancestorId] as Statement))"
          :standalone="false"
          :shown="statementsLoaded"
          class="w-full"
        />
      </div>
      <!-- Add statement to end -->
      <StatementAddArea
        class="flex-1 pb-96"
        :style="statementAddAreaPositionX"
        position="end"
        @click="bench.readonly || insertOrFocusStatementEnd()"
      />
    </div>
    <!-- TODO @UX: client indicators next to statements -->
    <!-- (these move smoothly as the other client moves but instantly as we scroll...) -->
    <div
      v-for="client in localClients"
      :key="client.id"
      class="absolute transition-all duration-150"
      :style="{
        left: getStatementBounding(client.statementId).right + 32 + 'px',
        top: getStatementBounding(client.statementId).top + 4 + 'px',
      }"
    >
      <UserAvatar :client-id="client.id" :user="client.user" class="h-4 w-4" />
    </div>
  </div>
</template>
