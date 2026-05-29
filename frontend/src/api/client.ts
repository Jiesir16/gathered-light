import type {
  BulkDeleteReq,
  BulkPrivacyReq,
  BulkResp,
  BulkTagsReq,
  CategoryAdminDto,
  CompleteRequest,
  CompleteResponse,
  DashboardResp,
  NewCategoryReq,
  NewTagReq,
  NewUserReq,
  Photo,
  PhotoListResp,
  PhotoPayload,
  PresignPayload,
  PresignResponse,
  Tag,
  TagListResponse,
  UpdateCategoryReq,
  TokenPair,
  UpdateTagReq,
  UpdateUserReq,
  User,
  UserListItem,
  UserListResponse
} from "../types";

const accessKey = "gathered_light_access_token";
const refreshKey = "gathered_light_refresh_token";
const apiBase = import.meta.env.VITE_API_BASE ?? "";

type ApiRequestInit = Omit<RequestInit, "body"> & {
  body?: unknown;
};

export type HeroI18n = { zh: string; en: string };
export type HeroLines = { zh: string[]; en: string[] };
export type HeroCopy = {
  issue: HeroI18n;
  headline: HeroI18n;
  introLines: HeroLines;
};

export type SiteSettings = {
  theme: string;
  range: string;
  hero: HeroCopy;
};

export class ApiError extends Error {
  constructor(
    message: string,
    public readonly status: number,
    public readonly code?: string
  ) {
    super(message);
  }
}

async function request<T>(path: string, init: ApiRequestInit = {}, auth = true, retry = true): Promise<T> {
  const headers = new Headers(init.headers);
  let body: BodyInit | undefined;
  if (init.body instanceof FormData || typeof init.body === "string") {
    body = init.body;
  } else if (init.body !== undefined) {
    body = JSON.stringify(init.body);
    headers.set("content-type", "application/json");
  }

  const accessToken = localStorage.getItem(accessKey);
  if (auth && accessToken) headers.set("authorization", `Bearer ${accessToken}`);

  const response = await fetch(`${apiBase}${path}`, { ...init, headers, body });
  if (response.status === 401 && auth && retry && localStorage.getItem(refreshKey)) {
    const refreshed = await refreshTokens();
    if (refreshed) return request<T>(path, init, auth, false);
  }

  if (response.status === 204) return undefined as T;

  const text = await response.text();
  const data = text ? parseJson(text) : null;
  if (!response.ok) {
    throw new ApiError(data?.message ?? response.statusText, response.status, data?.code);
  }
  return data as T;
}

function parseJson(text: string): any {
  try {
    return JSON.parse(text);
  } catch {
    return { message: text };
  }
}

function saveTokens(pair: TokenPair) {
  localStorage.setItem(accessKey, pair.access_token);
  localStorage.setItem(refreshKey, pair.refresh_token);
}

export function hasSession() {
  return Boolean(localStorage.getItem(accessKey) || localStorage.getItem(refreshKey));
}

export function clearSession() {
  localStorage.removeItem(accessKey);
  localStorage.removeItem(refreshKey);
}

export async function login(email: string, password: string) {
  const pair = await request<TokenPair>("/api/v1/auth/login", {
    method: "POST",
    body: { email, password }
  }, false);
  saveTokens(pair);
  return pair.user;
}

export async function refreshTokens() {
  const refreshToken = localStorage.getItem(refreshKey);
  if (!refreshToken) return false;
  try {
    const pair = await request<TokenPair>("/api/v1/auth/refresh", {
      method: "POST",
      body: { refresh_token: refreshToken }
    }, false, false);
    saveTokens(pair);
    return true;
  } catch {
    clearSession();
    return false;
  }
}

export async function logout() {
  try {
    await request("/api/v1/auth/logout", { method: "POST" });
  } finally {
    clearSession();
  }
}

const adminPhotoApi = {
  list: (page = 1, pageSize = 20, q = "", category = "", privacy = "") =>
    request<PhotoListResp>(
      `/api/v1/admin/photos?page=${page}&page_size=${pageSize}` +
        (q ? `&q=${encodeURIComponent(q)}` : "") +
        (category ? `&category=${encodeURIComponent(category)}` : "") +
        (privacy ? `&privacy=${encodeURIComponent(privacy)}` : "")
    ),
  create: (photo: PhotoPayload) =>
    request<Photo>("/api/v1/admin/photos", { method: "POST", body: photo }),
  update: (id: number, photo: PhotoPayload) =>
    request<Photo>(`/api/v1/admin/photos/${id}`, { method: "PUT", body: photo }),
  remove: (id: number) =>
    request<void>(`/api/v1/admin/photos/${id}`, { method: "DELETE" }),
  resetSeeds: () =>
    request<Photo[]>("/api/v1/admin/photos/reset", { method: "POST" }),
  updatePrivacy: (id: number, privacy: Photo["privacy"]) =>
    request<Photo>(`/api/v1/admin/photos/${id}/privacy`, {
      method: "PATCH",
      body: { privacy }
    }),
  recoverUrl: (id: number) =>
    request<Photo>(`/api/v1/admin/photos/${id}/recover-url`, { method: "POST" }),
  bulkDelete: (req: BulkDeleteReq) =>
    request<BulkResp>("/api/v1/admin/photos/bulk/delete", { method: "POST", body: req }),
  bulkPrivacy: (req: BulkPrivacyReq) =>
    request<BulkResp>("/api/v1/admin/photos/bulk/privacy", { method: "POST", body: req }),
  bulkSetTags: (req: BulkTagsReq) =>
    request<BulkResp>("/api/v1/admin/photos/bulk/tags", { method: "POST", body: req })
};

export const api = {
  me: () => request<{ user: User }>("/api/v1/auth/me"),
  dashboard: () => request<DashboardResp>("/api/v1/admin/dashboard"),
  publicPhotos: () => request<Photo[]>("/api/v1/photos?category=all", {}, false),
  publicTags: () => request<Tag[]>("/api/v1/tags", {}, false),
  adminPhotos: adminPhotoApi,
  createPhoto: adminPhotoApi.create,
  updatePhoto: adminPhotoApi.update,
  deletePhoto: adminPhotoApi.remove,
  resetPhotos: adminPhotoApi.resetSeeds,
  updatePrivacy: adminPhotoApi.updatePrivacy,
  unlockPhoto: (id: number, passcode: string) => request<{ unlocked: boolean }>(`/api/v1/photos/${id}/unlock`, { method: "POST", body: { passcode } }, false),
  presign: (payload: PresignPayload) => request<PresignResponse>("/api/v1/admin/media/presign", { method: "POST", body: payload }),
  complete: (payload: CompleteRequest) => request<CompleteResponse>("/api/v1/admin/media/complete", { method: "POST", body: payload })
};

export const settings = {
  get: () => request<SiteSettings>("/api/v1/settings", { method: "GET" }, false)
};

export const adminSettings = {
  updateTheme: (theme: string) =>
    request<SiteSettings>("/api/v1/admin/settings/theme", {
      method: "PATCH",
      body: { theme }
    }),
  updateRange: (range: string) =>
    request<SiteSettings>("/api/v1/admin/settings/range", {
      method: "PATCH",
      body: { range }
    }),
  updateHero: (hero: HeroCopy) =>
    request<SiteSettings>("/api/v1/admin/settings/hero", {
      method: "PATCH",
      body: hero
    })
};

export const adminTags = {
  list: (page = 1, pageSize = 50) =>
    request<TagListResponse>(`/api/v1/admin/tags?page=${page}&page_size=${pageSize}`),
  create: (req: NewTagReq) =>
    request<Tag>("/api/v1/admin/tags", { method: "POST", body: req }),
  update: (id: number, req: UpdateTagReq) =>
    request<Tag>(`/api/v1/admin/tags/${id}`, { method: "PATCH", body: req }),
  remove: (id: number) =>
    request<void>(`/api/v1/admin/tags/${id}`, { method: "DELETE" })
};

export const adminCategories = {
  list: () => request<CategoryAdminDto[]>("/api/v1/admin/categories"),
  create: (req: NewCategoryReq) =>
    request<CategoryAdminDto>("/api/v1/admin/categories", { method: "POST", body: req }),
  update: (id: number, req: UpdateCategoryReq) =>
    request<CategoryAdminDto>(`/api/v1/admin/categories/${id}`, { method: "PATCH", body: req }),
  remove: (id: number) =>
    request<void>(`/api/v1/admin/categories/${id}`, { method: "DELETE" })
};

export const adminUsers = {
  list: (page = 1, pageSize = 20) =>
    request<UserListResponse>(`/api/v1/admin/users?page=${page}&page_size=${pageSize}`),
  create: (req: NewUserReq) =>
    request<UserListItem>("/api/v1/admin/users", { method: "POST", body: req }),
  update: (id: number, req: UpdateUserReq) =>
    request<UserListItem>(`/api/v1/admin/users/${id}`, { method: "PATCH", body: req }),
  resetPassword: (id: number, newPassword: string) =>
    request<void>(`/api/v1/admin/users/${id}/password`, {
      method: "POST",
      body: { new_password: newPassword }
    }),
  remove: (id: number) =>
    request<void>(`/api/v1/admin/users/${id}`, { method: "DELETE" })
};
