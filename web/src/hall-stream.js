export async function readHallStream(token, { signal, onEvent, onOpen }) {
  const response = await fetch("/api/hall/stream", {
    cache: "no-store",
    headers: {
      Accept: "text/event-stream",
      Authorization: "Bearer " + token,
    },
    signal,
  });
  if (!response.ok) {
    const result = await response.json();
    const failure = new Error(result.error || response.statusText);
    failure.status = response.status;
    failure.code = result.code;
    throw failure;
  }
  onOpen();
  const reader = response.body.getReader();
  const decoder = new TextDecoder();
  let buffer = "";
  try {
    while (true) {
      const { value, done } = await reader.read();
      if (done) break;
      buffer += decoder.decode(value, { stream: true });
      const events = buffer.split(/\r?\n\r?\n/);
      buffer = events.pop();
      for (const event of events) {
        const data = event
          .split(/\r?\n/)
          .filter((line) => line.startsWith("data:"))
          .map((line) => line.slice(5).trimStart())
          .join("\n");
        if (!data) continue;
        const result = JSON.parse(data);
        if (
          event.split(/\r?\n/).some((line) => /^event:\s*error\s*$/.test(line))
        ) {
          const failure = new Error(result.error);
          failure.status = result.status;
          failure.code = result.code;
          throw failure;
        }
        onEvent(result);
      }
    }
    if (!signal.aborted) throw new Error("大厅推送已断开");
  } finally {
    reader.releaseLock();
  }
}
