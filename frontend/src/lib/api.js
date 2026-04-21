class ApiError extends Error {
  constructor(status, message) {
    super(message);
    this.status = status;
  }
}

async function req(path, f, opts) {
  const res = await f("/api" + path, {
    ...opts,
    headers: { "Content-Type": "application/json", ...opts?.headers },
  });
  if (res.status === 401) {
    window.location.href = "/auth/signin";
    throw new ApiError(401, "unauthorized");
  }
  const body = await res.json();
  if (!res.ok) throw new ApiError(res.status, body.error || "request failed");
  return body;
}

export function createApi(f = fetch) {
  return {
    get: (p) => req(p, f),
    post: (p, d) => {
      const isRaw = typeof d === "string";
      return req(p, f, {
        method: "POST",
        body: isRaw ? d : d ? JSON.stringify(d) : undefined,
        headers: isRaw ? { "Content-Type": "text/plain" } : {},
      });
    },
    put: (p, d) => {
      const isRaw = typeof d === "string";
      return req(p, f, {
        method: "PUT",
        body: isRaw ? d : JSON.stringify(d),
        headers: isRaw ? { "Content-Type": "text/plain" } : {},
      });
    },
    del: (p) => req(p, f, { method: "DELETE" }),
  };
}

export const api = createApi();
export { ApiError };
