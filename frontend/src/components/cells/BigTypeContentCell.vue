<script lang="ts" setup>
import InlineValueCell from "@/components/cells/InlineValueCell.vue";
import EditableSpan from "@/components/EditableSpan.vue";
import { makeTypeNodeData, mapToTypeNode, useStatementContext } from "@/components/statement";
import { TypeTag } from "@/gql/graphql";
import { generateKeyBetween } from "@/utils/fractional";
import { useDebounceFn } from "@vueuse/shared";
import { computed, nextTick, ref, type Ref } from "vue";

const context = useStatementContext();
const description: Ref<string> = ref(context.statement.value.description ?? "");
const descriptionRef: Ref<InstanceType<typeof EditableSpan> | null> = ref(null);
context.syncDescription(description);

// enums have a "head type", structs do not
const isEnum = computed(() => context.typeNodeRoot.value?.tag == TypeTag.Enum);
const isStruct = computed(() => context.typeNodeRoot.value?.tag == TypeTag.Struct);
const headTypeNode = computed(() => {
  if (isEnum.value) {
    return context.typeNodesChildren.value?.[0];
  } else {
    return undefined;
  }
});
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

const STRING_TYPE_NODE = mapToTypeNode([makeTypeNodeData({ tag: TypeTag.String })]);

// TODO @Incomplete: track and manage editing column
const editingColumn: Ref<[string, string] | null> = ref(null);
const columnRefs: Ref<Record<string, InstanceType<typeof InlineValueCell>>> = ref({});

function registerColumnRef(
  memberId: string,
  column: ColumnType,
  ref: InstanceType<typeof InlineValueCell> | undefined
) {
  const columnId = memberId + "." + column;
  columnRefs.value[columnId] = ref ?? undefined;
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
    context.navigateUp();
  } else {
    focus(memberIdx - 1, column);
  }
}

function navigateDown(memberId: string, column: ColumnType) {
  const memberIdx = memberTypeNodes.value?.findIndex((m) => m.id === memberId) ?? 0;
  if (memberIdx == membersLength.value - 1) {
    context.navigateDown();
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
      name: "Field " + (membersLength.value + 1),
      tag: TypeTag.String,
      orderKey,
      parentId: context.typeNodeRoot.value?.id,
    });
  }

  await context.createTypeNode(newMemberNode);
  nextTick(() => focus(membersLength.value - 1, "name"));
}

function _updateMemberColumn(memberId: string, column: ColumnType, value: any) {
  const member = memberTypeNodes.value?.find((m) => m.id === memberId);
  if (!member) {
    return;
  }
  console.log("updateMemberColumn", memberId, column, value);
  const updatedMember = { ...member, [column]: value };
  context.updateTypeNode(updatedMember);
}

const _updateMemberColumnDebounced = useDebounceFn(_updateMemberColumn, 200, { maxWait: 1000 });
function updateMemberColumn(memberId: string, column: ColumnType, value: any) {
  _updateMemberColumnDebounced(memberId, column, value);
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

function focusFirstIfExists() {
  if (membersLength.value == 0) {
    addMemberRef.value?.focus();
  } else {
    focus(0, columnsInOrder.value[0]);
  }
}

defineExpose({
  focus: () => descriptionRef.value?.focus(),
  defocus: () => {
    descriptionRef.value?.defocus();
    addMemberRef.value?.blur();
    Object.values(columnRefs.value).forEach((r) => r.defocus());
  },
});
</script>
<template>
  <!-- Description -->
  <EditableSpan
    ref="descriptionRef"
    v-model="description"
    :readonly="context.readonly.value"
    @navigate-up="context.navigateUp"
    @navigate-down="focusFirstIfExists"
  />
  <button
    tabindex="-1"
    v-if="description.length == 0"
    @click="descriptionRef?.focus()"
    class="w-fit rounded-sm px-0.5 text-gray-400 hover:bg-orange-50 hover:text-gray-700"
  >
    +describe
  </button>
  <!-- Enum options -->
  <div
    class="my-2 grid w-fit gap-x-3"
    :class="{
      'grid-cols-[160px_minmax(160px,1fr)]': isEnum,
      'grid-cols-[160px_160px_1fr]': isStruct,
    }"
  >
    <template v-for="member of memberTypeNodes" :key="member.id">
      <template v-for="column in columnsInOrder" :key="member.id + '.' + column">
        <InlineValueCell
          :ref="(el: any) => registerColumnRef(member.id, column, el)"
          :model-value="member[column] ?? ''"
          @update:model-value="(val) => updateMemberColumn(member.id, column, val)"
          :readonly="context.readonly.value"
          :editing="false"
          :type="STRING_TYPE_NODE"
          @navigate-left="navigateLeft(member.id, column)"
          @navigate-right="navigateRight(member.id, column)"
          @navigate-up="navigateUp(member.id, column)"
          @navigate-down="navigateDown(member.id, column)"
          @delete-left="deleteMember(member.id)"
          :class="{
            'w-full rounded-sm border border-transparent py-0.5': true,
            'focus:border-dashed focus:border-gray-700 focus:bg-orange-50': true,
          }"
        />
      </template>
    </template>
    <!-- Add a member -->
    <button
      tabindex="-1"
      ref="addMemberRef"
      class="w-fit rounded-sm px-0.5 text-gray-400 outline-none hover:bg-orange-50 hover:text-gray-700 focus:bg-orange-50"
      @click="insertBelow()"
      @enter="insertBelow()"
    >
      +{{ isEnum ? "option" : "field" }}
    </button>
  </div>
</template>
