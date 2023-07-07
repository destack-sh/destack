<script lang="ts" setup>
import EditableSpan from "@/components/basic/EditableSpan.vue";
import StatementDeclaration from "@/components/statements/StatementDeclaration.vue";
import FunctionType from "@/components/statements/FunctionType.vue";
import InlineActions from "@/components/statements/StatementActions.vue";
import { useBenchState, useEditorContext, type EditorGroup, type StatementAction } from "@/state/bench";
import { TypeFlag } from "@/state/module";
import { useStatementContext } from "@/state/statement";
import {
  PencilSquareIcon,
  ArrowDownRightIcon,
  ArrowUpRightIcon,
  ArrowLongRightIcon,
  WindowIcon,
  Bars3Icon,
} from "@heroicons/vue/24/outline";
import { computed, nextTick, ref, type Ref } from "vue";
import StatementTags from "@/components/statements/StatementTags.vue";

const props = defineProps<{ isTyped: boolean; folded?: boolean }>();
const emit = defineEmits<{ (e: "toggleFold"): void }>();

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

const declarationRef: Ref<InstanceType<typeof StatementDeclaration> | null> = ref(null);
const typeRef: Ref<InstanceType<typeof FunctionType> | null> = ref(null);

function run() {
  const nextGroup = bench.nextGroup(editor.editor.value.group as EditorGroup); // open in opposite group
  bench.openRun(context.statement.value, { group: nextGroup, focus: true });
}

function unfoldIfFolded() {
  if (props.folded) emit("toggleFold");
}

const extraActions = computed(() => {
  const inlineActions: StatementAction[] = [
    {
      label: "Add description",
      icon: Bars3Icon,
      disabled: showDescription.value,
      action: () => {
        unfoldIfFolded();
        addingDescription.value = true;
        nextTick(() => descriptionRef.value?.focus());
      },
    },
    {
      label: "Add input",
      icon: ArrowDownRightIcon,
      action: () => {
        nextTick(() => (unfoldIfFolded(), typeRef.value?.createInput()));
      },
      hideInline: true,
    },
    {
      label: "Add output",
      icon: ArrowUpRightIcon,
      action: () => {
        nextTick(() => (unfoldIfFolded(), typeRef.value?.createOutput()));
      },
      hideInline: true,
    },
    {
      label: "Launch",
      icon: WindowIcon,
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
    <div class="flex flex-row">
      <StatementDeclaration
        ref="declarationRef"
        class="inline-flex"
        @navigate-down="
          folded ? context.navigateDown() : addingDescription ? descriptionRef?.focus() : typeRef?.focus('first')
        "
        @navigate-right="typeRef?.focus"
      />
      <StatementTags />
    </div>
    <InlineActions
      class="transition duration-150 group-hover/statement:opacity-100"
      :class="context.focused.value ? '' : 'opacity-0'"
      :extraActions="extraActions"
    />
  </div>
  <!-- Folded info -->
  <button
    v-if="folded"
    class="-mx-0.5 flex max-w-full flex-row gap-1.5 truncate rounded-sm px-0.5 text-gray-400 hover:bg-gray-100"
    @click="$emit('toggleFold')"
  >
    <span v-for="input in inputs" :key="input.id">{{ input.name }}</span>
    <ArrowLongRightIcon v-if="outputs.length > 0" class="mt-0.5 h-4 w-4 text-gray-400" />
    <span v-for="output in outputs" :key="output.id">{{ output.name }}</span>
  </button>
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
    <FunctionType
      v-if="isTyped && (context.fields.value.length > 0 || !context.readonly.value)"
      ref="typeRef"
      @navigate-up="showDescription ? descriptionRef?.focus() : declarationRef?.focus()"
      @navigate-down="context.navigateDown"
      @navigate-left="descriptionRef?.focus"
    />
  </div>
</template>
