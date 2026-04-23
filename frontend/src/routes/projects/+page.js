import { createApi } from "$lib/api";

export async function load({ fetch }) {
  const api = createApi(fetch);
  return { projects: await api.get("/projects") };
}
