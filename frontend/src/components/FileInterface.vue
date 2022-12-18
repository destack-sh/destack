<script lang="ts" setup>
import StatementInterface from "@/components/StatementInterface.vue";
import { useTimeFromNow } from "@/composables/useNow";
import { graphql, useFragment } from "@/gql";
import type { StatementContentFragment } from "@/gql/graphql";
import { FileHeaderType, StatementContentType } from "@/utils/fragments";
import { useOperations } from "@/utils/operations";
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
          id
          ...StatementContent
          parent {
            id
          }
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
});

const operations = useOperations();
function restore() {
  operations.file.restore(fileHeader.value?.id);
}

/* Statements are hierarchical but laid out linearly (in one column) */
type PositionedStatement = {
  depth: number;
  lineNumberBase: number;
  isLastInRoot: boolean;
  statement: StatementContentFragment;
};
const positionedStatements = computed(() => {
  const positionedStatements: PositionedStatement[] = [];
  let lineNumberBase = 0;

  // depth first traversal
  function walkDfs(statement: StatementContentFragment, depth: number, isLast: boolean) {
    const children = statements.value.filter((child) => child.parent?.id == statement.id);
    const isLastInRoot = isLast && children.length == 0;
    positionedStatements.push({ depth, lineNumberBase, statement, isLastInRoot });
    lineNumberBase += 1; // should be statement.content.length but that's not implemented yet

    // sort by index
    children.sort((a, b) => (a.index ?? 0) - (b.index ?? 0));
    children.forEach((child, i) => walkDfs(child, depth + 1, isLast && i == children.length - 1));
  }

  // start with roots sorted by index
  const roots = statements.value.filter((statement) => statement.parent == undefined);
  roots.sort((a, b) => (a.index ?? 0) - (b.index ?? 0));
  roots.forEach((root) => walkDfs(root, 0, true));
  return positionedStatements;
});
</script>

<template>
  <div class="mx-8 my-3 flex h-full flex-col" v-if="fileHeader" :class="isDeleted ? 'opacity-50' : ''">
    <StatementInterface
      v-for="positioned in positionedStatements"
      :key="positioned.statement.id"
      :file="fileHeader"
      :statement="positioned.statement"
      :depth="positioned.depth"
      :isLastInRoot="positioned.isLastInRoot"
      :lineNumberBase="positioned.lineNumberBase"
      class="mx-auto w-full max-w-[1000px] bg-white"
      :class="positioned.depth == 0 ? 'mt-6' : ''"
    />
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
