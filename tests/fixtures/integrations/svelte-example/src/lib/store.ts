import { writable } from "svelte/store";

export const count = writable(0);

// whitelabel *
export function onclick() {
  count.update((c) => c + 1);
}
