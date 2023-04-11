<script lang="ts" setup>
import { ArrowLongRightIcon } from "@heroicons/vue/24/outline";
import { computed, nextTick, ref, type Ref } from "vue";
import InlineTypeCell from "@/components/cells/InlineTypeCell.vue";
import InlineValueCell from "@/components/cells/InlineValueCell.vue";
import { makeTypeNode, STRING_TYPE_NODE, useStatementContext, type SimpleType } from "@/components/statement";
import { TypeTag, type SimpleTypeNode } from "@/gql/graphql";
import { useNavigationGrid } from "@/components/cells/grid";
import { generateKeyBetween } from "@/utils/fractional";

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
  gridNavigateDown: () => emit("navigateDown"),
  gridNavigateLeft: () => emit("navigateLeft"),
});
const outputGrid = useNavigationGrid<string, InstanceType<typeof InlineTypeCell>>(columnsInOrder, outputNodes, {
  gridNavigateUp: () => emit("navigateUp"),
  gridNavigateDown: () => emit("navigateDown"),
  gridNavigateRight: () => emit("navigateRight"),
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
function writeColumn(memberId: string, column: ColumnType, value: any) {
  const member = nodes.value?.find((m) => m.id === memberId);
  if (!member) {
    return;
  }
  if (column == "type") {
    context.updateTypeNode(member as SimpleType, value as SimpleType);
  } else {
    context.updateTypeNode(member as SimpleType, { ...member, [column]: value } as SimpleType);
  }
}

function insertBelow(kind: "input" | "output", memberId?: string) {
  const members = kind == "input" ? inputNodes.value : outputNodes.value;

  let orderKey;
  if (memberId == null) {
    const lastMember = members[members.length - 1];
    orderKey = generateKeyBetween(lastMember?.orderKey ?? null, null);
  } else {
    const member = members.find((m) => m.id === memberId);
    orderKey = generateKeyBetween(member?.orderKey ?? null, null);
  }

  const newMemberNode = makeTypeNode({
    name: "field " + (members.length + 1),
    tag: TypeTag.String,
    orderKey,
  });

  context.createTypeNode(newMemberNode);
  nextTick(() => (kind == "input" ? inputGrid : outputGrid).focus(members.length - 1, "name"));
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

function focusLast(kind: "input" | "output") {
  if (kind == "input") {
    inputGrid.focus(inputNodes.value.length - 1, "name");
  }
}

defineExpose({
  focus: () => focusLast("input"),
  blur: () => {
    inputGrid.blur();
    outputGrid.blur();
    addInputRef.value?.blur();
    addOutputRef.value?.blur();
  },
});
</script>
<template>
  <div class="flex flex-row justify-between">
    <!-- Inputs -->
    <!-- TODO @Cleanup: FunctionTypeCell (input & output) + TypeDefinitionCell are suspiciously similar -->
    <div class="my-1 grid w-fit grid-cols-[minmax(40px,auto)_120px_minmax(160px,1fr)]">
      <!-- Rows -->
      <template v-for="member of inputNodes" :key="member.id">
        <!-- Columns -->
        <template v-for="column in columnsInOrder" :key="member.id + '.' + column">
          <!-- Individual column: a bit messy -->
          <component
            :is="column == 'type' ? InlineTypeCell : InlineValueCell"
            :model-value="readColumn(member as SimpleTypeNode, column)"
            @update:model-value="(val: any) => writeColumn(member.id, column, val)"
            :ref="(el: any) => inputGrid.registerColumnRef(member.id, column, el)"
            :readonly="context.readonly.value"
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
        @keydown.up.exact="focusLast('input')"
        @keydown.down.exact="context.navigateDown"
      >
        +input
      </button>
    </div>
    <ArrowLongRightIcon class="h-4 w-4 text-gray-700" />
    <!-- Outputs -->
    <div class="my-1 grid w-fit grid-cols-[minmax(40px,auto)_120px_minmax(160px,1fr)]">
      <!-- Rows -->
      <template v-for="member of outputNodes" :key="member.id">
        <!-- Columns -->
        <template v-for="column in columnsInOrder" :key="member.id + '.' + column">
          <!-- Individual column: a bit messy -->
          <component
            :is="column == 'type' ? InlineTypeCell : InlineValueCell"
            :model-value="readColumn(member as SimpleTypeNode, column)"
            @update:model-value="(val: any) => writeColumn(member.id, column, val)"
            :ref="(el: any) => outputGrid.registerColumnRef(member.id, column, el)"
            :readonly="context.readonly.value"
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
        v-show="!context.readonly.value"
        tabindex="-1"
        ref="addOutputRef"
        class="w-fit select-none rounded-sm px-0.5 text-gray-300 outline-none hover:bg-orange-100 hover:text-gray-700 focus:bg-orange-100 group-focus-within/statement:text-gray-400"
        @click="insertBelow('output')"
        @enter="insertBelow('output')"
        @keydown.up.exact="focusLast('output')"
        @keydown.down.exact="context.navigateDown"
      >
        +output
      </button>
    </div>
  </div>
</template>
