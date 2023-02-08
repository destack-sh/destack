<script lang="ts" setup>
import EditableSpan from "@/components/EditableSpan.vue";
import { makeTypeNodeData, useStatementContext } from "@/components/statement";
import { generateKeyBetween } from "@/utils/fractional";
import { useDebounceFn } from "@vueuse/shared";
import { computed, nextTick, ref, type Ref } from "vue";

const context = useStatementContext();
const description: Ref<string> = ref(context.statement.value.description ?? "");
const descriptionRef: Ref<InstanceType<typeof EditableSpan> | null> = ref(null);
context.syncDescription(description);

const headTypeNode = computed(() => context.typeNodesChildren.value?.[0]);
const memberTypeNodes = computed(() => context.typeNodesChildren.value?.slice(1));
const membersLength = computed(() => memberTypeNodes.value?.length ?? 0);
const addMemberRef: Ref<HTMLButtonElement | null> = ref(null);

// dynamic member refs for names, values & descriptions for each member
type FieldType = "name" | "description";
const FIELDS_IN_ORDER: FieldType[] = ["name", "description"];

const nameRefs: Ref<Record<string, InstanceType<typeof EditableSpan>>> = ref({});
const descriptionRefs: Ref<Record<string, InstanceType<typeof EditableSpan>>> = ref({});

function focus(index: number, field: FieldType) {
  const member = memberTypeNodes.value?.[index];
  if (!member) {
    return;
  }
  if (field === "name") {
    nameRefs.value?.[member.id]?.focus();
  } else if (field === "description") {
    descriptionRefs.value?.[member.id]?.focus();
  }
}

function navigateUp(memberId: string, field: FieldType) {
  const memberIdx = memberTypeNodes.value?.findIndex((m) => m.id === memberId);
  if (!memberIdx) {
    context.navigateUp();
  } else {
    focus(memberIdx - 1, field);
  }
}

function navigateDown(memberId: string, field: FieldType) {
  const memberIdx = memberTypeNodes.value?.findIndex((m) => m.id === memberId) ?? 0;
  if (memberIdx == membersLength.value - 1) {
    context.navigateDown();
  } else {
    focus(memberIdx + 1, field);
  }
}

function navigateRight(memberId: string, field: FieldType) {
  const memberIdx = memberTypeNodes.value?.findIndex((m) => m.id === memberId) ?? -1;
  const fieldIdx = FIELDS_IN_ORDER.findIndex((f) => f === field);
  if (fieldIdx == FIELDS_IN_ORDER.length - 1) {
    if (memberIdx != membersLength.value - 1) {
      focus(memberIdx + 1, FIELDS_IN_ORDER[0]);
    }
  } else {
    focus(memberIdx, FIELDS_IN_ORDER[fieldIdx + 1]);
  }
}

function navigateLeft(memberId: string, field: FieldType) {
  const memberIdx = memberTypeNodes.value?.findIndex((m) => m.id === memberId) ?? -1;
  const fieldIdx = FIELDS_IN_ORDER.findIndex((f) => f === field);
  if (fieldIdx == 0) {
    if (memberIdx != 0) {
      focus(memberIdx - 1, FIELDS_IN_ORDER[FIELDS_IN_ORDER.length - 1]);
    }
  } else {
    focus(memberIdx, FIELDS_IN_ORDER[fieldIdx - 1]);
  }
}

async function insertBelow(memberId?: string) {
  if (headTypeNode.value == null) {
    // head type node must always be set for enums
    throw new Error("headTypeNode is null");
  }
  let orderKey;
  if (memberId == null) {
    const lastMember = memberTypeNodes.value?.[membersLength.value - 1];
    orderKey = generateKeyBetween(lastMember?.orderKey ?? null, null);
  } else {
    const member = memberTypeNodes.value?.find((m) => m.id === memberId);
    orderKey = generateKeyBetween(member?.orderKey ?? null, null);
  }
  await context.createTypeNode(
    makeTypeNodeData({ name: "", tag: headTypeNode.value?.tag, orderKey, parentId: context.typeNodeRoot.value?.id })
  );
  nextTick(() => focus(membersLength.value - 1, "name"));
}

// local copy of fields for immediate editing
// TODO @Cleanup: apply member field updates immediately once done with individual "field edit mode"
//  (separate statement editing from individual field editing)
const memberFields: Ref<Record<string, any>> = ref({});
for (const member of memberTypeNodes.value ?? []) {
  for (const field of FIELDS_IN_ORDER) {
    memberFields.value[member.id + "." + field] = member[field];
  }
}

function _updateMemberField(memberId: string, field: FieldType, value: any) {
  const member = memberTypeNodes.value?.find((m) => m.id === memberId);
  if (!member) {
    return;
  }
  console.log("updateMemberField", memberId, field, value);
  const updatedMember = { ...member, [field]: value };
  context.updateTypeNode(updatedMember);
}

const _updateMemberFieldDebounced = useDebounceFn(_updateMemberField, 200, { maxWait: 1000 });
function updateMemberField(memberId: string, field: FieldType, value: any) {
  memberFields.value[memberId + "." + field] = value;
  _updateMemberFieldDebounced(memberId, field, value);
}

function memberField(memberId: string, field: FieldType) {
  return memberFields.value[memberId + "." + field];
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

defineExpose({
  focus: () => descriptionRef.value?.focus(),
  defocus: () => {
    descriptionRef.value?.defocus();
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
    @navigate-down="focus(0, FIELDS_IN_ORDER[0])"
  />
  <button
    tabindex="-1"
    v-if="description.length == 0"
    @click="descriptionRef?.focus()"
    class="w-fit rounded-sm px-0.5 text-gray-400 hover:bg-orange-50 hover:text-gray-700"
  >
    +describe
  </button>
  <!-- Members -->
  <div class="grid-flow-dense my-2 grid w-fit grid-cols-[160px_80px_1fr] gap-x-6">
    <template v-for="member of memberTypeNodes" :key="member.id">
      <EditableSpan
        :ref="(el: any) => nameRefs[member.id] = el ?? undefined"
        :model-value="memberField(member.id, 'name') ?? ''"
        @update:model-value="(val) => updateMemberField(member.id, 'name', val)"
        :readonly="context.readonly.value"
        @navigate-up="navigateUp(member.id, 'name')"
        @navigate-down="navigateDown(member.id, 'name')"
        @navigate-right="navigateRight(member.id, 'name')"
        @navigate-left="navigateLeft(member.id, 'name')"
        @delete-left="deleteMember(member.id)"
        @enter="insertBelow(member.id)"
      />
      <span class="w-fit rounded-sm px-0.5 text-gray-400 hover:bg-orange-50 hover:text-gray-700">
        {{ member.value }}
      </span>
      <div>
        <EditableSpan
          :ref="(el: any) => descriptionRefs[member.id] = el ?? undefined"
          :model-value="memberField(member.id, 'description') ?? ''"
          @update:model-value="(val) => updateMemberField(member.id, 'description', val)"
          :readonly="context.readonly.value"
          @navigate-up="navigateUp(member.id, 'description')"
          @navigate-down="navigateDown(member.id, 'description')"
          @navigate-right="navigateRight(member.id, 'description')"
          @navigate-left="navigateLeft(member.id, 'description')"
          @enter="insertBelow(member.id)"
        />
        <button
          v-if="descriptionRefs[member.id]?.innerText?.length == 0"
          @click="descriptionRefs[member.id].focus()"
          class="w-fit rounded-sm px-0.5 text-gray-400 hover:bg-orange-50 hover:text-gray-700"
        >
          +describe
        </button>
      </div>
    </template>
    <!-- Add more -->
    <button
      tabindex="-1"
      ref="addMemberRef"
      class="w-fit rounded-sm px-0.5 text-gray-400 hover:bg-orange-50 hover:text-gray-700"
      @click="insertBelow()"
      @enter="insertBelow()"
    >
      +option
    </button>
  </div>
</template>
