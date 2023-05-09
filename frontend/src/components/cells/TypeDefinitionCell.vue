<script lang="ts" setup>
import InlineActions from "@/components/basic/InlineActions.vue";
import DeclarationCell from "@/components/cells/DeclarationCell.vue";
import { useNavigationGrid } from "@/components/cells/grid";
import InlineTypeCell from "@/components/cells/InlineTypeCell.vue";
import InlineValueCell from "@/components/cells/InlineValueCell.vue";
import EditableSpan from "@/components/EditableSpan.vue";
import { makeTypeNode, STRING_TYPE_NODE, useStatementContext, type SimpleType } from "@/components/statement";
import { TypeTag, type SimpleTypeNode } from "@/gql/graphql";
import { generateKeyBetween } from "@/utils/fractional";
import { computed, nextTick, ref, type Ref } from "vue";

const context = useStatementContext();
const declarationRef: Ref<InstanceType<typeof DeclarationCell> | null> = ref(null);
const description: Ref<string> = ref(context.statement.value.description ?? "");
const descriptionRef: Ref<InstanceType<typeof EditableSpan> | null> = ref(null);
context.syncDescription(
  description,
  computed(() => descriptionRef.value?.focused)
);

const isEnum = computed(() => context.typeRootTag.value == TypeTag.Enum);
const isStruct = computed(() => context.typeRootTag.value == TypeTag.Struct);
const members = computed(() => context.typeNodes.value ?? []);
const membersLength = computed(() => members.value?.length ?? 0);
const addMemberRef: Ref<HTMLButtonElement | null> = ref(null);

// dynamic member refs for names, values & descriptions for each member
type ColumnType = "name" | "value" | "type" | "description";
const columnsInOrder: Ref<ColumnType[]> = computed(() => {
  if (isEnum.value) {
    return ["name", "description"];
  } else if (isStruct.value) {
    return ["name", "type", "description"];
  } else {
    throw new Error("unexpected type node tag: " + context.typeRootTag.value);
  }
});
const grid = useNavigationGrid<ColumnType, InstanceType<typeof InlineTypeCell>>(columnsInOrder, members, {
  gridNavigateUp,
  gridNavigateDown,
});
const isEditing = computed(() => grid.refs.value.find((n) => n.editing));

function insertBelow(memberId?: string) {
  let orderKey;
  if (memberId == null) {
    const lastMember = members.value?.[membersLength.value - 1];
    orderKey = generateKeyBetween(lastMember?.orderKey ?? null, null);
  } else {
    const member = members.value?.find((m) => m.id === memberId);
    orderKey = generateKeyBetween(member?.orderKey ?? null, null);
  }

  let newMemberNode;
  if (isEnum.value) {
    const name = "Option " + (membersLength.value + 1);
    newMemberNode = makeTypeNode({
      name,
      tag: TypeTag.Literal,
      value: name, // :LiteralStringEnum
      orderKey,
    });
  } else {
    newMemberNode = makeTypeNode({
      name: "field " + (membersLength.value + 1),
      tag: TypeTag.String,
      orderKey,
    });
  }

  context.createTypeNode(newMemberNode);
  nextTick(() => grid.focus(membersLength.value - 1, "name"));
}

function readColumn(member: SimpleTypeNode, column: ColumnType) {
  if (column == "type") {
    return member;
  } else {
    return member[column];
  }
}

function writeColumn(memberId: string, column: ColumnType, value: any) {
  const member = members.value?.find((m) => m.id === memberId);
  if (!member) {
    return;
  }
  if (column == "type") {
    context.updateTypeNode(member as SimpleType, value as SimpleType);
  } else if (column == "name" && isEnum.value) {
    // copy name over to value for literal string enums :LiteralStringEnum
    context.updateTypeNode(member as SimpleType, { ...member, [column]: value, value: value } as SimpleType);
  } else {
    context.updateTypeNode(member as SimpleType, { ...member, [column]: value } as SimpleType);
  }
}

function deleteMember(memberId: string) {
  const memberIdx = members.value?.findIndex((m) => m.id === memberId);
  if (memberIdx == null || memberIdx < 0) {
    return;
  }
  const member = members.value?.[memberIdx];
  context.deleteTypeNode(member as any); // must exist
  grid.focus(memberIdx - 1, "name"); // move focus above
}

function focusFirstIfExists() {
  if (membersLength.value == 0) {
    addMemberRef.value?.focus();
  } else {
    grid.focus(0, columnsInOrder.value[0]);
  }
}

function focusLast() {
  if (membersLength.value > 0) {
    grid.focus(membersLength.value - 1, columnsInOrder.value[0]);
  } else {
    descriptionRef.value?.focus();
  }
}

function gridNavigateUp() {
  descriptionRef.value?.focus();
}

function gridNavigateDown() {
  addMemberRef.value?.focus();
}

defineExpose({
  focus: () => declarationRef.value?.focus(),
  blur: () => {
    declarationRef.value?.blur();
    descriptionRef.value?.blur();
    addMemberRef.value?.blur();
    grid.blur();
  },
});
</script>
<template>
  <!-- Declaration -->
  <div class="flex items-center justify-between">
    <DeclarationCell
      ref="declarationRef"
      @navigate-down="descriptionRef?.focus()"
      @navigate-right="descriptionRef?.focus()"
    />
    <InlineActions
      class="transition duration-150 group-hover/statement:opacity-100"
      :class="context.focused.value ? '' : 'opacity-0'"
    />
  </div>
  <!-- Description -->
  <EditableSpan
    ref="descriptionRef"
    v-model="description"
    :readonly="context.readonly.value"
    @navigate-left="declarationRef?.focus()"
    @navigate-up="declarationRef?.focus()"
    @navigate-down="focusFirstIfExists"
  />
  <button
    tabindex="-1"
    v-if="description.length == 0 && !context.readonly.value"
    @click="descriptionRef?.focus()"
    class="w-fit rounded-sm px-0.5 text-gray-300 hover:bg-orange-100 hover:text-gray-700 group-focus-within/statement:text-gray-400"
  >
    +description
  </button>
  <!-- Members (enum options or struct fields) -->
  <div
    class="my-1 grid w-fit"
    :class="{
      'grid-cols-[minmax(40px,auto)_minmax(160px,1fr)]': isEnum,
      'grid-cols-[minmax(40px,auto)_120px_minmax(160px,1fr)]': isStruct,
    }"
  >
    <!-- Rows -->
    <template v-for="member of members" :key="member.id">
      <!-- Columns -->
      <template v-for="column in columnsInOrder" :key="member.id + '.' + column">
        <!-- Individual column: a bit messy -->
        <component
          :is="column == 'type' ? InlineTypeCell : InlineValueCell"
          :model-value="readColumn(member as SimpleTypeNode, column)"
          @update:model-value="(val: any) => writeColumn(member.id, column, val)"
          :ref="(el: any) => grid.registerColumnRef(member.id, column, el)"
          :readonly="context.readonly.value"
          :active="context.focused.value || context.editing.value"
          immediate
          debounced
          :placeholder-value="context.editing.value ? '+' + column : null"
          :type="STRING_TYPE_NODE"
          slim
          @navigate-left="grid.navigateLeft(member.id, column)"
          @navigate-right="grid.navigateRight(member.id, column)"
          @navigate-up="grid.navigateUp(member.id, column)"
          @navigate-down="grid.navigateDown(member.id, column)"
          @delete-left="deleteMember(member.id)"
          @keydown.delete.exact="isEditing || deleteMember(member.id)"
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
      ref="addMemberRef"
      class="w-fit select-none rounded-sm px-0.5 text-gray-300 outline-none hover:bg-orange-100 hover:text-gray-700 focus:bg-orange-100 group-focus-within/statement:text-gray-400"
      @click="insertBelow()"
      @enter="insertBelow()"
      @keydown.up.exact="focusLast"
      @keydown.down.exact="context.navigateDown"
    >
      +{{ isEnum ? "option" : "field" }}
    </button>
  </div>
</template>
