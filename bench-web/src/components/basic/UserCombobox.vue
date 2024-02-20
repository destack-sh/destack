<script lang="ts" setup>
import { isValidEmail, isValidSlug } from "@/composables/useValidation";
import { graphql } from "@/gql";
import { Combobox, ComboboxInput, ComboboxOption, ComboboxOptions } from "@headlessui/vue";
import { useQuery } from "@vue/apollo-composable";
import { useFocus } from "@vueuse/core";
import { computed, ref, type Ref } from "vue";

const props = defineProps<{ modelValue: { id?: string; email: string } | null; placeholder?: string }>();
const emit = defineEmits<{
  (e: "update:modelValue", value: { id?: string; email: string } | null): void;
}>();

const query = ref("");

// query by slug if it doesn't look like an email, query by email if it is a valid one

const validSlug = computed(() => isValidSlug(query.value));
const validEmail = computed(() => isValidEmail(query.value));

const { result: matchingUsersResult } = useQuery(
  graphql(/* GraphQL */ `
    query matchingUsers($slug: String, $email: String) {
      users(first: 10, filters: { slugPrefix: $slug, emailEquals: $email }) {
        totalCount
        edges {
          node {
            id
            slug
            username
            email
          }
        }
      }
    }
  `),
  computed(() => ({
    slug: validSlug.value ? query.value : null,
    email: validEmail.value ? query.value : null,
  })) as any,
  {
    enabled: computed(() => validSlug.value || validEmail.value) as any,
    fetchPolicy: "no-cache",
  }
);
const matchingUsers = computed(() => matchingUsersResult.value?.users.edges.map((e) => e.node));
const matchesCount = computed(() => matchingUsers.value?.length);

const inputRef: Ref<HTMLInputElement | null> = ref(null);
const inputRefFocused = useFocus(inputRef);
defineExpose({
  focus: () => (inputRefFocused.focused.value = true),
});
</script>
<template>
  <Combobox
    as="div"
    class="relative"
    :model-value="props.modelValue"
    @update:model-value="emit('update:modelValue', $event ?? (validEmail ? { email: query } : null))"
    nullable
  >
    <slot name="input">
      <ComboboxInput
        as="input"
        ref="inputRef"
        class="w-fit min-w-fit rounded-sm border-0 px-0 text-sm underline-offset-4 placeholder-gray-400 outline-none ring-0 focus:bg-orange-100 focus:underline focus:ring-0"
        @change="query = $event.target.value"
        :display-value="(stmt: any) => stmt?.username ?? query"
        :placeholder="placeholder"
      />
    </slot>
    <ComboboxOptions
      v-if="validSlug || validEmail"
      ref="optionsRef"
      class="absolute left-0 top-8 z-10 mt-0 w-56 rounded-sm bg-white px-1 py-1 shadow-md outline-none ring-1 ring-orange-900 ring-opacity-40"
    >
      <!-- Note that for some reason we can't use { email: query } as the value, so we use null to indicate new -->
      <!-- Invite non-existing user option -->
      <ComboboxOption v-if="query.length > 0 && validEmail && matchesCount == 0" :key="1" :value="null">
        <div class="flex flex-col px-2 py-1 hover:cursor-pointer hover:bg-orange-100">
          <span class="text-gray-900">(Invite to sign up)</span>
          <span class="text-gray-500">{{ query }}</span>
        </div>
      </ComboboxOption>
      <!-- Not found -->
      <div v-else-if="validSlug && matchesCount == 0" class="px-2 py-1">
        <span class="text-gray-500"
          >The bots can't find <span class="underline decoration-dotted underline-offset-4"> {{ query }} </span>.
        </span>
      </div>
      <!-- Matches -->
      <ComboboxOption v-for="user in matchingUsers" v-slot="{ active, selected }" :key="user.id" :value="user">
        <div
          class="flex flex-col px-2 py-1 hover:cursor-pointer hover:bg-orange-100"
          :class="[active ? 'bg-orange-100' : '', selected ? 'text-orange-600' : 'text-gray-900']"
        >
          <span class="">{{ user.username }}</span>
          <span class="text-gray-500">{{ user.email }}</span>
        </div>
      </ComboboxOption>
    </ComboboxOptions>
  </Combobox>
</template>
