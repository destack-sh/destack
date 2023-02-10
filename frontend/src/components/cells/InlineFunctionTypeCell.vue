<script lang="ts" setup>
import { useNavigationGrid } from "@/components/cells/grid";
import InlineTypeCell from "@/components/cells/InlineTypeCell.vue";
import InlineValueCell from "@/components/cells/InlineValueCell.vue";
import { makeTypeNodeData, STRING_TYPE_NODE, useStatementContext } from "@/components/statement";
import { TypeTag, type TypeNodeData } from "@/gql/graphql";
import { generateKeyBetween, INTEGER_ZERO } from "@/utils/fractional";
import { ArrowLongRightIcon } from "@heroicons/vue/24/outline";
import { computed, nextTick, ref, type Ref } from "vue";
const context = useStatementContext();

const inputNode = computed(() => context.typeNodesChildren.value?.find((n) => n.name === "input"));
const inputNodes = computed(
  () =>
    context.typeNodes.value
      ?.filter((n) => n.parentId == inputNode.value?.id)
      .sort((a, b) => (a.orderKey < b.orderKey ? -1 : 1)) ?? []
);
const lastInputNode = computed(() => inputNodes.value?.[inputNodes.value.length - 1]);
const outputNode = computed(() => context.typeNodesChildren.value?.find((n) => n.name === "output"));
const hasOutput = computed(() => outputNode.value != null && outputNode.value.tag != TypeTag.Null);

const emit = defineEmits<{
  (e: "navigateUp"): void;
  (e: "navigateDown"): void;
  (e: "navigateRight"): void;
  (e: "navigateLeft"): void;
}>();

const addInputRef: Ref<HTMLButtonElement | null> = ref(null);
const outputRef: Ref<HTMLButtonElement | null> = ref(null);
const inputGrid = useNavigationGrid<string, InstanceType<typeof InlineTypeCell>>(
  computed(() => ["name", "type"]),
  inputNodes,
  () => emit("navigateUp"),
  () => emit("navigateDown"),
  () => emit("navigateLeft"),
  () => addInputRef.value?.focus()
);

async function insertInput() {
  if (outputNode.value == null) {
    throw new Error("invalid function: input node is null");
  }
  await context.createTypeNode(
    makeTypeNodeData({
      name: "input" + inputNodes.value?.length,
      tag: TypeTag.String,
      parentId: inputNode.value?.id,
      orderKey: generateKeyBetween(lastInputNode.value?.orderKey ?? INTEGER_ZERO, null),
    })
  );
  nextTick(() => inputGrid.focus(-1, "name"));
}

async function insertOutput() {
  if (outputNode.value == null) {
    throw new Error("invalid function: output node is null");
  }
  await context.updateTypeNode({ ...outputNode.value, tag: TypeTag.Any });
  nextTick(() => outputRef?.value?.focus());
}

async function deleteNodeIfNotEditing(node: TypeNodeData) {
  if (!inputGrid.refs.value.find((r) => r.editing) && !outputRef.value?.editing) {
    const inputIndex = inputNodes.value.findIndex((n) => n.id == node.id);
    if (node.name == "output" || inputIndex <= 0) {
      addInputRef.value?.focus();
    } else {
      inputGrid.focus(inputIndex - 1, "name");
    }
    await context.deleteTypeNode(node);
  }
}

async function nullNodeIfNotEditing(node: TypeNodeData) {
  if (!inputGrid.refs.value.find((r) => r.editing) && !outputRef.value?.editing) {
    await context.updateTypeNode({ ...node, tag: TypeTag.Null });
    nextTick(() => outputRef.value?.focus());
  }
}

function updateNodeName(node: TypeNodeData, name: string) {
  const updatedNode = {
    ...node,
    name,
  };
  context.updateTypeNode(updatedNode);
}

function updateNodeType(inputNode: TypeNodeData, changed: Pick<TypeNodeData, "tag" | "reference">) {
  const updatedMember = {
    ...inputNode,
    tag: changed.tag,
    reference: changed.reference,
  };
  context.updateTypeNode(updatedMember);
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
      class="inline-flex gap-1 focus-within:bg-orange-50"
      @keydown.delete.exact="deleteNodeIfNotEditing(inputNode)"
    >
      <InlineValueCell
        :ref="(el: any) => inputGrid.registerColumnRef(inputNode.id, 'name', el)"
        :model-value="inputNode.name ?? ''"
        @update:model-value="(name) => updateNodeName(inputNode, name)"
        :type="STRING_TYPE_NODE"
        :readonly="context.readonly.value"
        :editing="false"
        :immediate="false"
        @navigate-up="emit('navigateUp')"
        @navigate-down="emit('navigateDown')"
        @navigate-right="inputGrid.navigateRight(inputNode.id, 'name')"
        @navigate-left="inputGrid.navigateLeft(inputNode.id, 'name')"
        class="rounded-sm border border-transparent py-0.5 focus-within:border-dashed focus-within:border-gray-700 focus-within:bg-orange-50"
      />
      <!-- Note :EditableCellStyle (should be symmetric) -->
      <InlineTypeCell
        :ref="(el: any) => inputGrid.registerColumnRef(inputNode.id, 'type', el)"
        :model-value="inputNode"
        @update:model-value="(node) => updateNodeType(inputNode, node)"
        :readonly="context.readonly.value"
        :editing="false"
        @navigate-up="emit('navigateUp')"
        @navigate-down="emit('navigateDown')"
        @navigate-right="inputGrid.navigateRight(inputNode.id, 'type')"
        @navigate-left="inputGrid.navigateLeft(inputNode.id, 'type')"
        class="rounded-sm border border-transparent py-0.5 focus-within:border-dashed focus-within:border-gray-700 focus-within:bg-orange-50"
      />
    </span>
    <!-- Add input button -->
    <button
      ref="addInputRef"
      tabindex="-1"
      v-if="context.focused.value"
      @keydown.left.exact.prevent="focusLastInputOrNavigateLeft"
      @keydown.right.exact.prevent="outputRef?.focus()"
      @keydown.up.exact.prevent="emit('navigateUp')"
      @keydown.down.exact.prevent="emit('navigateDown')"
      @keydown.enter.exact.prevent="insertInput"
      @click="insertInput"
      class="-ml-1 w-fit rounded-sm px-0.5 text-gray-400 outline-none hover:bg-orange-50 hover:text-gray-700 focus:bg-orange-50"
    >
      +input
    </button>
    <!-- Add output button (if no outputs) -->
    <button
      ref="outputRef"
      v-if="!hasOutput && context.focused.value"
      @keydown.left.exact.prevent="addInputRef?.focus()"
      @keydown.right.exact.prevent="outputRef?.focus()"
      @keydown.up.exact.prevent="emit('navigateUp')"
      @keydown.down.exact.prevent="emit('navigateDown')"
      @keydown.enter.exact.prevent="insertOutput"
      tabindex="-1"
      @click="insertOutput"
      class="relative w-fit items-baseline rounded-sm px-0.5 pl-5 text-gray-400 outline-none hover:bg-orange-50 hover:text-gray-700 focus:bg-orange-50"
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
        @update:model-value="(node) => updateNodeType(outputNode, node)"
        @keydown.delete.exact="nullNodeIfNotEditing(outputNode)"
        :readonly="context.readonly.value"
        :editing="false"
        @navigate-left="addInputRef?.focus()"
        @navigate-up="emit('navigateUp')"
        @navigate-down="emit('navigateDown')"
        @navigate-right="emit('navigateRight')"
        class="rounded-sm border border-transparent py-0.5 focus-within:border-dashed focus-within:border-gray-700 focus-within:bg-orange-50"
      />
    </span>
  </div>
</template>
