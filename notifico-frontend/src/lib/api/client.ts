import { goto } from '$app/navigation';

function getApiKey(): string | null {
  if (typeof localStorage === 'undefined') return null;
  return localStorage.getItem('notifico_api_key');
}

export function setApiKey(key: string) {
  localStorage.setItem('notifico_api_key', key);
}

export function clearApiKey() {
  localStorage.removeItem('notifico_api_key');
}

export function isAuthenticated(): boolean {
  return !!getApiKey();
}

async function request<T>(path: string, options: RequestInit = {}): Promise<T> {
  const apiKey = getApiKey();
  const headers: Record<string, string> = {
    'Content-Type': 'application/json',
    ...((options.headers as Record<string, string>) || {}),
  };

  if (apiKey) {
    headers['Authorization'] = `Bearer ${apiKey}`;
  }

  const resp = await fetch(path, { ...options, headers });

  if (resp.status === 401) {
    clearApiKey();
    goto('/login');
    throw new Error('Unauthorized');
  }

  if (!resp.ok) {
    const text = await resp.text();
    throw new Error(`API error ${resp.status}: ${text}`);
  }

  if (resp.status === 204) return undefined as T;
  return resp.json();
}

export const api = {
  get: <T>(path: string) => request<T>(path),
  post: <T>(path: string, body?: unknown) =>
    request<T>(path, { method: 'POST', body: body ? JSON.stringify(body) : undefined }),
  put: <T>(path: string, body?: unknown) =>
    request<T>(path, { method: 'PUT', body: body ? JSON.stringify(body) : undefined }),
  delete: <T>(path: string) => request<T>(path, { method: 'DELETE' }),
};
