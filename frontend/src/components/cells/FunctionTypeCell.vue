<script lang="ts" setup>
import { useNavigationGrid } from "@/composables/useGrid";
import TypeTupleInterface from "@/components/interfaces/TypeTupleInterface.vue";
import ValueInterface from "@/components/interfaces/ValueInterface.vue";
import { makeField, NAME_FIELD, useStatementContext } from "@/state/statement";
import { TypeTag, type Field } from "@/gql/graphql";
import { TypeFlag } from "@/state/module";
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

const nodes = computed(() => context.fields.value ?? []);
const inputNodes = computed(
  () => context.fields.value?.filter((n) => !(n.flags & TypeFlag.IsOutput)).map((n) => n as Field) ?? []
);
const outputNodes = computed(
  () => context.fields.value?.filter((n) => n.flags & TypeFlag.IsOutput).map((n) => n as Field) ?? []
);

type ColumnType = "type";
const columnsInOrder: Ref<ColumnType[]> = ref(["type"] as ColumnType[]);
const inputGrid = useNavigationGrid<string, InstanceType<typeof TypeTupleInterface>>(columnsInOrder, inputNodes, {
  gridNavigateUp: () => emit("navigateUp"),
  gridNavigateDown: () => addInputRef.value?.focus(),
  gridNavigateRight: (rowIdx) => focusColumn("output", rowIdx, 0),
  nowrapLeft: true,
  nowrapRight: true,
});
const outputGrid = useNavigationGrid<string, InstanceType<typeof TypeTupleInterface>>(columnsInOrder, outputNodes, {
  gridNavigateUp: () => emit("navigateUp"),
  gridNavigateDown: () => addOutputRef.value?.focus(),
  gridNavigateLeft: (rowIdx) => focusColumn("input", rowIdx, -1),
  nowrapLeft: true,
  nowrapRight: true,
});

const addInputRef: Ref<HTMLButtonElement | null> = ref(null);
const addOutputRef: Ref<HTMLButtonElement | null> = ref(null);

function readColumn(member: Field, column: ColumnType) {
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
  const flags = value.flags | (kind == "output" ? TypeFlag.IsOutput : 0);
  if (column == "type") {
    context.updateField(member as Field, { ...value, flags } as Field);
  } else {
    context.updateField(member as Field, { ...member, [column]: value, flags } as Field);
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
  const newMemberNode = makeField({
    name: membersOfKind.length == 0 ? kind : kind + " " + (membersOfKind.length + 1),
    tag: TypeTag.String,
    orderKey,
    flags: kind == "output" ? TypeFlag.IsOutput : 0,
  });
  context.createField(newMemberNode);
  nextTick(() => (kind == "input" ? inputGrid : outputGrid).focus(-1, "type"));
}

function deleteMember(kind: "input" | "output", memberId: string) {
  const members = kind == "input" ? inputNodes.value : outputNodes.value;
  const memberIdx = members.findIndex((m) => m.id === memberId);
  if (memberIdx == null || memberIdx < 0) {
    return;
  }
  const member = members[memberIdx];
  context.deleteField(member as any); // must exist
  (kind == "input" ? inputGrid : outputGrid).focus(memberIdx - 1, "type"); // move focus above
}

function focus(what: "first" | "last", kind: "input" | "output") {
  const nodes = kind == "input" ? inputNodes.value : outputNodes.value;
  if (nodes.length == 0) {
    // focus add button
    (kind == "input" ? addInputRef : addOutputRef).value?.focus();
  } else {
    // focus first/last
    (kind == "input" ? inputGrid : outputGrid).focus(what == "first" ? 0 : -1, "type");
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
  focus: (position: "first" | "last") => {
    if (position == "first") {
      focus("first", "input");
    } else if (addInputRef.value != null) {
      addInputRef.value.focus();
    } else {
      focus("last", "input");
    }
  },
  blur: () => {
    inputGrid.blur();
    outputGrid.blur();
    addInputRef.value?.blur();
    addOutputRef.value?.blur();
  },
});
</script>
<template>
  <div class="flex w-full flex-row flex-wrap items-start gap-4">
    <!-- Inputs -->
    <div class="-mx-1 my-1 flex h-fit w-fit flex-col gap-0.5">
      <template v-for="member of inputNodes" :key="member.id">
        <TypeTupleInterface
          :ref="(el: any) => inputGrid.registerColumnRef(member.id, 'type', el)"
          :model-value="readColumn(member as Field, 'type')"
          @update:model-value="(val: any) => writeColumn('input', member.id, 'type', val)"
          :readonly="context.readonly.value"
          :active="context.focused.value || context.editing.value"
          immediate
          debounced
          :placeholder-value="context.editing.value ? '+' + 'type' : null"
          :type="NAME_FIELD"
          slim
          @navigate-left="inputGrid.navigateLeft(member.id, 'type')"
          @navigate-right="inputGrid.navigateRight(member.id, 'type')"
          @navigate-up="inputGrid.navigateUp(member.id, 'type')"
          @navigate-down="inputGrid.navigateDown(member.id, 'type')"
          @delete-left="deleteMember('input', member.id)"
          @delete-self="deleteMember('input', member.id)"
          class="w-full self-start border border-transparent px-1 py-0.5 text-gray-400 focus-within:border-solid focus-within:border-orange-900 focus-within:border-opacity-[15%] focus-within:bg-orange-100 hover:bg-orange-100"
        />
      </template>
      <!-- Add a member -->
      <button
        v-show="!context.readonly.value"
        tabindex="-1"
        ref="addInputRef"
        class="w-fit select-none rounded-sm px-0.5 text-gray-300 outline-none hover:bg-orange-100 hover:text-gray-700 focus:bg-orange-100 group-focus-within/statement:text-gray-400"
        @click="insertBelow('input')"
        @enter="insertBelow('input')"
        @keydown.up.exact.prevent="inputNodes.length > 0 ? focus('last', 'input') : $emit('navigateUp')"
        @keydown.down.exact.prevent="context.navigateDown"
        @keydown.right.exact.prevent="addOutputRef?.focus"
      >
        +input
      </button>
    </div>
    <!-- Lil' arrow -->
    <ArrowLongRightIcon class="mt-2 h-4 w-4 text-gray-700" />
    <!-- Outputs -->
    <div class="-mx-1 my-1 flex h-fit w-fit flex-col gap-0.5">
      <template v-for="member of outputNodes" :key="member.id">
        <TypeTupleInterface
          :ref="(el: any) => outputGrid.registerColumnRef(member.id, 'type', el)"
          :is="'type' == 'type' ? TypeTupleInterface : ValueInterface"
          :model-value="readColumn(member as Field, 'type')"
          @update:model-value="(val: any) => writeColumn('output', member.id, 'type', val)"
          :readonly="context.readonly.value"
          :active="context.focused.value || context.editing.value"
          immediate
          debounced
          :placeholder-value="context.editing.value ? '+' + 'type' : null"
          :type="NAME_FIELD"
          @navigate-left="outputGrid.navigateLeft(member.id, 'type')"
          @navigate-right="outputGrid.navigateRight(member.id, 'type')"
          @navigate-up="outputGrid.navigateUp(member.id, 'type')"
          @navigate-down="outputGrid.navigateDown(member.id, 'type')"
          @delete-left="deleteMember('output', member.id)"
          @delete-self="deleteMember('output', member.id)"
          class="w-full self-start border border-transparent px-1 py-0.5 text-gray-400 focus-within:border-solid focus-within:border-orange-900 focus-within:border-opacity-[15%] focus-within:bg-orange-100 hover:bg-orange-100"
        />
      </template>
      <!-- Add a member -->
      <button
        v-if="!context.readonly.value"
        tabindex="-1"
        ref="addOutputRef"
        class="w-fit select-none rounded-sm px-0.5 text-gray-300 outline-none hover:bg-orange-100 hover:text-gray-700 focus:bg-orange-100 group-focus-within/statement:text-gray-400"
        @click="insertBelow('output')"
        @enter="insertBelow('output')"
        @keydown.up.exact.prevent="outputNodes.length > 0 ? focus('last', 'output') : $emit('navigateUp')"
        @keydown.down.exact.prevent="context.navigateDown"
        @keydown.left.exact.prevent="addInputRef?.focus"
      >
        +output
      </button>
    </div>
  </div>
</template>
