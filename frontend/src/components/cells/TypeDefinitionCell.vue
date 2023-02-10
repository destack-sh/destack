<script lang="ts" setup>
import DeclarationCell from "@/components/cells/DeclarationCell.vue";
import InlineTypeCell from "@/components/cells/InlineTypeCell.vue";
import InlineValueCell from "@/components/cells/InlineValueCell.vue";
import EditableSpan from "@/components/EditableSpan.vue";
import { makeTypeNodeData, STRING_TYPE_NODE, useStatementContext } from "@/components/statement";
import { TypeTag, type TypeNodeData } from "@/gql/graphql";
import { generateKeyBetween } from "@/utils/fractional";
import { computed, nextTick, ref, watch, type Ref } from "vue";

const context = useStatementContext();
const declarationRef: Ref<InstanceType<typeof DeclarationCell> | null> = ref(null);
const description: Ref<string> = ref(context.statement.value.description ?? "");
const descriptionRef: Ref<InstanceType<typeof EditableSpan> | null> = ref(null);
context.syncDescription(description);

// enums have a "head type", structs do not
const isEnum = computed(() => context.typeNodeRoot.value?.tag == TypeTag.Enum);
const isStruct = computed(() => context.typeNodeRoot.value?.tag == TypeTag.Struct);
const memberTypeNodes = computed(() => {
  if (isEnum.value) {
    return context.typeNodesChildren.value?.slice(1) ?? [];
  } else if (isStruct.value) {
    return context.typeNodesChildren.value ?? [];
  } else {
    return [];
  }
});
const membersLength = computed(() => memberTypeNodes.value?.length ?? 0);
const addMemberRef: Ref<HTMLButtonElement | null> = ref(null);

// dynamic member refs for names, values & descriptions for each member
type ColumnType = "name" | "value" | "type" | "description";
const columnsInOrder: Ref<ColumnType[]> = computed(() => {
  if (isEnum.value) {
    return ["name", "description"];
  } else if (isStruct.value) {
    return ["name", "type", "description"];
  } else {
    throw new Error("unexpected type node tag: " + context.typeNodeRoot.value?.tag);
  }
});

const columnRefs: Ref<Record<string, InstanceType<typeof InlineValueCell>>> = ref({});

function registerColumnRef(
  memberId: string,
  column: ColumnType,
  ref: InstanceType<typeof InlineValueCell> | undefined
) {
  const columnId = memberId + "." + column;
  if (ref != undefined) {
    columnRefs.value[columnId] = ref;
  } else {
    delete columnRefs.value[columnId];
  }
}

function focus(index: number | string, column: ColumnType) {
  let member;
  if (typeof index == "string") {
    member = memberTypeNodes.value?.find((m) => m.id == index);
  } else {
    member = memberTypeNodes.value?.[index];
  }

  if (!member) {
    console.warn("no member found for index", index);
    return;
  }
  const columnId = member.id + "." + column;
  columnRefs.value?.[columnId]?.focus();
}

function navigateUp(memberId: string, column: ColumnType) {
  const memberIdx = memberTypeNodes.value?.findIndex((m) => m.id === memberId);
  if (!memberIdx) {
    gridNavigateUp();
  } else {
    focus(memberIdx - 1, column);
  }
}

function navigateDown(memberId: string, column: ColumnType) {
  const memberIdx = memberTypeNodes.value?.findIndex((m) => m.id === memberId) ?? 0;
  if (memberIdx == membersLength.value - 1) {
    gridNavigateDown();
  } else {
    focus(memberIdx + 1, column);
  }
}

function navigateRight(memberId: string, column: ColumnType) {
  const memberIdx = memberTypeNodes.value?.findIndex((m) => m.id === memberId) ?? -1;
  const columnIdx = columnsInOrder.value.findIndex((f) => f === column);
  if (columnIdx == columnsInOrder.value.length - 1) {
    if (memberIdx != membersLength.value - 1) {
      focus(memberIdx + 1, columnsInOrder.value[0]);
    }
  } else {
    focus(memberIdx, columnsInOrder.value[columnIdx + 1]);
  }
}

function navigateLeft(memberId: string, column: ColumnType) {
  const memberIdx = memberTypeNodes.value?.findIndex((m) => m.id === memberId) ?? -1;
  const columnIdx = columnsInOrder.value.findIndex((f) => f === column);
  if (columnIdx == 0) {
    if (memberIdx != 0) {
      focus(memberIdx - 1, columnsInOrder.value[columnsInOrder.value.length - 1]);
    }
  } else {
    focus(memberIdx, columnsInOrder.value[columnIdx - 1]);
  }
}

async function insertBelow(memberId?: string) {
  let orderKey;
  if (memberId == null) {
    const lastMember = memberTypeNodes.value?.[membersLength.value - 1];
    orderKey = generateKeyBetween(lastMember?.orderKey ?? null, null);
  } else {
    const member = memberTypeNodes.value?.find((m) => m.id === memberId);
    orderKey = generateKeyBetween(member?.orderKey ?? null, null);
  }

  let newMemberNode;
  if (isEnum.value) {
    newMemberNode = makeTypeNodeData({
      name: "Option " + (membersLength.value + 1),
      tag: TypeTag.Literal,
      orderKey,
      parentId: context.typeNodeRoot.value?.id,
    });
  } else {
    newMemberNode = makeTypeNodeData({
      name: "field " + (membersLength.value + 1),
      tag: TypeTag.String,
      orderKey,
      parentId: context.typeNodeRoot.value?.id,
    });
  }

  await context.createTypeNode(newMemberNode);
  nextTick(() => focus(membersLength.value - 1, "name"));
}

function readColumn(member: TypeNodeData, column: ColumnType) {
  if (column == "type") {
    return member;
  } else {
    return member[column];
  }
}

function writeColumn(memberId: string, column: ColumnType, value: any) {
  const member = memberTypeNodes.value?.find((m) => m.id === memberId);
  if (!member) {
    return;
  }
  if (column == "type") {
    // special case because it touches the underlying type node data
    value = value as TypeNodeData;
    const updatedMember = {
      ...member,
      tag: value.tag,
      reference: value.reference,
    };
    context.updateTypeNode(updatedMember);
  } else {
    const updatedMember = { ...member, [column]: value };
    context.updateTypeNode(updatedMember);
  }
}

const editingColumn: Ref<string | null> = ref(null);

function editColumn(memberId: string, column: ColumnType) {
  console.log("edit column", memberId, column);
  editingColumn.value = memberId + "." + column;
}

function deleteMember(memberId: string) {
  const memberIdx = memberTypeNodes.value?.findIndex((m) => m.id === memberId);
  if (memberIdx == null || memberIdx < 0) {
    return;
  }
  const member = memberTypeNodes.value?.[memberIdx];
  context.deleteTypeNode(member as any); // must exist
  focus(memberIdx - 1, "name"); // move focus above
}

function deleteMemberIfNotEditing(memberId: string) {
  if (editingColumn.value == null) {
    deleteMember(memberId);
  }
}

function focusFirstIfExists() {
  if (membersLength.value == 0) {
    addMemberRef.value?.focus();
  } else {
    focus(0, columnsInOrder.value[0]);
  }
}

function focusLast() {
  if (membersLength.value > 0) {
    focus(membersLength.value - 1, columnsInOrder.value[0]);
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

// stop editing if blured
watch(
  () => context.focused.value,
  () => {
    if (!context.focused.value) {
      editingColumn.value = null;
    }
  }
);

defineExpose({
  focus: () => declarationRef.value?.focus(),
  blur: () => {
    declarationRef.value?.blur();
    descriptionRef.value?.blur();
    addMemberRef.value?.blur();
    Object.values(columnRefs.value).forEach((r) => r.blur());
  },
});
</script>
<template>
  <!-- Declaration -->
  <DeclarationCell
    ref="declarationRef"
    @navigate-down="descriptionRef?.focus()"
    @navigate-right="descriptionRef?.focus()"
  />
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
    v-if="description.length == 0"
    @click="descriptionRef?.focus()"
    class="w-fit rounded-sm px-0.5 text-gray-400 hover:bg-orange-50 hover:text-gray-700"
  >
    +description
  </button>
  <!-- Members (enum options or struct fields) -->
  <div
    class="my-1 grid w-fit gap-x-3"
    :class="{
      'grid-cols-[minmax(80px,160px)_minmax(160px,1fr)]': isEnum,
      'grid-cols-[minmax(80px,160px)_120px_minmax(160px,1fr)]': isStruct,
    }"
  >
    <!-- Not sure whether to include column headers... -->
    <template v-if="false">
      <span v-for="column in columnsInOrder" :key="column" class="text-xs text-gray-400">
        {{ column }}
      </span>
    </template>
    <!-- Rows -->
    <template v-for="member of memberTypeNodes" :key="member.id">
      <!-- Columns -->
      <template v-for="column in columnsInOrder" :key="member.id + '.' + column">
        <!-- Individual column: a bit messy -->
        <component
          :is="column == 'type' ? InlineTypeCell : InlineValueCell"
          :model-value="readColumn(member, column)"
          @update:model-value="(val: any) => writeColumn(member.id, column, val)"
          :ref="(el: any) => registerColumnRef(member.id, column, el)"
          :readonly="context.readonly.value"
          :immediate="false"
          :editing="editingColumn == member.id + '.' + column"
          @edit="editColumn(member.id, column)"
          :placeholder-value="context.editing.value ? '+' + column : null"
          :type="STRING_TYPE_NODE"
          @navigate-left="navigateLeft(member.id, column)"
          @navigate-right="navigateRight(member.id, column)"
          @navigate-up="navigateUp(member.id, column)"
          @navigate-down="navigateDown(member.id, column)"
          @delete-left="deleteMember(member.id)"
          @keydown.delete.exact="deleteMemberIfNotEditing(member.id)"
          @escape="editingColumn = null"
          :class="{
            'w-full self-start rounded-sm border border-transparent py-0.5': true,
            'focus-within:border-dashed focus-within:border-gray-700 focus-within:bg-orange-50':
              editingColumn != member.id + '.' + column,
          }"
        />
        <!-- Note the :EditableCellStyle above (should be symmetric) -->
      </template>
    </template>
    <!-- Add a member -->
    <button
      tabindex="-1"
      ref="addMemberRef"
      class="w-fit rounded-sm px-0.5 text-gray-400 outline-none hover:bg-orange-50 hover:text-gray-700 focus:bg-orange-50"
      @click="insertBelow()"
      @enter="insertBelow()"
      @keydown.up.exact="focusLast"
      @keydown.down.exact="context.navigateDown"
    >
      +{{ isEnum ? "option" : "field" }}
    </button>
  </div>
</template>
