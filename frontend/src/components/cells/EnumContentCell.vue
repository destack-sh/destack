<script lang="ts" setup>
import EditableSpan from "@/components/EditableSpan.vue";
import { useStatementContext } from "@/components/statement";
import { computed, ref, toRef, type Ref } from "vue";

const context = useStatementContext();
const description: Ref<string> = ref(context.statement.value.description ?? "");
const descriptionRef: Ref<HTMLButtonElement | null> = ref(null);
context.syncDescription(description);

const headTypeNode = toRef(context, "typeNodeHead");
const memberTypeNodes = computed(() => context.typeNodes.value?.filter((n) => n.parentId == headTypeNode.value?.id));
const nameRefs: Ref<HTMLDivElement[]> = ref([]);
const addMemberRef: Ref<HTMLButtonElement | null> = ref(null);
</script>
<template>
  <EditableSpan ref="descriptionRef" v-model="description" :readonly="context.readonly.value" />
  <div class="grid w-full grid-cols-2">
    <template v-for="(member, i) of memberTypeNodes" :key="member.id">
      <EditableSpan ref="nameRefs" :model-value="member.name ?? ''" :readonly="context.readonly.value" />
      <span>description</span>
    </template>
    <button ref="addMemberRef" class="w-fit text-gray-500">+member</button>
  </div>
</template>
