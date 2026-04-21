/** @type {EventSource | null} */
let es = null;
let readers = 0;
const _globals = $state(/** @type {Record<string, any>} */ ({}));

function connect() {
  if (es || typeof window === "undefined") return;
  es = new EventSource("/api/globals/stream");
  es.onmessage = (e) => {
    const msg = JSON.parse(e.data);
    if (msg.snapshot) Object.assign(_globals, msg.snapshot);
    else _globals[msg.k] = msg.v;
  };
  es.onerror = () =>
    console.warn(
      "SSE connection lost, will auto-reconnect (sometimes it can be the backend shutdown/restart)",
    ); // will auto-reconnect
}

function disconnect() {
  if (es) {
    es.close();
    es = null;
  }
}

export function globals() {
  readers++;
  connect();

  $effect(() => {
    return () => {
      readers--;
      if (readers === 0) disconnect();
    };
  });

  return _globals;
}
