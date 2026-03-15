const DEFAULT_HEADERS = {
  "Content-Type": "application/json",
};

export async function apiGet<T = unknown>(path: string, userId = 1): Promise<T> {
  const response = await fetch(path, {
    method: "GET",
    headers: {
      ...DEFAULT_HEADERS,
      "x-user-id": String(userId),
    },
  });

  if (!response.ok) {
    throw new Error(`GET ${path} failed with status ${response.status}`);
  }

  return response.json() as Promise<T>;
}
