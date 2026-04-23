import { createApi } from "$lib/api.js";

export async function load({ fetch }) {
  const api = createApi(fetch);
  return { sources: await api.get("/git-sources") };
}
