<script lang="ts" setup>
import ProtoStatementTypeCell from "@/components/statements/ProtoStatementTypeCell.vue";
import StatementTypeCell from "@/components/statements/StatementTypeCell.vue";
import { StatementType } from "@/gql/graphql";
import { useStatementContext } from "@/state/statement";
import { ref, type Ref } from "vue";

defineProps<{ showDots?: boolean; folded?: boolean }>();

const context = useStatementContext();

const gapRef: Ref<InstanceType<typeof ProtoStatementTypeCell> | null> = ref(null);

defineExpose({
  focus: (position: "first" | "last" = "first") => {
    if (context.statement.value.type == null) {
      // prever gap if we don't have a symbol type declared yet
      gapRef.value?.focus();
    } else {
      gapRef.value?.focus();
    }
  },
  blur: () => {
    gapRef.value?.blur();
  },
  loading: ref(false),
});
</script>
<template>
  <span class="flex flex-row outline-none">
    <ProtoStatementTypeCell
      class=""
      ref="gapRef"
      @navigate-up="context.navigateUp"
      @navigate-down="context.navigateDown"
      @delete-left="context.tryDeleteLeft"
      @enter="context.insertAbove"
      @escape="context.escape"
    />
    <StatementTypeCell v-if="context.statement.value.type" />
    <!-- Empty dots / prompt -->
    <div
      v-if="
        showDots &&
        context.statement.value.type == StatementType.Blank &&
        gapRef?.content?.length == 0 &&
        context.focused.value
      "
      class="h-full w-full select-none group-hover:opacity-100"
    >
      <span class="text-gray-400" v-if="!context.editing.value">...</span>
      <span class="text-gray-400" v-else>Press '/' for commands, type for text...</span>
    </div>
  </span>
</template>
