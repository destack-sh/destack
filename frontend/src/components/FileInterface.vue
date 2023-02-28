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
import { useQuery } from "@vue/apollo-composable";
import { useDebounceFn } from "@vueuse/shared";
import { computed, ref, watch, type Ref } from "vue";

const props = defineProps<{ fileId: string; focused: boolean }>();
const editor = useEditorState();
const actions = useActions();

const { result: file } = useQuery(
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

const operations = useOperations();
function restore() {
  operations.file.restore(fileHeader.value?.id);
}

/* Statements are hierarchical but laid out linearly (in one column) */
type PositionedStatement = {
  depth: number;
  lineNumberBase: number;
  isFirstInGroup: boolean;
  isLastInGroup: boolean;
  statement: StatementContentFragment;
};
const positionedStatements = computed(() => {
  const positionedStatements: PositionedStatement[] = [];
  let lineNumberBase = 0;

  // depth first traversal
  function walkDfs(statement: StatementContentFragment, depth: number, isLast: boolean) {
    if (statement?.id == null) {
      // bail in case a bad statement ends in here due to some other bug to prevent recursion death
      console.warn("got bad statement with null id", statement, depth, isLast);
      return;
    }

    const children = statements.value.filter((child) => child.parent?.id == statement.id);

    const isFirstInGroup = depth == 0;
    const isLastInRoot = isLast && children.length == 0;

    positionedStatements.push({ depth, lineNumberBase, statement, isFirstInGroup, isLastInGroup: isLastInRoot });
    lineNumberBase += 1;

    // sort by order key
    children.sort((a, b) => ((a.orderKey ?? INTEGER_ZERO) < (b.orderKey ?? INTEGER_ZERO) ? -1 : 1));
    children.forEach((child, i) => walkDfs(child, depth + 1, isLast && i == children.length - 1));
  }

  // start with roots sorted by order key
  const roots = rootStatements.value;
  roots.sort((a, b) => ((a.orderKey ?? INTEGER_ZERO) < (b.orderKey ?? INTEGER_ZERO) ? -1 : 1));
  roots.forEach((root) => walkDfs(root, 0, true));

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

const fileState: Ref<FileState> = computed(
  () =>
    ({
      focused: props.focused,
      file: fileHeader.value as any,
      statements: orderedStatements.value,
      depths: depths.value,
    } as FileState)
);
provideStatementActions(fileState);

async function insertStatementStart() {
  actions.apply("statement.insertStart");
}

async function insertOrFocusStatementEnd() {
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

// set name first if
watch(
  () => fileHeader.value?.name,
  () => {
    if (name.value == null) {
      name.value = fileHeader.value?.name ?? null;
    }
  }
);
function renameFile(newName: string) {
  if (fileHeader.value == null || newName.trim().length == 0) {
    return;
  }
  ops.file.rename(fileHeader.value?.id, fileHeader.value?.name ?? "", newName);
}
const renameFileDebounced = useDebounceFn(renameFile, 500);
</script>

<template>
  <!-- File container div -->
  <div>
    <!-- Deleted file status and restore -->
    <div v-if="isDeleted && fileHeader" class="sticky top-0 z-10 -mr-12 w-full bg-red-600 px-12 py-2">
      <div class="mx-auto flex max-w-[800px] flex-row items-center gap-2">
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
      <div class="mx-auto flex max-w-[800px] flex-row items-center gap-2">
        <div class="text-sm font-bold text-white">This file belongs to another version.</div>
        <router-link
          class="text-sm text-white underline decoration-dashed underline-offset-4 hover:decoration-solid"
          :to="{ query: { version: fileHeader?.projectVersion?.id } }"
        >
          Go there
        </router-link>
      </div>
    </div>
    <!-- bottom padding is in last StatementAddArea -->
    <div class="relative flex flex-col bg-white px-12" v-if="fileHeader">
      <!-- Non-clickable invisible overlay if deleted -->
      <div v-if="isDeleted" class="absolute inset-0 z-10 flex justify-center opacity-100" />
      <!-- File name & meta -->
      <!-- TODO @UX: move nav focus smoothly between file name and statements (up/down)  -->
      <div
        class="relative mx-auto w-full max-w-[800px] px-2 pt-6 font-bold text-gray-900"
        :class="editor.fontMono ? 'font-mono' : ''"
      >
        <EditableSpan
          ref="nameRef"
          class="text-3xl"
          :readonly="editor.readonly || isDeleted || isOtherVersion"
          @update:model-value="(newName) => ((name = newName), renameFileDebounced(newName))"
          :model-value="name"
        />
        <span
          class="cursor-text select-none text-3xl text-gray-300"
          v-if="name?.trim().length == 0"
          @click="nameRef?.focus()"
        >
          Untitled AI
        </span>
      </div>
      <!-- Add statement to start -->
      <StatementAddArea class="mx-auto max-w-[850px]" @click="editor.readonly || insertStatementStart()" />
      <!-- File's statements -->
      <template v-for="positioned in positionedStatements" :key="positioned.statement.id">
        <StatementInterface
          :file="(fileHeader as any)"
          :statement="(positioned.statement as any)"
          :readonly="isDeleted || isOtherVersion"
          :depth="positioned.depth"
          :isFirstInGroup="positioned.isFirstInGroup"
          :isLastInGroup="positioned.isLastInGroup"
          :lineNumberBase="positioned.lineNumberBase"
          class="mx-auto w-full max-w-[800px]"
        />
      </template>
      <!-- Add statement to end -->
      <StatementAddArea
        class="mx-auto max-w-[850px] flex-1 pb-60"
        @click="editor.readonly || insertOrFocusStatementEnd()"
      />
    </div>
  </div>
</template>
