<script lang="ts" setup>
import { useNavigationGrid } from "@/composables/useGrid";
import FieldInterface from "@/components/interfaces/FieldInterface.vue";
import ValueInterface from "@/components/interfaces/ValueInterface.vue";
import CreateFieldInterface from "@/components/interfaces/CreateFieldInterface.vue";
import { makeField, useStatementContext } from "@/state/statement";
import type { Field } from "@/gql/graphql";
import { TypeFlag } from "@/state/module";
import { generateKeyBetween } from "@/utils/fractional";
import { ArrowLongRightIcon, PlusIcon } from "@heroicons/vue/24/outline";
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
const inputGrid = useNavigationGrid<string, InstanceType<typeof FieldInterface>>(columnsInOrder, inputNodes, {
  gridNavigateUp: () => emit("navigateUp"),
  gridNavigateDown: () => addInputRef.value?.focus(),
  gridNavigateRight: (rowIdx) => focusColumn("output", rowIdx, 0),
  nowrapLeft: true,
  nowrapRight: true,
});
const outputGrid = useNavigationGrid<string, InstanceType<typeof FieldInterface>>(columnsInOrder, outputNodes, {
  gridNavigateUp: () => emit("navigateUp"),
  gridNavigateDown: () => addOutputRef.value?.focus(),
  gridNavigateLeft: (rowIdx) => focusColumn("input", rowIdx, -1),
  nowrapLeft: true,
  nowrapRight: true,
});

const addInputRef: Ref<HTMLButtonElement | null> = ref(null);
const createInputRef: Ref<InstanceType<typeof CreateFieldInterface> | null> = ref(null);
const addOutputRef: Ref<HTMLButtonElement | null> = ref(null);
const createOutputRef: Ref<InstanceType<typeof CreateFieldInterface> | null> = ref(null);

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

function insertBelow(
  kind: "input" | "output",
  template: Pick<Field, "tag" | "hint" | "flags" | "reference" | "metadata">
) {
  const lastMember = nodes.value[nodes.value.length - 1];
  const orderKey = generateKeyBetween(lastMember?.orderKey ?? null, null);
  const newMemberNode = makeField({
    ...template,
    reference: template.reference as any,
    orderKey,
    flags: (kind == "output" ? TypeFlag.IsOutput : 0) | (template.flags ?? 0),
  });
  context.createNewField(newMemberNode);
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
  focus: (position: "first" | "last" = "first") => {
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
  createInput: () => {
    createInputRef.value?.show();
  },
  createOutput: () => {
    createOutputRef.value?.show();
  },
});
</script>
<template>
  <div class="flex w-full flex-row flex-wrap items-start gap-4">
    <!-- Inputs -->
    <div class="-mx-1 flex h-fit w-fit flex-1 flex-col gap-0.5">
      <template v-for="member of inputNodes" :key="member.id">
        <FieldInterface
          :ref="(el: any) => inputGrid.registerColumnRef(member.id, 'type', el)"
          :model-value="readColumn(member as Field, 'type')"
          @update:model-value="(val: any) => writeColumn('input', member.id, 'type', val)"
          :readonly="context.readonly.value"
          tuple-name="input"
          @navigate-left="inputGrid.navigateLeft(member.id, 'type')"
          @navigate-right="inputGrid.navigateRight(member.id, 'type')"
          @navigate-up="inputGrid.navigateUp(member.id, 'type')"
          @navigate-down="inputGrid.navigateDown(member.id, 'type')"
          @delete-left="deleteMember('input', member.id)"
          @delete-self="deleteMember('input', member.id)"
          class="w-full self-start px-1 py-1 text-gray-400 focus-within:bg-orange-100 hover:bg-orange-100"
        />
      </template>
      <!-- Add a field -->
      <button
        v-show="!context.readonly.value"
        tabindex="-1"
        ref="addInputRef"
        class="mt-0.5 flex w-fit select-none flex-row items-center gap-0.5 rounded-sm px-0.5 text-gray-300 outline-none hover:bg-orange-100 hover:text-gray-700 focus:bg-orange-100 group-focus-within/statement:text-gray-400"
        @click="createInputRef?.show()"
        @enter="createInputRef?.show()"
        @keydown.up.exact.prevent="inputNodes.length > 0 ? focus('last', 'input') : $emit('navigateUp')"
        @keydown.down.exact.prevent="context.navigateDown"
        @keydown.right.exact.prevent="addOutputRef?.focus"
      >
        <PlusIcon class="h-4 w-4" /> Input
        <CreateFieldInterface
          ref="createInputRef"
          :title="'New input to ' + context.statement.value.name"
          @select="insertBelow('input', $event)"
        />
      </button>
    </div>
    <!-- Lil' arrow -->
    <ArrowLongRightIcon class="mt-1 h-5 w-5 text-gray-700" />
    <!-- Outputs -->
    <!-- TODO @Cleanup: outputs are almost exactly like inputs, much duplication -->
    <div class="-mx-1 flex h-fit w-fit flex-1 flex-col gap-0.5">
      <template v-for="member of outputNodes" :key="member.id">
        <FieldInterface
          :ref="(el: any) => outputGrid.registerColumnRef(member.id, 'type', el)"
          :is="'type' == 'type' ? FieldInterface : ValueInterface"
          :model-value="readColumn(member as Field, 'type')"
          @update:model-value="(val: any) => writeColumn('output', member.id, 'type', val)"
          :readonly="context.readonly.value"
          tuple-name="output"
          @navigate-left="outputGrid.navigateLeft(member.id, 'type')"
          @navigate-right="outputGrid.navigateRight(member.id, 'type')"
          @navigate-up="outputGrid.navigateUp(member.id, 'type')"
          @navigate-down="outputGrid.navigateDown(member.id, 'type')"
          @delete-left="deleteMember('output', member.id)"
          @delete-self="deleteMember('output', member.id)"
          class="w-full self-start px-1 py-1 text-gray-400 focus-within:bg-orange-100 hover:bg-orange-100"
        />
      </template>
      <!-- Add a field -->
      <button
        v-if="!context.readonly.value"
        tabindex="-1"
        ref="addOutputRef"
        class="mt-0.5 flex w-fit select-none flex-row items-center gap-0.5 rounded-sm px-0.5 text-gray-300 outline-none hover:bg-orange-100 hover:text-gray-700 focus:bg-orange-100 group-focus-within/statement:text-gray-400"
        @click="createOutputRef?.show()"
        @enter="createOutputRef?.show()"
        @keydown.up.exact.prevent="outputNodes.length > 0 ? focus('last', 'output') : $emit('navigateUp')"
        @keydown.down.exact.prevent="context.navigateDown"
        @keydown.left.exact.prevent="addInputRef?.focus"
      >
        <PlusIcon class="h-4 w-4" /> Output
        <CreateFieldInterface
          ref="createOutputRef"
          :title="'New output of ' + context.statement.value.name"
          @select="insertBelow('output', $event)"
        />
      </button>
    </div>
  </div>
</template>
