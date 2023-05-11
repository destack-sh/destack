<script lang="ts" setup>
import EditableSpan from "@/components/EditableSpan.vue";
import StatementAddArea from "@/components/StatementAddArea.vue";
import StatementInterface from "@/components/StatementInterface.vue";
import ClientsPopover from "@/components/basic/ClientsPopover.vue";
import { provideFileState, type FileState } from "@/components/file";
import { useTimeFromNow } from "@/composables/useNow";
import { graphql, useFragment } from "@/gql";
import { StatementType } from "@/gql/graphql";
import { useActions } from "@/state/actions";
import { useAuth } from "@/state/auth";
import { useEditorState, type StatementHeader } from "@/state/editor";
import { FileHeaderType, StatementContentType } from "@/state/fragments";
import { useOperations } from "@/state/operations";
import { syncProperty } from "@/utils/sync";
import { ArrowUturnRightIcon, DocumentDuplicateIcon, TrashIcon } from "@heroicons/vue/24/outline";
import { useQuery } from "@vue/apollo-composable";
import { computed, nextTick, ref, watch, type Ref } from "vue";

const props = defineProps<{ editorId: string; fileId: string; focused: boolean }>();
const emit = defineEmits<{ (e: "close"): void }>();
const editor = useEditorState();
const actions = useActions();

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
    fileId: props.fileId,
  })
);
const fileHeader = computed(() => useFragment(FileHeaderType, file.value?.file) ?? undefined);
const isDeleted = computed(() => fileHeader.value?.deletedAt != null);
const isOtherVersion = computed(
  () =>
    editor.currentProjectVersionId != null &&
    fileHeader.value != null &&
    fileHeader.value?.projectVersion?.id != editor.currentProjectVersionId
);
const now = useTimeFromNow(fileHeader.value?.deletedAt);
const statements = computed(() => {
  return (
    file.value?.file?.statements
      .map((statement) => useFragment(StatementContentType, statement))
      .filter((statement) => statement.deletedAt == null) || []
  );
}, {});

const fileState: Ref<FileState | null> = computed(() => {
  if (fileHeader.value == null) {
    return null;
  }
  return {
    editorId: props.editorId,
    focused: props.focused,
    file: fileHeader.value as any,
    statementsUnordered: statements.value,
    navigateUp: () => (editor.blurElement(), nameRef.value?.focus()),
    navigateDown: () => ({}), // no-op?
  } as FileState;
});
const context = provideFileState(fileState);

const ops = useOperations();
function restore() {
  ops.file.restore(null, fileHeader.value?.id);
}

async function insertStatementStart() {
  if (fileHeader.value == null) return;
  editor.focusFile(fileHeader.value);
  actions.apply("statement.insertStart");
}

async function insertOrFocusStatementEnd() {
  if (fileHeader.value == null) return;
  editor.focusFile(fileHeader.value);
  // focus last statement if it's a blank
  const lastStatement = context.value?.positionedStatements[context.value.positionedStatements.length - 1];
  if (lastStatement?.statement.type == StatementType.Blank) {
    editor.editElement(lastStatement.statement as StatementHeader);
    return;
  } else {
    actions.apply("statement.insertEnd");
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

function goToContent() {
  nameRef.value?.blur();
  if (context.value?.positionedStatements.length == 0) {
    insertStatementStart();
  } else {
    insertOrFocusStatementEnd();
  }
}

// file meta actions
const metaActions = computed(() => [
  {
    label: "Duplicate",
    icon: DocumentDuplicateIcon,
    action: () => {
      // not implemented yet
    },
    enabled: false,
  },
  {
    label: "Move",
    icon: ArrowUturnRightIcon,
    action: () => {
      // not implemented yet
    },
    enabled: false,
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
    enabled: !editor.readonly,
  },
]);

const auth = useAuth();
</script>

<template>
  <!-- File container div -->
  <div>
    <!-- Deleted file status and restore -->
    <div v-if="isDeleted && fileHeader" class="sticky top-0 z-10 -mr-12 w-full bg-red-600 px-12 py-2">
      <div class="mx-auto flex max-w-[800px] flex-row items-center justify-center gap-2">
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
    <div v-else-if="!isDeleted && isOtherVersion" class="sticky top-0 z-10 -mr-12 w-full bg-yellow-600 px-12 py-2">
      <div class="mx-auto flex max-w-[800px] flex-row items-center justify-center gap-2">
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
    <div v-else-if="!fileLoading && fileHeader == null" class="sticky top-0 z-10 -mr-12 w-full bg-red-600 px-12 py-2">
      <div class="mx-auto flex max-w-[800px] flex-row items-center justify-center gap-2">
        <div class="text-sm font-bold text-white">File failed to load.</div>
      </div>
    </div>
    <!-- bottom padding is in last StatementAddArea -->
    <div class="relative flex flex-col bg-white px-12" v-if="fileHeader">
      <!-- Non-clickable invisible overlay if deleted -->
      <div v-if="isDeleted" class="absolute inset-0 z-10 flex justify-center opacity-100" />
      <!-- File name & meta actions -->
      <div
        class="group/meta relative mx-auto flex w-full max-w-[900px] flex-row items-center justify-between px-[58px] pt-6 font-bold text-gray-900"
        :class="editor.fontMono ? 'font-mono' : ''"
      >
        <!-- Name & actions -->
        <span class="flex flex-row items-center">
          <!-- Name -->
          <span>
            <!-- Note the :EditableSyncDance on the name update -->
            <EditableSpan
              ref="nameRef"
              class="text-3xl"
              suppress-shortcuts
              :readonly="editor.readonly || isDeleted || isOtherVersion"
              v-model="name"
              @enter="goToContent"
              @keyup.up.prevent="() => ({}) /* noop */"
            />
            <span
              class="cursor-text select-none text-3xl text-gray-300"
              v-if="name?.trim().length == 0"
              @click="nameRef?.focus()"
            >
              Untitled AI
            </span>
          </span>
          <!-- Actions -->
          <span class="ml-4 flex flex-row gap-1">
            <button
              v-for="action in metaActions"
              :key="action.label"
              class="p-1 text-gray-300 hover:bg-orange-100 hover:text-gray-700 focus:bg-orange-100 group-focus-within/meta:text-gray-500 group-hover/meta:text-gray-500"
              :class="[action.enabled ? '' : 'opacity-50 hover:cursor-not-allowed']"
              @click="action.action()"
              :disabled="!action.enabled"
            >
              <component :is="action.icon" class="h-5 w-5" />
            </button>
          </span>
        </span>
        <!-- Other clients presence -->
        <ClientsPopover v-if="auth.loggedIn.value" size="medium" :file-id="props.fileId" />
      </div>
      <!-- Add statement to start -->
      <StatementAddArea
        class="mx-auto max-w-[900px]"
        position="start"
        @click="editor.readonly || insertStatementStart()"
        v-if="statements?.length > 0"
      />
      <!-- File's statements -->
      <div
        v-for="positioned in context?.positionedStatements"
        :key="positioned.statement.id"
        class="mx-auto w-full max-w-[900px]"
      >
        <StatementInterface
          :file="(fileHeader as any)"
          :statement="(positioned.statement as any)"
          :readonly="isDeleted || isOtherVersion"
          :depth="positioned.depth"
          :ancestors="positioned.ancestors.map((ancestorId) => context?.statementsById[ancestorId])"
          :isFirstInGroup="positioned.isFirstInGroup"
          :isLastInGroup="positioned.isLastInGroup"
          :lineNumberBase="positioned.lineNumberBase"
          class="w-full"
        />
      </div>
      <!-- Add statement to end -->
      <StatementAddArea
        class="mx-auto max-w-[900px] flex-1 pb-60"
        position="end"
        @click="editor.readonly || insertOrFocusStatementEnd()"
      />
    </div>
  </div>
</template>
