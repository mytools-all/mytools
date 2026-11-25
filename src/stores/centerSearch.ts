import { defineStore } from "pinia";
import { persistedUiValue } from "@/stores/uiState";

export const useCenterSearchStore = defineStore("centerSearch", () => {
  const query = persistedUiValue("center.query", "", (value):value is string=>typeof value==="string");
  return { query };
});
