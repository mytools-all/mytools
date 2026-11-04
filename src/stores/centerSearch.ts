import { defineStore } from "pinia";
import { ref } from "vue";

export const useCenterSearchStore = defineStore("centerSearch", () => {
  const query = ref("");
  return { query };
});
