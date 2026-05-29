export type Lang = "zh" | "en";
export type Privacy = "public" | "locked" | "private";

export interface I18nText {
  zh: string;
  en: string;
}

export interface Category {
  key: string;
  zh: string;
  en: string;
}

export interface CategoryAdminDto {
  id: number;
  slug: string;
  name: I18nText;
  sort_order: number;
  photo_count: number;
}

export interface NewCategoryReq {
  slug: string;
  name: I18nText;
  sort_order?: number;
}

export interface UpdateCategoryReq {
  slug?: string;
  name?: I18nText;
  sort_order?: number;
}

export interface PhotoVariants {
  /** 长边 400px，列表缩略 */
  thumb?: string;
  /** 长边 900px JPG，瀑布流卡片 */
  medium?: string;
  /** 长边 1800px JPG，lightbox 大图 */
  full?: string;
  /** 长边 900px WebP，比 medium 再省 30-40% 带宽 */
  webp?: string;
  /** 用户上传的原图（10MB+），「保存原图」用 */
  original?: string;
}

export interface Photo {
  id: number;
  slug: string;
  /** 默认首图 URL（= variants.medium），保留兼容老前端代码；新代码用 variants */
  src: string;
  cat: string;
  title: I18nText;
  loc: I18nText;
  caption?: I18nText;
  alt_text?: I18nText;
  date: string;
  privacy: Privacy;
  /** 卡片图原始像素宽高，用于设 aspect-ratio 预留版位、消除瀑布流加载抖动 */
  width?: number;
  height?: number;
  tags: TagSummary[];
  variants: PhotoVariants;
  /** Locked 照片的明文口令（仅 admin 接口下发） */
  passcode?: string;
}

export interface TagSummary {
  id: number;
  slug: string;
  name: I18nText;
}

export type Tag = TagSummary;

export interface TagListResponse {
  items: Tag[];
  total: number;
  page: number;
  page_size: number;
}

export interface NewTagReq {
  slug: string;
  name: I18nText;
}

export interface UpdateTagReq {
  slug?: string;
  name?: I18nText;
}

export type PhotoPayload = Omit<Photo, "id" | "tags" | "slug" | "caption" | "alt_text" | "variants" | "passcode"> & {
  slug?: string;
  caption?: I18nText;
  alt_text?: I18nText;
  tag_ids: number[];
  /** 仅 privacy=locked 时有意义；其它隐私下后端忽略 */
  passcode?: string;
};

export interface PhotoListResp {
  items: Photo[];
  total: number;
  page: number;
  page_size: number;
}

export interface DashboardResp {
  photos: DashboardPhotoStats;
  media: DashboardMediaStats;
  users_total: number;
  tags_total: number;
  categories_total: number;
}

export interface DashboardPhotoStats {
  total: number;
  by_privacy: DashboardPrivacyCount;
  by_category: DashboardCategoryCount[];
  recent: DashboardPhotoSummary[];
}

export interface DashboardPrivacyCount {
  public: number;
  locked: number;
  private: number;
}

export interface DashboardCategoryCount {
  slug: string;
  name: I18nText;
  count: number;
}

export interface DashboardPhotoSummary {
  id: number;
  slug: string;
  title: I18nText;
  src: string;
  created_at: string;
}

export interface DashboardMediaStats {
  total: number;
  pending: number;
  processing: number;
  ready: number;
  failed: number;
}

export interface BulkDeleteReq {
  ids: number[];
}

export interface BulkPrivacyReq {
  ids: number[];
  privacy: Privacy;
}

export type BulkTagMode = "replace" | "append";

export interface BulkTagsReq {
  ids: number[];
  tag_ids: number[];
  mode: BulkTagMode;
}

export interface BulkResp {
  affected: number;
  skipped: number[];
}

export interface User {
  id: number;
  email: string;
  display_name: string;
  role: string;
}

export type UserRole = "owner" | "editor" | "viewer";

export interface UserListItem {
  id: number;
  email: string;
  display_name: string | null;
  role: UserRole;
  created_at: string;
}

export interface UserListResponse {
  items: UserListItem[];
  total: number;
  page: number;
  page_size: number;
}

export interface NewUserReq {
  email: string;
  password: string;
  display_name?: string | null;
  role: UserRole;
}

export interface UpdateUserReq {
  display_name?: string;
  role?: UserRole;
}

export interface TokenPair {
  access_token: string;
  refresh_token: string;
  token_type: "Bearer";
  expires_in: number;
  user: User;
}

export interface PresignPayload {
  file_name: string;
  mime_type: string;
  byte_size: number;
}

export interface PresignResponse {
  method: "PUT";
  upload_url: string;
  public_url: string;
  storage_key: string;
  headers: Record<string, string>;
  max_bytes: number;
  expires_in: number;
}

export type CompleteRequest = { storage_key: string };

export interface CompleteResponse {
  asset_id: number;
  storage_key: string;
  public_url: string;
}
