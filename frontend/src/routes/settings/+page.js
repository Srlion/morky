import { createApi } from "$lib/api.js";

export async function load({ fetch }) {
    const api = createApi(fetch);
    return { settings: await api.get("/settings") };
}
