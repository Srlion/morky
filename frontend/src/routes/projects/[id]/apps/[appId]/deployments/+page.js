import { createApi } from "$lib/api.js";

export async function load({ params, fetch, url }) {
  const api = createApi(fetch);
  const p = url.searchParams.get("page") || "1";
  const res = await api.get(
    `/apps/${params.appId}/deployments?page=${p}&per_page=12`,
  );
  return {
    deployments: res.items,
    total: res.total,
    page: res.page,
    perPage: res.per_page,
  };
}
