<script lang="ts" setup>
import { makeTypeNodeData, STRING_TYPE_NODE, useStatementContext } from "@/components/statement";
import { ArrowLongRightIcon } from "@heroicons/vue/24/outline";
import { computed, nextTick, ref, type Ref } from "vue";
import InlineTypeCell from "@/components/cells/InlineTypeCell.vue";
import InlineValueCell from "@/components/cells/InlineValueCell.vue";
import { TypeTag } from "@/gql/graphql";
import { generateKeyBetween, INTEGER_ZERO } from "@/utils/fractional";
import { useNavigationGrid } from "@/components/cells/grid";
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

function insertInput() {
  if (outputNode.value == null) {
    throw new Error("invalid function: input node is null");
  }
  context.createTypeNode(
    makeTypeNodeData({
      name: "input" + inputNodes.value?.length,
      tag: TypeTag.String,
      parentId: inputNode.value?.id,
      orderKey: generateKeyBetween(lastInputNode.value?.orderKey ?? INTEGER_ZERO, null),
    })
  );
}

function insertOutput() {
  if (outputNode.value == null) {
    throw new Error("invalid function: output node is null");
  }
  context.updateTypeNode({ ...outputNode.value, tag: TypeTag.Any });
  nextTick(() => outputRef?.value?.focus());
}

function focusLastInputOrNavigateLeft() {
  if (inputGrid.refs.value.length > 0) {
    inputGrid.focus(-1, "type");
  } else {
    emit("navigateLeft");
  }
}

function isEditingInput(memberId: string, column: string): boolean {
  return inputGrid.getRef(memberId, column)?.editing;
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
    <span v-for="inputNode in inputNodes" :key="inputNode.id" class="inline-flex gap-1 focus-within:bg-orange-50">
      <InlineValueCell
        :ref="(el: any) => inputGrid.registerColumnRef(inputNode.id, 'name', el)"
        :model-value="inputNode.name ?? ''"
        :type="STRING_TYPE_NODE"
        :readonly="context.readonly.value"
        :editing="false"
        :immediate="false"
        @navigate-up="emit('navigateUp')"
        @navigate-down="emit('navigateDown')"
        @navigate-right="inputGrid.navigateRight(inputNode.id, 'name')"
        @navigate-left="inputGrid.navigateLeft(inputNode.id, 'name')"
        :class="{
          'w-full self-start rounded-sm border border-transparent py-0.5': true,
          'focus-within:border-dashed focus-within:border-gray-700 focus-within:bg-orange-50': !isEditingInput(
            inputNode.id,
            'name'
          ),
        }"
      />
      <!-- Note :EditableCellStyle (should be symmetric) -->
      <InlineTypeCell
        :ref="(el: any) => inputGrid.registerColumnRef(inputNode.id, 'type', el)"
        :model-value="inputNode"
        :readonly="context.readonly.value"
        :editing="false"
        @navigate-up="emit('navigateUp')"
        @navigate-down="emit('navigateDown')"
        @navigate-right="inputGrid.navigateRight(inputNode.id, 'type')"
        @navigate-left="inputGrid.navigateLeft(inputNode.id, 'type')"
        :class="{
          'w-full self-start rounded-sm border border-transparent py-0.5': true,
          'focus-within:border-dashed focus-within:border-gray-700 focus-within:bg-orange-50': !isEditingInput(
            inputNode.id,
            'type'
          ),
        }"
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
      class="w-fit rounded-sm px-0.5 text-gray-400 outline-none hover:bg-orange-50 hover:text-gray-700 focus:bg-orange-50"
    >
      +input
    </button>
    <!-- Add output button (if no outputs) -->
    <ArrowLongRightIcon v-if="hasOutput" class="h-4 w-4 px-3" />
    <button
      ref="outputRef"
      v-if="!hasOutput && context.focused.value"
      @keydown.left.exact.prevent="addInputRef?.focus()"
      @keydown.right.exact.prevent=""
      @keydown.up.exact.prevent="emit('navigateUp')"
      @keydown.down.exact.prevent="emit('navigateDown')"
      @keydown.enter.exact.prevent="insertOutput"
      tabindex="-1"
      @click="insertOutput"
      class="inline-flex w-fit items-center rounded-sm px-0.5 text-gray-400 outline-none hover:bg-orange-50 hover:text-gray-700 focus:bg-orange-50"
    >
      <ArrowLongRightIcon v-if="!hasOutput" class="mr-2 h-4 w-4" />
      +output
    </button>
    <!-- Output type -->
    <InlineTypeCell
      ref="outputRef"
      v-else-if="hasOutput && outputNode != null"
      :model-value="outputNode"
      :readonly="context.readonly.value"
      :editing="false"
      @navigate-left="addInputRef?.focus()"
      @navigate-up="emit('navigateUp')"
      @navigate-down="emit('navigateDown')"
      @navigate-right="emit('navigateRight')"
    />
  </div>
</template>
