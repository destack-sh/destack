<script lang="ts" setup>
import StatementAddArea from "@/components/StatementAddArea.vue";
import StatementInterface from "@/components/StatementInterface.vue";
import { useTimeFromNow } from "@/composables/useNow";
import { graphql, useFragment } from "@/gql";
import { StatementType, SymbolType, type StatementContentFragment } from "@/gql/graphql";
import { useStatementActions } from "@/state/actions/statement";
import { useEditorState } from "@/state/editor";
import { FileHeaderType, StatementContentType } from "@/state/fragments";
import { useOperations } from "@/state/operations";
import { useQuery } from "@vue/apollo-composable";
import { computed } from "vue";

const props = defineProps<{ fileId: string }>();

const { result: file } = useQuery(
  graphql(/* GraphQL */ `
    query fileContentById($fileId: GlobalID!) {
      file(id: $fileId) {
        id
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
const fileHeader = computed(() => useFragment(FileHeaderType, file.value?.file));
const isDeleted = computed(() => fileHeader.value?.deletedAt != null);
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

function getStatementContentLength(statement: StatementContentFragment) {
  return 0;
}

/* Statements are hierarchical but laid out linearly (in one column) */
/* Certain statements may be grouped outside the hierarchy */
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
    const children = statements.value.filter((child) => child.parent?.id == statement.id);

    const isFirstInGroup = depth == 0;
    const isLastInRoot = isLast && children.length == 0;

    positionedStatements.push({ depth, lineNumberBase, statement, isFirstInGroup, isLastInGroup: isLastInRoot });
    lineNumberBase += 1 + getStatementContentLength(statement);

    // sort by index
    children.sort((a, b) => (a.index ?? 0) - (b.index ?? 0));
    children.forEach((child, i) => walkDfs(child, depth + 1, isLast && i == children.length - 1));
  }

  // start with roots sorted by index
  const roots = rootStatements.value;
  roots.sort((a, b) => (a.index ?? 0) - (b.index ?? 0));
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

const editor = useEditorState();
const orderedStatements = computed(() => positionedStatements.value.map((positioned) => positioned.statement));
const focused = computed(() => editor.focusedFileId == fileHeader.value?.id);

useStatementActions(focused, fileHeader, orderedStatements);
</script>

<template>
  <div class="flex h-full flex-col bg-white py-3 px-7" v-if="fileHeader" :class="isDeleted ? 'opacity-50' : ''">
    <!-- Add statement to start -->
    <StatementAddArea class="mx-auto max-w-[1050px]" :file="fileHeader" :index="0" />
    <!-- File's statements -->
    <template v-for="positioned in positionedStatements" :key="positioned.statement.id">
      <StatementInterface
        :file="(fileHeader as any)"
        :statement="(positioned.statement as any)"
        :depth="positioned.depth"
        :isFirstInGroup="positioned.isFirstInGroup"
        :isLastInGroup="positioned.isLastInGroup"
        :lineNumberBase="positioned.lineNumberBase"
        class="mx-auto w-full max-w-[1000px]"
      />
    </template>
    <!-- Add statement to end -->
    <StatementAddArea class="mx-auto max-w-[1050px] flex-1" :file="fileHeader" :index="rootStatements.length" />
    <!-- Deleted overlay with restore button -->
    <div v-if="isDeleted" class="absolute inset-0 flex items-center justify-center opacity-100">
      <div class="flex flex-col items-center gap-2">
        <div class="text-2xl font-bold text-red-700">Deleted</div>
        <div class="text-center text-sm">
          {{ fileHeader.path }} was deleted ({{ getTimeFromNowString(fileHeader.deletedAt) }}).
        </div>
        <button class="" @click="restore">Restore</button>
      </div>
    </div>
  </div>
</template>
