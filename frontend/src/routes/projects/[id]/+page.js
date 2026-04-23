import { createApi } from "$lib/api";
import { error } from "@sveltejs/kit";

export async function load({ params, fetch }) {
  const api = createApi(fetch);
  try {
    return await api.get(`/projects/${params.id}`);
  } catch {
    error(404, "Project not found");
  }
}
