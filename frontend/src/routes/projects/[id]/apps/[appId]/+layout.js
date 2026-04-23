import { createApi } from "$lib/api";
import { error } from "@sveltejs/kit";

export async function load({ params, fetch, parent }) {
  const api = createApi(fetch);
  const parentData = await parent();
  try {
    return {
      project: parentData.project,
      app: await api.get(`/apps/${params.appId}`),
    };
  } catch {
    error(404, "App not found");
  }
}
