<script lang="ts" setup>
import { StatementType } from "@/gql/graphql";
import { useCurrentModule } from "@/state/module";
import { useStatementContext } from "@/state/statement";

const context = useStatementContext();
const module = useCurrentModule();

const availableTags = module.statementsLike({
  types: [StatementType.Tag],
  includeDependencies: true,
});
</script>
<template>
  <div>tags! of {{ availableTags.length }}</div>
</template>
