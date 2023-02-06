<script lang="ts" setup>
import EditableSpan from "@/components/EditableSpan.vue";
import { useStatementContext } from "@/components/statement";
import { ref, watch, type Ref } from "vue";

defineProps<{ showDots: boolean }>();

const context = useStatementContext();
const content: Ref<string> = ref("");
const spanRef: Ref<InstanceType<typeof EditableSpan> | null> = ref(null);

function insertBelow() {
  context.actions.apply("statement.insertBelowCurrent");
}

function deleteSelf() {
  context.actions.apply("statement.deleteCurrent");
}

watch(content, (newContent) => {
  console.log("content changed in " + context.statement.value.id, newContent);
});

defineExpose({
  focus: () => spanRef.value?.focus(),
  defocus: () => spanRef.value?.defocus(),
});
</script>
<template>
  <EditableSpan
    ref="spanRef"
    v-model="content"
    :readonly="context.readonly.value"
    @enter="insertBelow"
    @delete-left="deleteSelf"
    @navigate-up="context.navigateUp"
    @navigate-down="context.navigateDown"
    @escape="context.escape"
  />
  <div
    v-if="showDots && content.length == 0"
    class="absolute bottom-0 mx-1 h-full w-full select-none text-gray-300 group-hover:opacity-100"
    :class="{ 'opacity-100': context.focused.value, 'opacity-0': !context.focused.value }"
  >
    ...
  </div>
</template>
