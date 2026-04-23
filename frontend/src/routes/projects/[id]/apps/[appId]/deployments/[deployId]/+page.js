import { createApi } from "$lib/api.js";

export async function load({ params, fetch }) {
  const api = createApi(fetch);
  return {
    deployment: await api.get(
      `/apps/${params.appId}/deployments/${params.deployId}`,
    ),
  };
}
