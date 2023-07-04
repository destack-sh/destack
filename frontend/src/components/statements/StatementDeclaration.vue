<script lang="ts" setup>
import EditableSpan from "@/components/basic/EditableSpan.vue";
import { getStatementIcon, useStatementContext } from "@/state/statement";
import { computed, ref, type Ref } from "vue";
import { useKeyModifier } from "@vueuse/core";
import { useEditorContext, type StatementHeader } from "@/state/bench";

const context = useStatementContext();

const emit = defineEmits<{
  (e: "navigateDown"): void;
  (e: "navigateLeft"): void;
  (e: "navigateRight"): void;
}>();

const nameRef: Ref<InstanceType<typeof EditableSpan> | null> = ref(null);
const name: Ref<string> = ref(context.statement.value.name ?? "");
context.syncName(
  name,
  computed(() => nameRef.value?.focused)
);
const hasName = computed(() => name.value.trim().length > 0);
const startRef: Ref<InstanceType<typeof EditableSpan> | null> = ref(null);
const icon = computed(() => getStatementIcon(context.statement.value.type, context.statement.value.rootTypeTag));

const altKeyState = useKeyModifier("Alt");
const editor = useEditorContext();

function openInEditor() {
  editor.editor.value.bench.openStatement(context.statement.value as StatementHeader, { focus: true });
}

defineExpose({
  focus: (position: "first" | "last" = "first") => nameRef.value?.focus(),
  blur: () => {
    startRef.value?.blur();
    nameRef.value?.blur();
  },
});
</script>
<template>
  <div class="relative flex w-fit flex-row whitespace-nowrap">
    <!-- Icon -->
    <component :is="icon" class="absolute top-0.5 h-4 w-4 text-orange-600" />
    <!-- Alt click to open in full -->
    <EditableSpan
      ref="nameRef"
      class="text-md ml-[18px] px-0.5 font-semibold text-orange-600"
      :class="altKeyState ? 'cursor-pointer decoration-gray-600 underline-offset-4 hover:underline' : ''"
      @click="altKeyState && openInEditor()"
      v-model="name"
      :readonly="context.readonly.value"
      @navigate-up="context.navigateUp"
      @navigate-down="emit('navigateDown')"
      @navigate-left="emit('navigateLeft')"
      @navigate-right="emit('navigateRight')"
      @escape="context.escape"
      @enter="context.insertBelow"
    />
    <!-- 'anon' placeholder if unnamed as a button -->
    <button
      tabindex="-1"
      v-if="!hasName"
      @click="nameRef?.focus()"
      class="-ml-0.5 -mt-0.5 w-fit select-none rounded-sm text-gray-300 hover:bg-orange-100 hover:text-gray-700 group-focus-within/statement:text-gray-400"
    >
      unnamed
    </button>
  </div>
</template>
