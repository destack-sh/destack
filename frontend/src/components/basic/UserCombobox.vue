<script lang="ts" setup>
import { isValidEmail, isValidSlug } from "@/composables/useValidation";
import { graphql } from "@/gql";
import { Combobox, ComboboxInput, ComboboxOption, ComboboxOptions } from "@headlessui/vue";
import { useQuery } from "@vue/apollo-composable";
import { computed, ref } from "vue";

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
    enabled: computed(() => validSlug.value || validEmail.value),
    fetchPolicy: "no-cache",
  }
);
const matchingUsers = computed(() => matchingUsersResult.value?.users.edges.map((e) => e.node));
const matchesCount = computed(() => matchingUsers.value?.length);
</script>
<template>
  <Combobox
    as="div"
    class="relative"
    :model-value="props.modelValue"
    @update:model-value="emit('update:modelValue', $event)"
    nullable
  >
    <slot name="input">
      <ComboboxInput
        as="input"
        ref="inputRef"
        class="w-auto min-w-fit rounded-sm border-0 px-0 underline-offset-4 placeholder-gray-400 outline-none ring-0 focus:underline focus:ring-0"
        @change="query = $event.target.value"
        :display-value="(stmt: any) => stmt?.username"
        :placeholder="placeholder"
      />
    </slot>
    <ComboboxOptions
      v-if="validSlug || validEmail"
      ref="optionsRef"
      class="absolute left-0 top-8 z-10 mt-0 w-56 rounded-sm bg-white px-1 py-1 shadow-md outline-none ring-1 ring-orange-900 ring-opacity-40"
    >
      <ComboboxOption v-if="query.length > 0 && validEmail" :key="0" :value="{ email: query }">
        {{ query }}
      </ComboboxOption>
      <div v-else-if="validSlug && matchesCount == 0">no matches</div>
      <ComboboxOption :key="user.id" :value="user" v-for="user in matchingUsers">
        <div class="flex flex-col">
          <span class="text-gray-900">{{ user.username }}</span>
          <span class="text-gray-500">{{ user.email }}</span>
        </div>
      </ComboboxOption>
    </ComboboxOptions>
  </Combobox>
</template>
