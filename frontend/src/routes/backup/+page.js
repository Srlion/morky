import { createApi } from "$lib/api.js";

export async function load({ fetch }) {
  const api = createApi(fetch);
  return { status: await api.get("/backup/status") };
}
