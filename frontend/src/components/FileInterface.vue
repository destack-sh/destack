<script lang="ts" setup>
import StatementInterface from "@/components/StatementInterface.vue";
import { graphql, useFragment, type FragmentType } from "@/gql";
import { FileHeaderType, StatementContentType } from "@/utils/fragments";
import { useQuery } from "@vue/apollo-composable";
import { computed } from "vue";

const props = defineProps<{ fileId: string }>();

const { result: file } = useQuery(
  graphql(/* GraphQL */ `
    query fileContentById($fileId: GlobalID!) {
      file(id: $fileId) {
        id
        ...FileHeader
        statements {
          id
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
const statements = computed(() => {
  return file.value?.file?.statements.map((statement) => useFragment(StatementContentType, statement)) || [];
});
type StatementContentTypeNested = FragmentType<typeof StatementContentType> & {
  children: StatementContentTypeNested[];
};

// statements can be nested (has parent and children fields)
// the statements array in the file is a flat list of all statements
// we need to build a tree of statements (that is ordered by their indices)
const statementsByParent = computed(() => {
  const statementsByParent: Record<string, FragmentType<typeof StatementContentType>[]> = {};
  for (const statement of statements.value) {
    if (statement.parent) {
      if (!statementsByParent[statement.parent.id]) {
        statementsByParent[statement.parent.id] = [];
      }
      statementsByParent[statement.parent.id].push(statement);
    }
  }
  // sort statements by their index
  for (const parent in statementsByParent) {
    statementsByParent[parent].sort((a, b) => a.index - b.index);
  }
  return statementsByParent;
});
const statementsWithChildren = computed(() => {
  const statementsWithChildren: StatementContentTypeNested[] = [];
  for (const statement of statements.value) {
    statementsWithChildren.push({
      ...statement,
      children: statementsByParent.value[statement.id] || [],
    });
  }
  return statementsWithChildren;
});
const rootStatements = computed(() => {
  return statementsWithChildren.value.filter((statement) => !statement.parent);
});
</script>

<template>
  <div class="mx-8 my-3 flex h-full flex-col gap-6">
    <StatementInterface
      v-for="statement in rootStatements"
      :key="statement.id"
      :file="fileHeader"
      :statement="statement"
      :depth="0"
      class="mx-auto w-full max-w-[1000px]"
    />
  </div>
</template>
