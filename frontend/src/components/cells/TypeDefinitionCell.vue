<script lang="ts" setup>
import InlineActions from "@/components/cells/InlineActionsCell.vue";
import DeclarationCell from "@/components/cells/DeclarationCell.vue";
import { useElementRefs, useNavigationGrid } from "@/composables/useGrid";
import TypeInterface from "@/components/interfaces/TypeInterface.vue";
import TypeTupleInterface from "@/components/interfaces/TypeTupleInterface.vue";
import ValueInterface from "@/components/interfaces/ValueInterface.vue";
import EditableSpan from "@/components/basic/EditableSpan.vue";
import { makeTypeNode, NAME_TYPE_NODE, useStatementContext, type SimpleType } from "@/state/statement";
import { TypeTag } from "@/gql/graphql";
import type { StatementAction } from "@/state/bench";
import { newTypeNodeId, newTypeNodeKey } from "@/state/operations/statement";
import { TypeFlag } from "@/state/runtime";
import { generateKeyBetween } from "@/utils/fractional";
import { SquaresPlusIcon } from "@heroicons/vue/24/outline";
import CubeTransparentIcon from "@heroicons/vue/24/outline/CubeTransparentIcon";
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
const members = computed(
  () => context.typeNodes.value.filter((n) => !(n.flags & TypeFlag.IsUnionWith)).map((n) => n as SimpleType) ?? []
);
const membersLength = computed(() => members.value?.length ?? 0);
const extendedTypes = computed(
  () => context.typeNodes.value.filter((n) => n.flags & TypeFlag.IsUnionWith).map((n) => n as SimpleType) ?? []
);
const addMemberRef: Ref<HTMLButtonElement | null> = ref(null);
const addingDescription = ref(false);

// dynamic member refs for names, values & descriptions for each member
type ColumnType = "type" | "description";
const grid = useNavigationGrid<ColumnType, InstanceType<typeof TypeInterface>>(
  ref(["type", "description"] as ColumnType[]),
  members,
  {
    gridNavigateUp: focusDescriptionFromBottom,
    gridNavigateDown,
  }
);
const extendedTypesRefs = useElementRefs<InstanceType<typeof TypeInterface>>();
const extendButtonRef: Ref<HTMLButtonElement | null> = ref(null);
const isEditing = computed(() => grid.refs.value.find((n) => n.editing));

// TODO @Cleanup: reduce duplication between type definition & data definition cells (also in view below)
// perhaps move into statement context?

function insertMember(isUnionWith?: boolean) {
  const lastMember = context.typeNodes.value?.[context.typeNodes.value.length - 1];
  const orderKey = generateKeyBetween(lastMember?.orderKey ?? null, null);
  if (isEnum.value) {
    // enum member
    const name = "Option " + (membersLength.value + 1);
    const newMemberNode = makeTypeNode({
      name,
      tag: TypeTag.Literal,
      value: name, // :LiteralStringEnum
      orderKey,
    });
    context.createTypeNode(newMemberNode);
    nextTick(() => grid.focus(newMemberNode.id, "type"));
  } else if (!isUnionWith) {
    // struct field
    const newMemberNode = makeTypeNode({
      name: "field " + (membersLength.value + 1),
      tag: TypeTag.String,
      orderKey,
      flags: TypeFlag.IsNullable, // :DefaultTypeFlags
    });
    context.createTypeNode(newMemberNode);
    nextTick(() => grid.focus(newMemberNode.id, "type"));
  } else {
    const newMemberNode = makeTypeNode({
      name: "",
      tag: TypeTag.TypeReference,
      orderKey,
      flags: TypeFlag.IsUnionWith,
    });
    context.createTypeNode(newMemberNode);
    nextTick(() => extendedTypesRefs.focus(extendedTypes.value.slice(-1)[0].id));
  }
}

function duplicateMember(memberId: string) {
  // :DuplicateTypeNode
  const memberIdx = members.value?.findIndex((m) => m.id === memberId);
  if (memberIdx < 0) return;
  const member = members.value?.[memberIdx];
  const orderKey = generateKeyBetween(member?.orderKey ?? null, members.value?.[memberIdx + 1]?.orderKey ?? null);
  // "name" => "name 2", "name 2" => "name 3", etc.
  const newName =
    member.name?.replace(/(\d+)?$/, (_, num) => (parseInt(num ?? "1") + 1).toString()) ?? member.name + " 2";
  const newMemberNode = {
    ...member,
    id: newTypeNodeId(),
    key: newTypeNodeKey(),
    name: newName,
    orderKey,
  };
  context.createTypeNode(newMemberNode);
  nextTick(() => grid.focus(memberIdx + 1, "type"));
}

function deleteMember(memberId: string) {
  const memberIdx = members.value?.findIndex((m) => m.id === memberId);
  if (memberIdx == null || memberIdx < 0) {
    return;
  }
  const member = members.value?.[memberIdx];
  context.deleteTypeNode(member as any); // must exist
  grid.focus(memberIdx - 1, "type"); // move focus above
}

function moveMember(node: SimpleType, position: "before" | "after", other: SimpleType) {
  const otherIndex = members.value?.findIndex((n) => n.id == other.id);
  if (position == "before") {
    const orderKey = generateKeyBetween(members.value[otherIndex - 1]?.orderKey ?? null, other.orderKey);
    context.moveTypeNode(node, orderKey);
  } else {
    const orderKey = generateKeyBetween(other.orderKey, members.value[otherIndex + 1]?.orderKey ?? null);
    context.moveTypeNode(node, orderKey);
  }
}

function dropMember(droppedId: string, position: "above" | "below", memberId: string) {
  const dropped = members.value.find((n) => n.id == droppedId);
  const member = members.value.find((n) => n.id == memberId);
  if (dropped == null || member == null || dropped.id == member.id) return; // ignore invalid / cross statement drops
  moveMember(dropped, ["above", "left"].includes(position) ? "before" : "after", member);
  nextTick(() => grid.focus(droppedId, "type"));
}

function writeType(memberId: string, newType: SimpleType) {
  const oldType = members.value?.find((m) => m.id === memberId);
  if (!oldType) return;
  context.updateTypeNode(oldType, { ...oldType, ...newType, id: memberId });
}

function writeDescription(memberId: string, newDescription: string) {
  const oldType = members.value?.find((m) => m.id === memberId);
  if (!oldType) return;
  context.updateTypeNode(oldType, { ...oldType, description: newDescription });
}

function focusDescriptionFromTop() {
  if (description.value?.length > 0 || addingDescription.value) {
    descriptionRef.value?.focus();
  } else {
    focusFirstIfExists();
  }
}

function focusDescriptionFromBottom() {
  if (description.value?.length > 0 || addingDescription.value) {
    descriptionRef.value?.focus();
  } else {
    declarationRef.value?.focus();
  }
}

function focusFirstIfExists() {
  if (membersLength.value == 0) {
    addMemberRef.value?.focus();
  } else {
    grid.focus(0, "type");
  }
}

function focusLast() {
  if (membersLength.value > 0) {
    grid.focus(membersLength.value - 1, "type");
  } else {
    focusDescriptionFromBottom();
  }
}

function gridNavigateDown() {
  addMemberRef.value?.focus();
}

const extraInlineActions = computed(() => {
  const inlineActions: StatementAction[] = [];
  if (!isEnum.value) {
    inlineActions.push({
      label: "Extend",
      icon: CubeTransparentIcon,
      action: () => insertMember(true),
    });
  }
  inlineActions.push({
    label: "Add " + (isEnum.value ? "option" : "field"),
    icon: SquaresPlusIcon,
    action: () => insertMember(),
  });
  return inlineActions;
});

defineExpose({
  focus: (position: "first" | "last" = "first") =>
    position == "first" ? declarationRef.value?.focus() : addMemberRef.value?.focus(),
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
  <div class="flex flex-row justify-between">
    <div class="flex flex-row items-baseline">
      <DeclarationCell
        ref="declarationRef"
        @navigate-down="focusDescriptionFromTop"
        @navigate-right="
          extendedTypes.length > 0 ? extendedTypesRefs.focus(extendedTypes[0].id) : extendButtonRef?.focus()
        "
      />
      <!-- Extended types -->
      <!-- TODO @Cleanup: reduce duplication with data definition cell (and general ugliness of keyboard navigation...) -->
      <div class="ml-1 whitespace-nowrap" v-if="(extendedTypes?.length ?? 0) > 0">
        <span class="mr-1 text-orange-600">is</span>
        <div class="inline-flex flex-row gap-1">
          <TypeInterface
            v-for="field of extendedTypes"
            :ref="(el: any) => extendedTypesRefs.registerRef(field.id, el)"
            :model-value="field"
            @update:model-value="(val) => context.updateTypeNode(field, val)"
            @delete-self="context.deleteTypeNode(field)"
            @navigate-left="
              field.id == extendedTypes[0].id
                ? declarationRef?.focus()
                : extendedTypesRefs.focus(extendedTypes[extendedTypes.findIndex((n) => n.id == field.id) - 1].id)
            "
            @navigate-right="
              field.id == extendedTypes[extendedTypes.length - 1].id
                ? extendButtonRef?.focus()
                : extendedTypesRefs.focus(extendedTypes[extendedTypes.findIndex((n) => n.id == field.id) + 1].id)
            "
            @navigate-down="focusDescriptionFromTop"
            @navigate-up="context.navigateUp"
            :active="context.focused.value || context.editing.value"
            :key="field.id"
            :readonly="context.readonly.value"
            structref-only
            hide-flags
            hide-icon
            class="w-full rounded-sm border border-transparent focus-within:border-solid focus-within:border-orange-900 focus-within:border-opacity-[15%] focus-within:bg-orange-100 hover:bg-orange-100"
          />
        </div>
      </div>
      <!-- Inline buttons -->
      <button
        v-if="!isEnum && !context.readonly.value && (extendedTypes?.length ?? 0) <= 1"
        ref="extendButtonRef"
        @keydown.down.exact="focusDescriptionFromTop"
        @keydown.up.exact="context.navigateUp"
        @keydown.left.exact="
          extendedTypes.length > 0 ? extendedTypesRefs.focus(extendedTypes[0].id) : declarationRef?.focus()
        "
        tabindex="-1"
        @click="() => insertMember(true)"
        class="ml-2 w-fit rounded-sm px-0.5 text-gray-300 hover:bg-orange-100 hover:text-gray-700 focus:bg-orange-100 focus:outline-none group-focus-within/statement:text-gray-400"
      >
        +extend
      </button>
      <button
        tabindex="-1"
        v-if="description.length == 0 && !context.readonly.value && !addingDescription"
        @click="
          addingDescription = true;
          descriptionRef?.focus();
        "
        class="ml-2 w-fit rounded-sm px-0.5 text-gray-300 hover:bg-orange-100 hover:text-gray-700 focus:bg-orange-100 focus:outline-none group-focus-within/statement:text-gray-400"
      >
        +description
      </button>
    </div>
    <InlineActions
      class="transition duration-150 group-hover/statement:opacity-100"
      :class="context.focused.value ? '' : 'opacity-0'"
      :extraActions="extraInlineActions"
    />
  </div>
  <!-- Description -->
  <EditableSpan
    ref="descriptionRef"
    :class="addingDescription ? '' : 'h-0'"
    v-model="description"
    :readonly="context.readonly.value"
    @navigate-left="declarationRef?.focus()"
    @navigate-up="declarationRef?.focus()"
    @navigate-down="focusFirstIfExists"
  />
  <button
    tabindex="-1"
    v-if="description.length == 0 && !context.readonly.value && addingDescription"
    @click="descriptionRef?.focus()"
    class="w-fit rounded-sm px-0.5 text-gray-300 hover:bg-orange-100 hover:text-gray-700 group-focus-within/statement:text-gray-400"
  >
    +description
  </button>
  <!-- Members (enum options or struct fields) -->
  <div v-if="membersLength > 0" class="my-1 grid w-fit grid-cols-[minmax(40px,auto)_minmax(160px,1fr)] gap-y-0.5">
    <!-- Rows -->
    <template v-for="member of members" :key="member.id">
      <!-- Type -->
      <TypeTupleInterface
        :model-value="member"
        @update:model-value="(val: any) => writeType(member.id, val)"
        :ref="(el: any) => grid.registerColumnRef(member.id, 'type', el)"
        :readonly="context.readonly.value"
        :isEnum="isEnum"
        :tupleName="isEnum ? 'option' : 'field'"
        orientation="vertical"
        @navigate-left="grid.navigateLeft(member.id, 'type')"
        @navigate-right="grid.navigateRight(member.id, 'type')"
        @navigate-up="grid.navigateUp(member.id, 'type')"
        @navigate-down="grid.navigateDown(member.id, 'type')"
        @delete-self="deleteMember(member.id)"
        @duplicate-self="duplicateMember(member.id)"
        @keydown.delete.exact="isEditing || deleteMember(member.id)"
        @drop="(p, v) => dropMember(v.id, p, member.id)"
        class="self-start border border-orange-900 border-opacity-0 text-gray-400 focus-within:bg-orange-100 hover:bg-orange-100"
        :class="
          isEnum
            ? 'w-fit focus-within:border-opacity-40 hover:border-opacity-40'
            : 'w-full py-0.5 focus-within:border-opacity-[15%]'
        "
      />
      <!-- Description -->
      <ValueInterface
        :model-value="member.description"
        @update:model-value="writeDescription(member.id, $event)"
        :ref="(el: any) => grid.registerColumnRef(member.id, 'description', el)"
        :readonly="context.readonly.value"
        :active="context.focused.value || context.editing.value"
        debounced
        :placeholder-value="context.editing.value ? '+' + 'description' : null"
        :type="NAME_TYPE_NODE"
        @navigate-left="grid.navigateLeft(member.id, 'description')"
        @navigate-right="grid.navigateRight(member.id, 'description')"
        @navigate-up="grid.navigateUp(member.id, 'description')"
        @navigate-down="grid.navigateDown(member.id, 'description')"
        @delete-self="deleteMember(member.id)"
        @keydown.delete.exact="isEditing || deleteMember(member.id)"
        class="w-full self-start border border-transparent px-2 py-0.5 focus-within:border-solid focus-within:border-gray-700 focus-within:border-opacity-[15%] focus-within:bg-orange-100 hover:bg-orange-100"
      />
      <!-- :EditableCellStyle -->
    </template>
  </div>
  <div class="mb-1">
    <!-- Add a member -->
    <button
      v-show="!context.readonly.value"
      tabindex="-1"
      ref="addMemberRef"
      class="mt-1 w-fit select-none rounded-sm px-0.5 text-gray-300 outline-none hover:bg-orange-100 hover:text-gray-700 focus:bg-orange-100 group-focus-within/statement:text-gray-400"
      @click="insertMember()"
      @enter="insertMember()"
      @keydown.up.exact.prevent="focusLast"
      @keydown.down.exact.prevent="context.navigateDown"
    >
      +{{ isEnum ? "option" : "field" }}
    </button>
  </div>
</template>
