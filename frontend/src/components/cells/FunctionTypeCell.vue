<script lang="ts" setup>
import { useNavigationGrid } from "@/components/cells/grid";
import InlineTypeCell from "@/components/cells/InlineTypeCell.vue";
import InlineValueCell from "@/components/cells/InlineValueCell.vue";
import { makeTypeNode, STRING_TYPE_NODE, useStatementContext, type SimpleType } from "@/components/statement";
import { TypeTag, type SimpleTypeNode } from "@/gql/graphql";
import { generateKeyBetween } from "@/utils/fractional";
import { ArrowLongRightIcon } from "@heroicons/vue/24/outline";
import { computed, nextTick, ref, type Ref } from "vue";

const context = useStatementContext();

const emit = defineEmits<{
  (e: "navigateUp"): void;
  (e: "navigateDown"): void;
  (e: "navigateRight"): void;
  (e: "navigateLeft"): void;
}>();

const nodes = computed(() => context.typeNodes.value ?? []);
const inputNodes = computed(
  () => context.typeNodes.value?.filter((n) => !n.isOutput).map((n) => n as SimpleTypeNode) ?? []
);
const outputNodes = computed(
  () => context.typeNodes.value?.filter((n) => n.isOutput).map((n) => n as SimpleTypeNode) ?? []
);

type ColumnType = "name" | "type" | "description";
const columnsInOrder: Ref<ColumnType[]> = ref(["name", "type", "description"] as ColumnType[]);
const inputGrid = useNavigationGrid<string, InstanceType<typeof InlineTypeCell>>(columnsInOrder, inputNodes, {
  gridNavigateUp: () => emit("navigateUp"),
  gridNavigateDown: () => addInputRef.value?.focus(),
  gridNavigateRight: (rowIdx) => focusColumn("output", rowIdx, 0),
  nowrapLeft: true,
  nowrapRight: true,
});
const outputGrid = useNavigationGrid<string, InstanceType<typeof InlineTypeCell>>(columnsInOrder, outputNodes, {
  gridNavigateUp: () => emit("navigateUp"),
  gridNavigateDown: () => addOutputRef.value?.focus(),
  gridNavigateLeft: (rowIdx) => focusColumn("input", rowIdx, -1),
  nowrapLeft: true,
  nowrapRight: true,
});
const isEditing = computed(
  () => inputGrid.refs.value.find((r) => r.editing) || outputGrid.refs.value.find((r) => r.editing)
);

const addInputRef: Ref<HTMLButtonElement | null> = ref(null);
const addOutputRef: Ref<HTMLButtonElement | null> = ref(null);

function readColumn(member: SimpleTypeNode, column: ColumnType) {
  if (column == "type") {
    return member;
  } else {
    return member[column];
  }
}
function writeColumn(kind: "input" | "output", memberId: string, column: ColumnType, value: any) {
  const member = nodes.value?.find((m) => m.id === memberId);
  if (!member) {
    return;
  }
  if (column == "type") {
    context.updateTypeNode(member as SimpleType, { ...value, isOutput: kind == "output" } as SimpleType);
  } else {
    context.updateTypeNode(
      member as SimpleType,
      { ...member, [column]: value, isOutput: kind == "output" } as SimpleType
    );
  }
}

function insertBelow(kind: "input" | "output", memberId?: string) {
  let orderKey;
  if (memberId == null) {
    const lastMember = nodes.value[nodes.value.length - 1];
    orderKey = generateKeyBetween(lastMember?.orderKey ?? null, null);
  } else {
    const member = nodes.value.find((m) => m.id === memberId);
    orderKey = generateKeyBetween(member?.orderKey ?? null, null);
  }
  const membersOfKind = kind == "input" ? inputNodes.value : outputNodes.value;
  const newMemberNode = makeTypeNode({
    name: membersOfKind.length == 0 ? kind : kind + " " + (membersOfKind.length + 1),
    tag: TypeTag.String,
    orderKey,
    isOutput: kind == "output",
  });
  context.createTypeNode(newMemberNode);
  nextTick(() => (kind == "input" ? inputGrid : outputGrid).focus(-1, "name"));
}

function deleteMember(kind: "input" | "output", memberId: string) {
  const members = kind == "input" ? inputNodes.value : outputNodes.value;
  const memberIdx = members.findIndex((m) => m.id === memberId);
  if (memberIdx == null || memberIdx < 0) {
    return;
  }
  const member = members[memberIdx];
  context.deleteTypeNode(member as any); // must exist
  (kind == "input" ? inputGrid : outputGrid).focus(memberIdx - 1, "name"); // move focus above
}

function focus(what: "first" | "last", kind: "input" | "output") {
  const nodes = kind == "input" ? inputNodes.value : outputNodes.value;
  if (nodes.length == 0) {
    // focus add button
    (kind == "input" ? addInputRef : addOutputRef).value?.focus();
  } else {
    // focus first/last
    (kind == "input" ? inputGrid : outputGrid).focus(what == "first" ? 0 : -1, "name");
  }
}

function focusColumn(kind: "input" | "output", rowIdx: number, columnIdx: number) {
  if (columnIdx < 0) {
    // wrap
    columnIdx = columnIdx + columnsInOrder.value.length;
  }
  const nodes = kind == "input" ? inputNodes.value : outputNodes.value;
  if (rowIdx < nodes.length) {
    (kind == "input" ? inputGrid : outputGrid).focus(rowIdx, columnsInOrder.value[columnIdx]);
  } else if (rowIdx == nodes.length) {
    // focus add button
    (kind == "input" ? addInputRef : addOutputRef).value?.focus();
  } else {
    // ignore?
  }
}

defineExpose({
  focus: () => focus("first", "input"),
  blur: () => {
    inputGrid.blur();
    outputGrid.blur();
    addInputRef.value?.blur();
    addOutputRef.value?.blur();
  },
});
</script>
<template>
  <div class="flex w-full flex-row flex-wrap items-start justify-evenly gap-4">
    <!-- Inputs -->
    <!-- TODO @Cleanup: FunctionTypeCell (input & output) + TypeDefinitionCell + are suspiciously similar -->
    <div class="my-1 grid h-fit w-fit flex-1 grid-cols-[minmax(60px,auto)_minmax(60px,auto)_minmax(60px,1fr)]">
      <!-- Rows -->
      <template v-for="member of inputNodes" :key="member.id">
        <!-- Columns -->
        <template v-for="column in columnsInOrder" :key="member.id + '.' + column">
          <!-- Individual column: a bit messy -->
          <component
            :is="column == 'type' ? InlineTypeCell : InlineValueCell"
            :model-value="readColumn(member as SimpleTypeNode, column)"
            @update:model-value="(val: any) => writeColumn('input', member.id, column, val)"
            :ref="(el: any) => inputGrid.registerColumnRef(member.id, column, el)"
            :readonly="context.readonly.value"
            :active="context.focused.value || context.editing.value"
            immediate
            debounced
            :placeholder-value="context.editing.value ? '+' + column : null"
            :type="STRING_TYPE_NODE"
            slim
            @navigate-left="inputGrid.navigateLeft(member.id, column)"
            @navigate-right="inputGrid.navigateRight(member.id, column)"
            @navigate-up="inputGrid.navigateUp(member.id, column)"
            @navigate-down="inputGrid.navigateDown(member.id, column)"
            @delete-left="deleteMember('input', member.id)"
            @keydown.delete.exact="isEditing || deleteMember('input', member.id)"
            class="w-full self-start border border-transparent py-0.5 pr-2 focus-within:border-solid focus-within:border-gray-700 focus-within:bg-orange-100 hover:bg-orange-100"
            :class="{
              'text-gray-400': column == 'type',
            }"
          />
          <!-- Note the :EditableCellStyle above (should be symmetric) -->
        </template>
      </template>
      <!-- Add a member -->
      <button
        v-show="!context.readonly.value"
        tabindex="-1"
        ref="addInputRef"
        class="w-fit select-none rounded-sm px-0.5 text-gray-300 outline-none hover:bg-orange-100 hover:text-gray-700 focus:bg-orange-100 group-focus-within/statement:text-gray-400"
        @click="insertBelow('input')"
        @enter="insertBelow('input')"
        @keydown.up.exact.prevent="focus('last', 'input')"
        @keydown.down.exact.prevent="context.navigateDown"
        @keydown.right.exact.prevent="addOutputRef?.focus"
      >
        +input
      </button>
    </div>
    <!-- Lil' arrow -->
    <ArrowLongRightIcon class="mt-2 h-4 w-4 text-gray-700" />
    <!-- Outputs -->
    <div class="my-1 grid h-fit w-fit flex-1 grid-cols-[minmax(60px,auto)_minmax(60px,auto)_minmax(60px,1fr)]">
      <!-- Rows -->
      <template v-for="member of outputNodes" :key="member.id">
        <!-- Columns -->
        <template v-for="column in columnsInOrder" :key="member.id + '.' + column">
          <!-- Individual column: a bit messy -->
          <component
            :is="column == 'type' ? InlineTypeCell : InlineValueCell"
            :model-value="readColumn(member as SimpleTypeNode, column)"
            @update:model-value="(val: any) => writeColumn('output', member.id, column, val)"
            :ref="(el: any) => outputGrid.registerColumnRef(member.id, column, el)"
            :readonly="context.readonly.value"
            :active="context.focused.value || context.editing.value"
            immediate
            debounced
            :placeholder-value="context.editing.value ? '+' + column : null"
            :type="STRING_TYPE_NODE"
            slim
            @navigate-left="outputGrid.navigateLeft(member.id, column)"
            @navigate-right="outputGrid.navigateRight(member.id, column)"
            @navigate-up="outputGrid.navigateUp(member.id, column)"
            @navigate-down="outputGrid.navigateDown(member.id, column)"
            @delete-left="deleteMember('output', member.id)"
            @keydown.delete.exact="isEditing || deleteMember('output', member.id)"
            class="w-full self-start border border-transparent py-0.5 pr-2 focus-within:border-solid focus-within:border-gray-700 focus-within:bg-orange-100 hover:bg-orange-100"
            :class="{
              'text-gray-400': column == 'type',
            }"
          />
          <!-- Note the :EditableCellStyle above (should be symmetric) -->
        </template>
      </template>
      <!-- Add a member -->
      <button
        v-if="!context.readonly.value"
        tabindex="-1"
        ref="addOutputRef"
        class="w-fit select-none rounded-sm px-0.5 text-gray-300 outline-none hover:bg-orange-100 hover:text-gray-700 focus:bg-orange-100 group-focus-within/statement:text-gray-400"
        @click="insertBelow('output')"
        @enter="insertBelow('output')"
        @keydown.up.exact.prevent="focus('last', 'output')"
        @keydown.down.exact.prevent="context.navigateDown"
        @keydown.left.exact.prevent="addInputRef?.focus"
      >
        +output
      </button>
    </div>
  </div>
</template>
