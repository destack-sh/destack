<script lang="ts" setup>
import EditedThingBanner from "@/components/editors/EditedThingBanner.vue";
import FixedInlineHeader from "@/components/editors/FixedInlineHeader.vue";
import StatementAddArea from "@/components/editors/StatementAddArea.vue";
import StatementInterface from "@/components/editors/StatementInterface.vue";
import TitleBanner from "@/components/editors/TitleBanner.vue";
import { useTimeFromNow } from "@/composables/useNow";
import { graphql, useFragment } from "@/gql";
import { StatementType } from "@/gql/graphql";
import { useActions } from "@/state/actions";
import { useAppearance } from "@/state/appearance";
import { FileEditor, useBenchState, type EditorContext, type FileAction, type StatementHeader } from "@/state/bench";
import { provideFileState, type FileState } from "@/state/file";
import { FileHeaderType, StatementContentType } from "@/state/fragments";
import { useOperations } from "@/state/operations";
import { syncProperty } from "@/utils/sync";
import { ArrowUturnRightIcon, DocumentDuplicateIcon, TrashIcon } from "@heroicons/vue/24/outline";
import { useQuery } from "@vue/apollo-composable";
import { whenever } from "@vueuse/core";
import { title } from "process";
import { computed, nextTick, onBeforeUnmount, ref, watch, type Ref } from "vue";

const props = defineProps<{ editor: EditorContext<FileEditor>; focused: boolean }>();
const emit = defineEmits<{ (e: "close"): void }>();
const bench = useBenchState();
const appearance = useAppearance();
const actions = useActions();
const editor = computed(() => props.editor.editor.value);
const now = useTimeFromNow();
const ops = useOperations();

// file state

const { result: file, loading: fileLoading } = useQuery(
  graphql(/* GraphQL */ `
    query fileContentById($fileId: GlobalID!) {
      file(id: $fileId) {
        id
        projectVersion {
          id
        }
        ...FileHeader
        statements(filters: { isVisible: true }) {
          ...StatementContent
        }
      }
    }
  `),
  () => ({
    fileId: props.editor.editor.value.fileId,
  })
);
const fileHeader = computed(() => useFragment(FileHeaderType, file.value?.file) ?? undefined);
const isDeleted = computed(() => fileHeader.value?.deletedAt != null);
const isOtherVersion = computed(
  () =>
    bench.currentProjectVersionId != null &&
    fileHeader.value != null &&
    fileHeader.value?.projectVersion?.id != bench.currentProjectVersionId
);
const statements = computed(() => {
  return (
    file.value?.file?.statements
      .map((statement) => useFragment(StatementContentType, statement))
      .filter((statement) => statement.deletedAt == null) || []
  );
}, {});
const statementsComponents = ref<Record<string, InstanceType<typeof StatementInterface>>>({});
const fileState: Ref<FileState | null> = computed(() => {
  if (fileHeader.value == null) {
    return null;
  }
  return {
    editor: props.editor.editor.value,
    focused: props.focused,
    file: fileHeader.value as any,
    statementsUnordered: statements.value,
    statementsComponents: statementsComponents.value,
    navigateUp: () => (props.editor.editor.value.blurElement(), titleRef.value?.focus()),
    navigateDown: () => ({}), // no-op?
  } as FileState;
});
const context = provideFileState(fileState);

function registerStatementRef(id: string, component: InstanceType<typeof StatementInterface> | undefined) {
  if (component == null) {
    delete statementsComponents.value[id];
  } else if (statementsComponents.value[id] !== component) {
    statementsComponents.value[id] = component;
  }
}

const name: Ref<string | null> = ref(fileHeader.value?.name ?? null);
const titleRef: Ref<InstanceType<typeof TitleBanner> | null> = ref(null);

syncProperty({
  value: name,
  editing: computed(() => titleRef.value?.editing),
  read: () => (name.value = fileHeader.value?.name ?? null),
  write: () => ops.file.rename(null, fileHeader.value?.id, fileHeader.value?.name ?? "", name.value ?? ""),
});

// sync name/path into editor
watch(name, () => ((editor.value.name = name.value ?? ""), (editor.value.path = name.value ?? "")));

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
  editor.value.editElement(context.value?.positionedStatements[0].statement as StatementHeader);
}

async function insertOrFocusStatementEnd() {
  if (fileHeader.value == null) return;
  bench.focusFile(fileHeader.value);
  // focus last statement if it's a blank
  const lastStatement = context.value?.positionedStatements[context.value.positionedStatements.length - 1];
  if (lastStatement?.statement.type == StatementType.Blank) {
    editor.value.editElement(lastStatement.statement as StatementHeader);
    return;
  } else {
    actions.apply("statement.insertEnd");
  }
}

// left click anywhere clears editor selection
function clearSelectionIfLeftClick(e: MouseEvent) {
  if (e.button == 0 && !e.altKey && !e.shiftKey) {
    editor.value?.clearSelection();
  }
}
document.addEventListener("click", clearSelectionIfLeftClick);
onBeforeUnmount(() => document.removeEventListener("click", clearSelectionIfLeftClick));
// whenever editing -> clears selection
whenever(
  computed(() => editor.value.editing),
  () => editor.value?.clearSelection()
);

function goToContent() {
  titleRef.value?.blur();
  if (context.value?.positionedStatements.length == 0) {
    insertStatementStart();
  } else {
    insertOrFocusStatementStart();
  }
}

// actions
const fileActions: Ref<FileAction[] & { hideInline?: boolean }> = computed(() => [
  {
    label: "Rename",
    icon: DocumentDuplicateIcon,
    action: () => {
      titleRef.value?.focus();
      titleRef.value?.selectAll();
    },
    hideInline: true,
  },
  {
    label: "Duplicate",
    icon: DocumentDuplicateIcon,
    action: () => {
      // not implemented yet
    },
    disabled: true,
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
      if (fileHeader.value == null) {
        return;
      }
      ops.file.softDelete(null, fileHeader.value?.id);
      emit("close");
    },
    disabled: bench.readonly,
  },
]);

// statement add areas (computed absolutely because I'm so tired of flex)
const statementAddAreaPositionX = computed(() => {
  const editorSize = props.editor.size.value;
  if (editorSize.width > appearance.contentWidthWithMargin) {
    const marginX = (editorSize.width - appearance.contentWidth) / 2;
    return {
      width: appearance.contentWidth - 4 + "px",
      marginLeft: marginX - 4 + "px", // no, not sure where the 4 comes from
      marginRight: marginX + "px",
    };
  } else {
    return {
      width: editorSize.width - appearance.contentMarginX * 2 + "px",
      marginLeft: appearance.contentMarginX + "px",
      marginRight: appearance.contentMarginX + "px",
    };
  }
});
</script>

<template>
  <!-- File container -->
  <!-- Only files have a white background :FileBackground -->
  <div class="overflow-x-hidden bg-white">
    <EditedThingBanner :thing="fileHeader" name="file" :is-loading="fileLoading" @restore="restore" />
    <!-- File main content -->
    <!-- (bottom padding is in last StatementAddArea) -->
    <div class="relative flex flex-col bg-white" v-if="fileHeader">
      <!-- Non-clickable invisible overlay if deleted -->
      <div v-if="isDeleted" class="absolute inset-0 z-20 flex justify-center opacity-100" />
      <!-- Editor inline header -->
      <FixedInlineHeader
        :thing="file"
        :actions="fileActions"
        :editing="editor.editing"
        :path="name"
        :subpath="context?.statementsById[editor.activeStatementId ?? '']?.name"
      />
      <!-- Title & inline actions -->
      <TitleBanner
        class="relative mx-auto w-full justify-between pt-14"
        :class="appearance.baseClass"
        :style="{
          'max-width': appearance.contentWidth + appearance.contentMarginX * 2 + 'px',
          paddingLeft: `${appearance.contentMarginX + 6}px`, // + for :StatementPadding
          paddingRight: `${appearance.contentMarginX + 6}px`,
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
        :style="{ 'max-width': appearance.contentWidth + appearance.contentMarginX * 2 + 'px' }"
      >
        <StatementInterface
          :ref="(el: any) => registerStatementRef(positioned.statement.id, el)"
          :file="(fileHeader as any)"
          :statement="(positioned.statement as any)"
          :readonly="isDeleted || isOtherVersion"
          :depth="positioned.depth"
          :ancestors="positioned.ancestors.map((ancestorId) => context?.statementsById[ancestorId])"
          :standalone="false"
          class="w-full"
        />
      </div>
      <!-- Add statement to end -->
      <StatementAddArea
        class="flex-1 pb-72"
        :style="statementAddAreaPositionX"
        position="end"
        @click="bench.readonly || insertOrFocusStatementEnd()"
      />
    </div>
  </div>
</template>
