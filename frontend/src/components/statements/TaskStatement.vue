<script lang="ts" setup>
import EditableSpan from "@/components/basic/EditableSpan.vue";
import DeclarationCell from "@/components/statements/DeclarationCell.vue";
import FunctionTypeCell from "@/components/statements/FunctionTypeCell.vue";
import InlineActions from "@/components/statements/InlineActionsCell.vue";
import { useBenchState, useEditorContext, type EditorGroup, type StatementAction } from "@/state/bench";
import { TypeFlag } from "@/state/module";
import { useStatementContext } from "@/state/statement";
import { PencilSquareIcon, RocketLaunchIcon, ArrowDownRightIcon, ArrowUpRightIcon } from "@heroicons/vue/24/outline";
import { computed, nextTick, ref, type Ref } from "vue";

// all tasks are typed, but we currently re-use TaskDefinitionCell for expectations
// which are implicitly typed only for now
defineProps<{
  isTyped: boolean;
  folded?: boolean;
}>();

const bench = useBenchState();
const editor = useEditorContext();
const context = useStatementContext();

const inputs = computed(() => context.fields.value.filter((f) => !(f.flags & TypeFlag.IsOutput)));
const outputs = computed(() => context.fields.value.filter((f) => f.flags & TypeFlag.IsOutput));
const description: Ref<string> = ref(context.statement.value.description ?? "");
const descriptionRef: Ref<InstanceType<typeof EditableSpan> | null> = ref(null);
context.syncDescription(
  description,
  computed(() => descriptionRef.value?.focused)
);
const addingDescription = ref(false);
const showDescription = computed(() => description.value.length > 0 || addingDescription.value);

const declarationRef: Ref<InstanceType<typeof DeclarationCell> | null> = ref(null);
const typeRef: Ref<InstanceType<typeof FunctionTypeCell> | null> = ref(null);

function run() {
  const nextGroup = bench.nextGroup(editor.editor.value.group as EditorGroup); // open in opposite group
  bench.openRun(context.statement.value, { group: nextGroup, focus: true });
}

const extraActions = computed(() => {
  const inlineActions: StatementAction[] = [
    {
      label: "Add description",
      icon: PencilSquareIcon,
      disabled: showDescription.value,
      action: () => {
        addingDescription.value = true;
        nextTick(() => descriptionRef.value?.focus());
      },
    },
    {
      label: "Add input",
      icon: ArrowDownRightIcon,
      action: () => {
        nextTick(() => typeRef.value?.createInput());
      },
      hideInline: true,
    },
    {
      label: "Add output",
      icon: ArrowUpRightIcon,
      action: () => {
        nextTick(() => typeRef.value?.createOutput());
      },
      hideInline: true,
    },
    {
      label: "Launch",
      icon: RocketLaunchIcon,
      action: run,
    },
  ];
  return inlineActions;
});
context.setCustomActions(extraActions);

function focus(position: "first" | "last" = "first") {
  if (position == "first") {
    declarationRef.value?.focus();
  } else if (typeRef.value != null) {
    typeRef.value?.focus(position);
  } else {
    descriptionRef.value?.focus();
  }
}

defineExpose({
  focus,
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
        @navigate-down="addingDescription ? descriptionRef?.focus() : typeRef?.focus('first')"
        @navigate-right="typeRef?.focus"
      />
      <!-- Folded info -->
      <div v-if="folded" class="ml-1 flex flex-row gap-1 text-gray-400">
        <span v-if="inputs.length > 0">{{ inputs.length }} inputs</span>
        <span v-if="outputs.length > 0">{{ outputs.length }} outputs</span>
      </div>
    </div>
    <InlineActions
      class="transition duration-150 group-hover/statement:opacity-100"
      :class="context.focused.value ? '' : 'opacity-0'"
      :extraActions="extraActions"
    />
  </div>
  <!-- Content -->
  <div v-if="!folded">
    <EditableSpan
      ref="descriptionRef"
      :class="showDescription ? '' : 'h-0'"
      v-model="description"
      :readonly="context.readonly.value"
      @navigate-left="declarationRef?.focus()"
      @navigate-up="declarationRef?.focus()"
      @navigate-down="isTyped ? typeRef?.focus('first') : context.navigateDown()"
      @enter="context.insertBelow"
    />
    <button
      v-if="description.length == 0 && !context.readonly.value && addingDescription"
      tabindex="-1"
      @click="descriptionRef?.focus()"
      class="w-fit rounded-sm text-gray-300 hover:bg-orange-100 hover:text-gray-700 group-focus-within/statement:text-gray-400"
    >
      Add description
    </button>
    <!-- Inline type -->
    <FunctionTypeCell
      v-if="isTyped && (context.fields.value.length > 0 || !context.readonly.value)"
      ref="typeRef"
      @navigate-up="showDescription ? descriptionRef?.focus() : declarationRef?.focus()"
      @navigate-down="context.navigateDown"
      @navigate-left="descriptionRef?.focus"
    />
  </div>
</template>
