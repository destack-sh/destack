<script lang="ts" setup>
import EditableSpan from "@/components/EditableSpan.vue";
import StatementAddArea from "@/components/StatementAddArea.vue";
import StatementInterface from "@/components/StatementInterface.vue";
import { useTimeFromNow } from "@/composables/useNow";
import { graphql, useFragment } from "@/gql";
import { StatementType, SymbolType, type StatementContentFragment } from "@/gql/graphql";
import { useActions } from "@/state/actions";
import { provideStatementActions, type FileState } from "@/state/actions/statement";
import { useEditorState, type StatementHeader } from "@/state/editor";
import { FileHeaderType, StatementContentType } from "@/state/fragments";
import { useOperations } from "@/state/operations";
import { INTEGER_ZERO } from "@/utils/fractional";
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
const { getTimeFromNowString } = useTimeFromNow(fileHeader.value?.deletedAt);
const statements = computed(() => {
  return (
    file.value?.file?.statements
      .map((statement) => useFragment(StatementContentType, statement))
      .filter((statement) => statement.deletedAt == null) || []
  );
}, {});

const rootStatements = computed(() => statements.value.filter((statement) => statement.parent == null));

const statementsById = computed(() => {
  const statementsById = {};
  statements.value.forEach((statement) => {
    statementsById[statement.id] = statement;
  });
  return statementsById;
});

/* Statements are hierarchical but laid out linearly (in one column) */
type PositionedStatement = {
  depth: number;
  lineNumberBase: number;
  isFirstInGroup: boolean;
  isLastInGroup: boolean;
  statement: StatementContentFragment;
  ancestors: string[];
};
const positionedStatements = computed(() => {
  const positionedStatements: PositionedStatement[] = [];
  let lineNumberBase = 0;

  // depth first traversal
  function walkDfs(statement: StatementContentFragment, ancestors: string[], isLast: boolean) {
    if (statement?.id == null) {
      // bail in case a bad statement ends in here due to some other bug to prevent recursion death
      console.warn("got bad statement with null id", statement, ancestors, isLast);
      return;
    }

    const children = statements.value.filter((child) => child.parent?.id == statement.id);
    const isFirstInGroup = ancestors.length == 0;
    const isLastInRoot = isLast && children.length == 0;

    positionedStatements.push({
      depth: ancestors.length,
      lineNumberBase,
      statement,
      ancestors,
      isFirstInGroup,
      isLastInGroup: isLastInRoot,
    });
    lineNumberBase += 1;

    // walk children, sorted by order key
    ancestors = [...ancestors, statement.id];
    children.sort((a, b) => ((a.orderKey ?? INTEGER_ZERO) < (b.orderKey ?? INTEGER_ZERO) ? -1 : 1));
    children.forEach((child, i) => walkDfs(child, ancestors, isLast && i == children.length - 1));
  }

  // start with roots sorted by order key
  const roots = rootStatements.value;
  roots.sort((a, b) => ((a.orderKey ?? INTEGER_ZERO) < (b.orderKey ?? INTEGER_ZERO) ? -1 : 1));
  roots.forEach((root) => walkDfs(root, [], true));

  // group groupable sibling statements at root
  for (const [i, positioned] of positionedStatements.entries()) {
    if (
      positioned.statement.type == StatementType.Import ||
      positioned.statement.type == StatementType.Comment ||
      positioned.statement.type == StatementType.Blank ||
      positioned.statement.symbolType == SymbolType.Requirement
    ) {
      const next = positionedStatements[i + 1];
      if (next && next.depth == 0 && next?.statement.type == positioned.statement.type) {
        positioned.isLastInGroup = false;
        next.isFirstInGroup = false;
      }
    }
  }

  return positionedStatements;
});

const orderedStatements = computed(() => positionedStatements.value.map((positioned) => positioned.statement));
const depths = computed(() => positionedStatements.value.map((positioned) => positioned.depth));

const fileState: Ref<FileState | null> = computed(() => {
  if (fileHeader.value == null) {
    return null;
  }
  return {
    editorId: props.editorId,
    focused: props.focused,
    file: fileHeader.value as any,
    statements: orderedStatements.value,
    depths: depths.value,
    navigateUp: () => (editor.blurElement(), nameRef.value?.focus()),
    navigateDown: () => ({}), // no-op?
  } as FileState;
});
provideStatementActions(fileState);

const operations = useOperations();
function restore() {
  operations.file.restore(fileHeader.value?.id);
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
  const lastStatement = positionedStatements.value[positionedStatements.value.length - 1];
  if (lastStatement?.statement.type == StatementType.Blank) {
    editor.editElement(lastStatement.statement as StatementHeader);
    return;
  } else {
    actions.apply("statement.insertEnd");
  }
}

const ops = useOperations();
const name: Ref<string | null> = ref(fileHeader.value?.name ?? null);
const nameRef: Ref<InstanceType<typeof EditableSpan> | null> = ref(null);

syncProperty({
  value: name,
  editing: computed(() => nameRef.value?.focused),
  read: () => (name.value = fileHeader.value?.name ?? null),
  write: () => ops.file.rename(fileHeader.value?.id, fileHeader.value?.name ?? "", name.value ?? ""),
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
  if (positionedStatements.value.length == 0) {
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
      ops.file.delete(fileHeader.value?.id);
      emit("close");
    },
    enabled: !editor.readonly,
  },
]);
</script>

<template>
  <!-- File container div -->
  <div>
    <!-- Deleted file status and restore -->
    <div v-if="isDeleted && fileHeader" class="sticky top-0 z-10 -mr-12 w-full bg-red-600 px-12 py-2">
      <div class="mx-auto flex max-w-[800px] flex-row items-center justify-center gap-2">
        <div class="text-sm font-bold text-white">This file is in Trash.</div>
        <div class="text-center text-sm text-white">
          {{ fileHeader.path }} was deleted ({{ getTimeFromNowString(fileHeader.deletedAt) }}).
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
        class="group/meta relative mx-auto flex w-full max-w-[900px] flex-row items-center px-[58px] pt-6 font-bold text-gray-900"
        :class="editor.fontMono ? 'font-mono' : ''"
      >
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
      </div>
      <!-- Add statement to start -->
      <StatementAddArea class="mx-auto max-w-[900px]" @click="editor.readonly || insertStatementStart()" />
      <!-- File's statements -->
      <div
        v-for="positioned in positionedStatements"
        :key="positioned.statement.id"
        class="mx-auto w-full max-w-[900px]"
      >
        <StatementInterface
          :file="(fileHeader as any)"
          :statement="(positioned.statement as any)"
          :readonly="isDeleted || isOtherVersion"
          :depth="positioned.depth"
          :ancestors="positioned.ancestors.map((ancestorId) => statementsById[ancestorId])"
          :isFirstInGroup="positioned.isFirstInGroup"
          :isLastInGroup="positioned.isLastInGroup"
          :lineNumberBase="positioned.lineNumberBase"
          class="w-full"
        />
      </div>
      <!-- Add statement to end -->
      <StatementAddArea
        class="mx-auto max-w-[900px] flex-1 pb-60"
        @click="editor.readonly || insertOrFocusStatementEnd()"
      />
    </div>
  </div>
</template>
