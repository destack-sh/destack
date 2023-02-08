<script lang="ts" setup>
import EditableSpan from "@/components/EditableSpan.vue";
import { useStatementContext } from "@/components/statement";
import { computed, ref, type Ref } from "vue";

const context = useStatementContext();
const description: Ref<string> = ref(context.statement.value.description ?? "");
const descriptionRef: Ref<InstanceType<typeof EditableSpan> | null> = ref(null);
context.syncDescription(description);

const headTypeNode = computed(() => context.typeNodesChildren.value?.[0]);
const memberTypeNodes = computed(() => context.typeNodesChildren.value?.slice(1));
const nameRefs: Ref<HTMLDivElement[]> = ref([]);
const addMemberRef: Ref<HTMLButtonElement | null> = ref(null);

defineExpose({
  focus: () => descriptionRef.value?.focus(),
  defocus: () => {
    descriptionRef.value?.defocus();
  },
});
</script>
<template>
  <EditableSpan ref="descriptionRef" v-model="description" :readonly="context.readonly.value" />
  <button v-if="description.length == 0" @click="descriptionRef?.focus()" class="text-gray-400">+describe</button>
  <div class="my-2 grid w-fit grid-cols-3 gap-x-6">
    <template v-for="(member, i) of memberTypeNodes" :key="member.id">
      <EditableSpan ref="nameRefs" :model-value="member.name ?? ''" :readonly="context.readonly.value" />
      <span>{{ member.value }}</span>
      <div>
        <button v-if="member.description?.length ?? 0 < 1" class="text-gray-400">+describe</button>
        <span>{{ member.description }}</span>
      </div>
    </template>
    <button ref="addMemberRef" class="w-fit text-gray-400">+option</button>
  </div>
</template>
