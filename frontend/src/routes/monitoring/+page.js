import { createApi } from "$lib/api.js";

export async function load({ fetch }) {
    const api = createApi(fetch);
    const [stats, cleanupSettings] = await Promise.all([
        api.get("/monitoring/stats"),
        api.get("/monitoring/cleanup/settings"),
    ]);
    return { stats, podmanDisk: null, cleanupSettings };
}
