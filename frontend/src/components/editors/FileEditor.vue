<script lang="ts" setup>
import EditableSpan from "@/components/basic/EditableSpan.vue";
import StatementAddArea from "@/components/editors/StatementAddArea.vue";
import StatementInterface from "@/components/editors/StatementInterface.vue";
import ClientsPopover from "@/components/basic/ClientsPopover.vue";
import { provideFileState, type FileState } from "@/state/file";
import { useTimeFromNow } from "@/composables/useNow";
import { graphql, useFragment } from "@/gql";
import { StatementType } from "@/gql/graphql";
import { useActions } from "@/state/actions";
import { useAuth } from "@/state/auth";
import { FileEditor, useBenchState, type EditorContext, type FileAction, type StatementHeader } from "@/state/editor";
import { FileHeaderType, StatementContentType } from "@/state/fragments";
import { useOperations } from "@/state/operations";
import { syncProperty } from "@/utils/sync";
import {
  ArrowUturnRightIcon,
  DocumentDuplicateIcon,
  TrashIcon,
  CodeBracketIcon,
  ChevronRightIcon,
} from "@heroicons/vue/24/outline";
import { useQuery } from "@vue/apollo-composable";
import { computed, nextTick, onBeforeUnmount, ref, watch, type Ref } from "vue";
import { useAppearance } from "@/state/appearance";
import ActionPopover from "@/components/basic/ActionPopover.vue";
import { whenever } from "@vueuse/core";
import FadeTransition from "@/components/basic/FadeTransition.vue";

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
    navigateUp: () => (props.editor.editor.value.blurElement(), nameRef.value?.focus()),
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
const nameRef: Ref<InstanceType<typeof EditableSpan> | null> = ref(null);

syncProperty({
  value: name,
  editing: computed(() => nameRef.value?.focused),
  read: () => (name.value = fileHeader.value?.name ?? null),
  write: () => ops.file.rename(null, fileHeader.value?.id, fileHeader.value?.name ?? "", name.value ?? ""),
});

// sync name/path into editor
watch(name, () => (editor.value.path = name.value ?? ""));

// auto-focus name once loaded and if contents are empty
watch(
  () => [name.value, props.focused],
  () => {
    if (name.value == null) {
      return;
    }
    if (props.focused && statements.value.length == 0 && name.value == "") {
      nameRef.value?.focus();
      nextTick(() => nameRef.value?.focus()); // required to focus if just loaded
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
    editor.value.editElement(context.value?.positionedStatements[0].statement as StatementHeader);
  }
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
  nameRef.value?.blur();
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
      nameRef.value?.focus();
      nameRef.value?.selectAll();
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
const auth = useAuth();
</script>

<template>
  <!-- File container -->
  <!-- Only files have a white background :FileBackground -->
  <div class="overflow-x-hidden bg-white">
    <!-- Deleted file status and restore -->
    <div v-if="isDeleted && fileHeader" class="sticky top-0 z-10 -mr-12 w-full bg-red-600 py-2">
      <div class="mx-auto flex flex-row items-center justify-center gap-2" :style="appearance.contentWidthAsMaxWidth">
        <div class="text-sm font-bold text-white">
          This file is in trash (was deleted {{ now.getTimeFromNowLongString(fileHeader.deletedAt) }}).
        </div>
        <button
          class="text-sm text-white underline decoration-dashed underline-offset-4 hover:decoration-solid"
          @click="restore"
        >
          Restore
        </button>
      </div>
    </div>
    <!-- Other version file -->
    <div v-else-if="!isDeleted && isOtherVersion" class="sticky top-0 z-10 -mr-12 w-full bg-yellow-600 py-2">
      <div class="mx-auto flex flex-row items-center justify-center gap-2" :style="appearance.contentWidthAsMaxWidth">
        <div class="text-sm font-bold text-white">This file belongs to another version.</div>
        <router-link
          class="text-sm text-white underline decoration-dashed underline-offset-4 hover:decoration-solid"
          :to="{ query: { version: fileHeader?.projectVersion?.id } }"
        >
          Go there
        </router-link>
      </div>
    </div>
    <!-- File failed to load -->
    <div v-else-if="!fileLoading && fileHeader == null" class="sticky top-0 z-10 -mr-12 w-full bg-red-600 py-2">
      <div class="mx-auto flex flex-row items-center justify-center gap-2" :style="appearance.contentWidthAsMaxWidth">
        <div class="text-sm font-bold text-white">File failed to load.</div>
      </div>
    </div>
    <!-- File main content -->
    <!-- (bottom padding is in last StatementAddArea) -->
    <div class="relative flex flex-col bg-white" v-if="fileHeader">
      <!-- Non-clickable invisible overlay if deleted -->
      <div v-if="isDeleted" class="absolute inset-0 z-20 flex justify-center opacity-100" />
      <!-- Fixed inline header :EditorInlineHeader -->
      <!-- (for some reason w-full doesn't work here, so set width absolutely..) -->
      <div
        class="fixed z-10 flex flex-row items-center justify-between gap-1 border-b border-orange-900 border-opacity-[12%] bg-white px-1.5"
        :class="appearance.baseClass"
        :style="{ height: appearance.editorHeaderHeight + 'px', width: props.editor.size?.value?.width + 'px' }"
      >
        <!-- Main info -->
        <div class="flex flex-row items-center">
          <!-- editor actions -->
          <ActionPopover anchor="left" :thing="file" :actions="props.editor.actions.value" class="">
            <CodeBracketIcon class="mt-1 h-4 w-4 text-gray-700" />
          </ActionPopover>
          <!-- editor path -->
          <ActionPopover anchor="left" :thing="file" :actions="fileActions" class="ml-1">
            <span class="text-gray-900">{{ name }}</span>
          </ActionPopover>
          <!-- sub path inside editor -->
          <FadeTransition mode="out-in">
            <span
              v-if="context?.statementsById[editor.activeStatementId ?? '']?.name != null"
              :key="editor.activeStatementId"
              class="flex flex-row text-gray-900"
              ><span class="text-gray-700"><ChevronRightIcon class="mr-0.5 mt-0.5 h-4 w-4 text-gray-400" /></span>
              {{ context?.statementsById[editor.activeStatementId as string].name }}</span
            >
          </FadeTransition>
          <span v-if="bench.debug" class="ml-2 bg-red-200 bg-opacity-50 text-gray-900">
            {{ editor.editing ? "(editing)" : "" }}
            {{ bench.focusedEditorId == editor.id ? "(focused)" : "" }}
          </span>
        </div>
        <div class="flex flex-row gap-1">
          <!-- Other clients presence -->
          <ClientsPopover v-if="auth.loggedIn.value" size="medium" :file-id="editor.fileId" />
        </div>
      </div>
      <!-- File header & inline actions -->
      <div
        class="group/meta relative mx-auto flex w-full flex-row items-center justify-between pt-14 font-bold text-gray-900"
        :class="appearance.baseClass"
        :style="{
          'max-width': appearance.contentWidth + appearance.contentMarginX * 2 + 'px',
          paddingLeft: `${appearance.contentMarginX + 4}px`, // + for :StatementPadding
          paddingRight: `${appearance.contentMarginX + 4}px`,
        }"
      >
        <!-- Name & actions -->
        <span class="flex flex-row items-center">
          <!-- Name -->
          <span>
            <!-- Note the :EditableSyncDance on the name update -->
            <EditableSpan
              ref="nameRef"
              class="text-3xl font-extrabold"
              :class="appearance.baseClassUnsized"
              suppress-shortcuts
              :readonly="bench.readonly || isDeleted || isOtherVersion"
              v-model="name"
              @enter="goToContent"
              @keyup.up.prevent="() => ({}) /* noop */"
            />
            <span
              class="cursor-text select-none text-3xl font-extrabold text-gray-300"
              v-if="name?.trim().length == 0"
              @click="nameRef?.focus()"
            >
              Untitled
            </span>
          </span>
          <!-- Actions -->
          <span class="ml-4 flex flex-row gap-1">
            <button
              v-for="action in fileActions.filter((action) => !action.hideInline)"
              :key="action.label"
              class="p-1 text-gray-300 hover:bg-orange-100 hover:text-gray-700 focus:bg-orange-100 group-focus-within/meta:text-gray-500 group-hover/meta:text-gray-500"
              :class="[!action.disabled ? '' : 'opacity-50 hover:cursor-not-allowed']"
              @click="action.action(fileHeader)"
              :disabled="action.disabled"
            >
              <component :is="action.icon" class="h-5 w-5" />
            </button>
          </span>
        </span>
      </div>
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
