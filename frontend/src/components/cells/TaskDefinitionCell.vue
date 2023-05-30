<script lang="ts" setup>
import InlineActions from "@/components/cells/InlineActionsCell.vue";
import DeclarationCell from "@/components/cells/DeclarationCell.vue";
import FunctionTypeCell from "@/components/cells/FunctionTypeCell.vue";
import EditableSpan from "@/components/basic/EditableSpan.vue";
import { useStatementContext } from "@/state/statement";
import { useBenchState, useEditorContext, type EditorGroup, type StatementAction } from "@/state/bench";
import { PlayIcon } from "@heroicons/vue/24/outline";
import { computed, ref, type Ref } from "vue";

// all tasks are typed, but we currently re-use TaskDefinitionCell for expectations
// which are implicitly typed only for now
defineProps<{
  isTyped: boolean;
}>();

const bench = useBenchState();
const editor = useEditorContext();
const context = useStatementContext();

const description: Ref<string> = ref(context.statement.value.description ?? "");
const descriptionRef: Ref<InstanceType<typeof EditableSpan> | null> = ref(null);
context.syncDescription(
  description,
  computed(() => descriptionRef.value?.focused)
);
const addingDescription = ref(false);

const declarationRef: Ref<InstanceType<typeof DeclarationCell> | null> = ref(null);
const typeRef: Ref<InstanceType<typeof FunctionTypeCell> | null> = ref(null);

function run() {
  const nextGroup = bench.nextGroup(editor.editor.value.group as EditorGroup); // open in opposite group
  bench.openRun(context.statement.value, { group: nextGroup, focus: true });
}

const extraActions = computed(() => {
  const inlineActions: StatementAction[] = [
    {
      label: "Run",
      icon: PlayIcon,
      action: run,
    },
  ];
  return inlineActions;
});

defineExpose({
  focus: (position: "first" | "last" = "first") => {
    if (position == "first") {
      declarationRef.value?.focus();
    } else if (typeRef.value != null) {
      typeRef.value?.focus(position);
    } else {
      descriptionRef.value?.focus();
    }
  },
  blur: () => {
    declarationRef.value?.blur();
    typeRef.value?.blur();
    descriptionRef.value?.blur();
  },
  run,
});
</script>
<template>
  <!-- Declaration -->
  <div class="flex flex-row justify-between">
    <div class="flex flex-row items-baseline">
      <DeclarationCell
        ref="declarationRef"
        class="inline-flex"
        @navigate-down="descriptionRef?.focus"
        @navigate-right="typeRef?.focus"
      />
      <button
        v-if="description.length == 0 && !context.readonly.value && !addingDescription"
        tabindex="-1"
        @click="
          addingDescription = true;
          descriptionRef?.focus();
        "
        class="ml-2 w-fit rounded-sm px-0.5 text-gray-300 hover:bg-orange-100 hover:text-gray-700 focus:bg-orange-100 focus:outline-none group-focus-within/statement:text-gray-400"
      >
        +description
      </button>
    </div>
    <InlineActions
      class="transition duration-150 group-hover/statement:opacity-100"
      :class="context.focused.value ? '' : 'opacity-0'"
      :extraActions="extraActions"
    />
  </div>
  <!-- Description -->
  <div>
    <EditableSpan
      ref="descriptionRef"
      :class="addingDescription ? '' : 'h-0'"
      v-model="description"
      :readonly="context.readonly.value"
      @navigate-left="declarationRef?.focus()"
      @navigate-up="declarationRef?.focus()"
      @navigate-down="isTyped ? typeRef?.focus() : context.navigateDown()"
      @enter="context.insertBelow"
    />
    <button
      v-if="description.length == 0 && !context.readonly.value && addingDescription"
      tabindex="-1"
      @click="descriptionRef?.focus()"
      class="w-fit rounded-sm px-0.5 text-gray-300 hover:bg-orange-100 hover:text-gray-700 group-focus-within/statement:text-gray-400"
    >
      +description
    </button>
    <!-- Inline type -->
    <FunctionTypeCell
      v-if="isTyped && (context.typeNodes.value.length > 0 || !context.readonly.value)"
      ref="typeRef"
      @navigate-up="context.navigateUp"
      @navigate-down="context.navigateDown"
      @navigate-left="descriptionRef?.focus"
    />
  </div>
</template>
