<script lang="ts" setup>
import EditableSpan from "@/components/basic/EditableSpan.vue";
import ModifierCell from "@/components/statements/ModifierCell.vue";
import ProtoStatementTypeCell from "@/components/statements/ProtoStatementTypeCell.vue";
import StatementTypeCell from "@/components/statements/StatementTypeCell.vue";
import { useCurrentModule } from "@/state/module";
import { useStatementContext } from "@/state/statement";
import { ref, type Ref } from "vue";

defineProps<{ showDots?: boolean }>();

const context = useStatementContext();

const startRef: Ref<InstanceType<typeof EditableSpan> | null> = ref(null);
const gapRef: Ref<InstanceType<typeof ProtoStatementTypeCell> | null> = ref(null);

// symbols available for reference
const module = useCurrentModule();

function deleteModifierOrAbove() {
  if (context.statement.value.modifier != null) {
    context.setModifier(null);
  } else {
    context.tryDeleteLeft();
  }
}

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
    startRef.value?.blur();
    gapRef.value?.blur();
  },
});
</script>
<template>
  <!-- TODO @Cleanup: compress/simplify navigation across cells (proto, definition, ..) -->
  <span class="flex flex-row gap-1 outline-none">
    <EditableSpan
      :model-value="''"
      ref="startRef"
      class="-mx-0.5"
      v-if="context.statement.value.modifier != null"
      @navigate-up="context.navigateUp"
      @navigate-down="context.navigateDown"
      @navigate-right="gapRef?.focus"
      @delete-left="context.tryDeleteLeft"
      @enter="context.insertAbove"
      @escape="context.escape"
      :readonly="context.readonly.value"
    />
    <ModifierCell v-if="context.statement.value.modifier" />
    <ProtoStatementTypeCell
      class="-mx-0.5"
      ref="gapRef"
      @navigate-up="context.navigateUp"
      @navigate-down="context.navigateDown"
      @delete-left="deleteModifierOrAbove"
      @navigate-left="startRef?.focus"
      @enter="context.insertAbove"
      @escape="context.escape"
    />
    <StatementTypeCell v-if="context.statement.value.type" />
    <!-- Empty dots / prompt -->
    <div
      v-if="
        showDots &&
        context.statement.value.type == null &&
        context.statement.value.modifier == null &&
        gapRef?.content?.length == 0 &&
        context.focused.value
      "
      class="h-full w-full select-none group-hover:opacity-100"
    >
      <span class="text-gray-400" v-if="!context.editing.value">...</span>
      <span class="text-gray-400" v-else>'/' for commands or just type...</span>
    </div>
  </span>
</template>
