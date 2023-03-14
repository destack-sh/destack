<script lang="ts" setup>
import { useNavigationGrid } from "@/components/cells/grid";
import InlineTypeCell from "@/components/cells/InlineTypeCell.vue";
import InlineValueCell from "@/components/cells/InlineValueCell.vue";
import { makeTypeNode, STRING_TYPE_NODE, useStatementContext, type SimpleType } from "@/components/statement";
import { TypeTag, type SimpleTypeNode } from "@/gql/graphql";
import { generateKeyBetween, INTEGER_ZERO } from "@/utils/fractional";
import { ArrowLongRightIcon } from "@heroicons/vue/24/outline";
import { computed, nextTick, ref, type Ref } from "vue";
const context = useStatementContext();

const inputNodes = computed(
  () =>
    context.typeNodes.value
      ?.filter((n) => !n.isOutput)
      .sort((a, b) => (a.orderKey < b.orderKey ? -1 : 1))
      .map((n) => n as SimpleTypeNode) ?? []
);
const lastNode = computed(() => context.typeNodes.value?.[context.typeNodes.value.length - 1]);
const outputNode = computed(() => context.typeNodes.value?.map((n) => n as SimpleTypeNode).find((n) => n.isOutput));
const hasOutput = computed(() => outputNode.value != null && outputNode.value.tag != TypeTag.Null);

const emit = defineEmits<{
  (e: "navigateUp"): void;
  (e: "navigateDown"): void;
  (e: "navigateRight"): void;
  (e: "navigateLeft"): void;
}>();

const addInputRef: Ref<HTMLButtonElement | null> = ref(null);
const outputRef: Ref<HTMLButtonElement | InstanceType<typeof InlineTypeCell> | null> = ref(null);
const inputGrid = useNavigationGrid<string, InstanceType<typeof InlineTypeCell>>(
  computed(() => ["name", "type"]),
  inputNodes,
  {
    gridNavigateUp: () => emit("navigateUp"),
    gridNavigateDown: () => emit("navigateDown"),
    gridNavigateLeft: () => emit("navigateLeft"),
    gridNavigateRight: () => addInputRef.value?.focus(),
  }
);
const isEditing = computed(() => inputGrid.refs.value.find((r) => r.editing) && !(outputRef.value as any)?.editing);

async function insertInput() {
  await context.createTypeNode(
    makeTypeNode({
      name: "input" + inputNodes.value?.length,
      tag: TypeTag.String,
      orderKey: generateKeyBetween(lastNode.value?.orderKey ?? INTEGER_ZERO, null),
    })
  );
  nextTick(() => inputGrid.focus(-1, "name"));
}

async function insertOutput(node: Partial<SimpleType> = {}) {
  if (outputNode.value != null) {
    return;
  }
  await context.createTypeNode(
    makeTypeNode({
      tag: TypeTag.Any,
      orderKey: generateKeyBetween(lastNode.value?.orderKey ?? INTEGER_ZERO, null),
      isOutput: true,
      ...node,
    } as any)
  );
  nextTick(() => outputRef?.value?.focus());
}

async function deleteNode(node: SimpleTypeNode) {
  const inputIndex = inputNodes.value.findIndex((n) => n.id == node.id);
  if (node.name == "output" || inputIndex <= 0) {
    addInputRef.value?.focus();
  } else {
    inputGrid.focus(inputIndex - 1, "name");
  }
  await context.deleteTypeNode(node);
}

function updateInputName(node: SimpleTypeNode, name: string) {
  context.updateTypeNode(node, { ...node, name });
}

function updateInputType(oldNode: SimpleTypeNode, newNode: SimpleType) {
  context.updateTypeNode(oldNode, newNode);
}

function updateOutputType(newNode: SimpleType) {
  if (outputNode.value == null) {
    insertOutput(newNode);
  } else {
    context.updateTypeNode(outputNode.value, { ...newNode, orderKey: outputNode.value.orderKey, isOutput: true });
  }
}

async function nullOutputType() {
  if (!isEditing.value && outputNode.value != null) {
    await context.updateTypeNode(outputNode.value, { ...outputNode.value, tag: TypeTag.Null });
    nextTick(() => outputRef.value?.focus());
  }
}

function focusLastInputOrNavigateLeft() {
  if (inputGrid.refs.value.length > 0) {
    inputGrid.focus(-1, "type");
  } else {
    emit("navigateLeft");
  }
}

function focus() {
  if (inputGrid.refs.value.length > 0) {
    inputGrid.focus(0, "name");
  } else {
    addInputRef.value?.focus();
  }
}

defineExpose({
  focus,
  blur: () => {
    addInputRef.value?.blur();
    inputGrid.blur();
    outputRef.value?.blur();
  },
});
</script>
<template>
  <div class="flex flex-row gap-2">
    <!-- Inputs  -->
    <span
      v-for="inputNode in inputNodes"
      :key="inputNode.id"
      class="inline-flex gap-1 focus-within:bg-orange-100"
      @keydown.delete.exact="isEditing || deleteNode(inputNode)"
    >
      <InlineValueCell
        :ref="(el: any) => inputGrid.registerColumnRef(inputNode.id, 'name', el)"
        :model-value="inputNode.name ?? ''"
        @update:model-value="(name) => updateInputName(inputNode, name)"
        :type="STRING_TYPE_NODE"
        :readonly="context.readonly.value"
        :editing="false"
        :immediate="false"
        slim
        @navigate-up="emit('navigateUp')"
        @navigate-down="emit('navigateDown')"
        @navigate-right="inputGrid.navigateRight(inputNode.id, 'name')"
        @navigate-left="inputGrid.navigateLeft(inputNode.id, 'name')"
        class="rounded-sm border border-transparent focus-within:border-dashed focus-within:border-gray-700 focus-within:bg-orange-100"
      />
      <!-- Note :EditableCellStyle (should be symmetric) -->
      <InlineTypeCell
        :ref="(el: any) => inputGrid.registerColumnRef(inputNode.id, 'type', el)"
        :model-value="inputNode"
        @update:model-value="(node: any) => updateInputType(inputNode, node)"
        :readonly="context.readonly.value"
        :editing="false"
        @navigate-up="emit('navigateUp')"
        @navigate-down="emit('navigateDown')"
        @navigate-right="inputGrid.navigateRight(inputNode.id, 'type')"
        @navigate-left="inputGrid.navigateLeft(inputNode.id, 'type')"
        class="rounded-sm border border-transparent text-gray-400 focus-within:border-dashed focus-within:border-gray-700 focus-within:bg-orange-100"
      />
    </span>
    <!-- Add input button -->
    <button
      ref="addInputRef"
      tabindex="-1"
      v-if="!context.readonly.value && context.focused.value"
      @keydown.left.exact.prevent="focusLastInputOrNavigateLeft"
      @keydown.right.exact.prevent="outputRef?.focus()"
      @keydown.up.exact.prevent="emit('navigateUp')"
      @keydown.down.exact.prevent="emit('navigateDown')"
      @keydown.enter.exact.prevent="insertInput"
      @click="insertInput"
      class="-ml-1 w-fit select-none rounded-sm px-0.5 text-gray-400 outline-none hover:bg-orange-100 hover:text-gray-700 focus:bg-orange-100"
    >
      +input
    </button>
    <!-- Add output button (if no outputs) -->
    <button
      ref="outputRef"
      v-if="!context.readonly.value && !hasOutput && context.focused.value"
      @keydown.left.exact.prevent="addInputRef?.focus()"
      @keydown.right.exact.prevent="outputRef?.focus()"
      @keydown.up.exact.prevent="emit('navigateUp')"
      @keydown.down.exact.prevent="emit('navigateDown')"
      @keydown.enter.exact.prevent="insertOutput()"
      tabindex="-1"
      @click="insertOutput()"
      class="relative w-fit select-none items-baseline rounded-sm px-0.5 pl-5 text-gray-400 outline-none hover:bg-orange-100 hover:text-gray-700 focus:bg-orange-100"
    >
      <ArrowLongRightIcon v-if="!hasOutput" class="absolute left-0 top-0.5 h-4 w-4 text-gray-600" />
      +output
    </button>
    <!-- Output type -->
    <span class="relative pl-5" v-if="hasOutput && outputNode != null">
      <ArrowLongRightIcon v-if="hasOutput" class="absolute left-0 top-1 h-4 w-4 text-gray-600" />
      <InlineTypeCell
        ref="outputRef"
        :model-value="outputNode"
        @update:model-value="(node: any) => updateOutputType(node)"
        @keydown.delete.exact="isEditing || nullOutputType"
        :readonly="context.readonly.value"
        :editing="false"
        @navigate-left="addInputRef?.focus()"
        @navigate-up="emit('navigateUp')"
        @navigate-down="emit('navigateDown')"
        @navigate-right="emit('navigateRight')"
        class="rounded-sm border border-transparent focus-within:border-dashed focus-within:border-gray-700 focus-within:bg-orange-100"
      />
    </span>
  </div>
</template>
