// 数据层 — 用 localStorage 模拟后端
const STORAGE_KEY = "shiguangji_photos_v1";
const AUTH_KEY = "shiguangji_auth_v1";
const LANG_KEY = "shiguangji_lang_v1";
const ADMIN_PASS = "admin"; // 演示密码

const DEFAULT_PHOTOS = [
  { id: 1,  src: "https://images.unsplash.com/photo-1502082553048-f009c37129b9?w=900&q=80",  cat: "landscape", title: { zh: "松岭", en: "Pine Ridge" },           loc: { zh: "北海道", en: "Hokkaido, JP" },     date: "2024.11", privacy: "public" },
  { id: 2,  src: "https://images.unsplash.com/photo-1519681393784-d120267933ba?w=900&q=80",  cat: "landscape", title: { zh: "冰川的安静", en: "Glacial Silence" }, loc: { zh: "班夫", en: "Banff, CA" },          date: "2024.08", privacy: "public" },
  { id: 3,  src: "https://images.unsplash.com/photo-1444703686981-a3abbc4d4fe3?w=900&q=80",  cat: "landscape", title: { zh: "极光", en: "Aurora" },                loc: { zh: "特罗姆瑟", en: "Tromsø, NO" },     date: "2024.02", privacy: "locked" },
  { id: 4,  src: "https://images.unsplash.com/photo-1493514789931-586cb221d7a7?w=900&q=80",  cat: "street",    title: { zh: "斑马线", en: "Crosswalk" },           loc: { zh: "东京", en: "Tokyo, JP" },          date: "2024.05", privacy: "public" },
  { id: 5,  src: "https://images.unsplash.com/photo-1473496169904-658ba7c44d8a?w=900&q=80",  cat: "street",    title: { zh: "黄色出租车", en: "Yellow Cab" },      loc: { zh: "纽约", en: "New York, US" },       date: "2023.10", privacy: "public" },
  { id: 6,  src: "https://images.unsplash.com/photo-1517021897933-0e0319cfbc28?w=900&q=80",  cat: "street",    title: { zh: "地铁线条", en: "Subway Lines" },      loc: { zh: "首尔", en: "Seoul, KR" },          date: "2024.03", privacy: "public" },
  { id: 7,  src: "https://images.unsplash.com/photo-1506905925346-21bda4d32df4?w=900&q=80",  cat: "landscape", title: { zh: "山中湖", en: "Mountain Lake" },       loc: { zh: "阿尔卑斯", en: "Alps, CH" },       date: "2023.07", privacy: "public" },
  { id: 8,  src: "https://images.unsplash.com/photo-1464822759023-fed622ff2c3b?w=900&q=80",  cat: "landscape", title: { zh: "雾中山口", en: "Foggy Pass" },        loc: { zh: "法罗群岛", en: "Faroe Islands" },  date: "2023.09", privacy: "public" },
  { id: 9,  src: "https://images.unsplash.com/photo-1507608616759-54f48f0af0ee?w=900&q=80",  cat: "life",      title: { zh: "晨间一壶", en: "Morning Pour" },      loc: { zh: "工作室", en: "Studio" },           date: "2024.04", privacy: "private" },
  { id: 10, src: "https://images.unsplash.com/photo-1495365200479-c4ed1d35e1aa?w=900&q=80",  cat: "life",      title: { zh: "阅读的光", en: "Reading Light" },     loc: { zh: "家中", en: "Home" },               date: "2024.01", privacy: "public" },
  { id: 11, src: "https://images.unsplash.com/photo-1465146344425-f00d5f5c8f07?w=900&q=80",  cat: "life",      title: { zh: "手与花瓣", en: "Hands & Petals" },    loc: { zh: "巴黎", en: "Paris, FR" },          date: "2023.06", privacy: "locked" },
  { id: 12, src: "https://images.unsplash.com/photo-1485081669829-bacb8c7bb1f3?w=900&q=80",  cat: "street",    title: { zh: "雨天的街", en: "Rainy Avenue" },      loc: { zh: "伦敦", en: "London, UK" },         date: "2024.06", privacy: "public" },
  { id: 13, src: "https://images.unsplash.com/photo-1500382017468-9049fed747ef?w=900&q=80",  cat: "landscape", title: { zh: "麦田", en: "Wheat Field" },           loc: { zh: "普罗旺斯", en: "Provence, FR" },   date: "2023.08", privacy: "public" },
  { id: 14, src: "https://images.unsplash.com/photo-1469474968028-56623f02e42e?w=900&q=80",  cat: "landscape", title: { zh: "穿过松林的光", en: "Sun Through Pines" }, loc: { zh: "俄勒冈", en: "Oregon, US" },   date: "2024.07", privacy: "public" },
  { id: 15, src: "https://images.unsplash.com/photo-1444930694458-01babe71870e?w=900&q=80",  cat: "life",      title: { zh: "安静的伴", en: "Quiet Companion" },   loc: { zh: "里斯本", en: "Lisbon, PT" },       date: "2024.09", privacy: "public" },
  { id: 16, src: "https://images.unsplash.com/photo-1455741292689-49d9bd47c6e9?w=900&q=80",  cat: "street",    title: { zh: "靠窗的座", en: "Window Seat" },       loc: { zh: "柏林", en: "Berlin, DE" },         date: "2024.02", privacy: "private" },
  { id: 17, src: "https://images.unsplash.com/photo-1469854523086-cc02fe5d8800?w=900&q=80",  cat: "landscape", title: { zh: "潮线", en: "Tideline" },              loc: { zh: "冰岛", en: "Iceland" },            date: "2023.11", privacy: "public" },
  { id: 18, src: "https://images.unsplash.com/photo-1501785888041-af3ef285b470?w=900&q=80",  cat: "landscape", title: { zh: "倒影", en: "Reflections" },           loc: { zh: "哈尔施塔特", en: "Hallstatt, AT" },date: "2024.10", privacy: "public" },
  { id: 19, src: "https://images.unsplash.com/photo-1502082553048-f009c37129b9?w=900&q=80&sat=-100", cat: "life", title: { zh: "初霜", en: "First Frost" },         loc: { zh: "院子", en: "Garden" },             date: "2025.01", privacy: "locked" },
  { id: 20, src: "https://images.unsplash.com/photo-1517677208171-0bc6725a3e60?w=900&q=80",  cat: "street",    title: { zh: "清晨六点的市场", en: "Market, 6 a.m." }, loc: { zh: "河内", en: "Hanoi, VN" },     date: "2024.12", privacy: "public" },
  { id: 21, src: "https://images.unsplash.com/photo-1502082553048-f009c37129b9?w=900&q=80&blur=10", cat: "landscape", title: { zh: "最后的光", en: "Last Light" },  loc: { zh: "多洛米蒂", en: "Dolomites, IT" }, date: "2025.02", privacy: "public" },
  { id: 22, src: "https://images.unsplash.com/photo-1490750967868-88aa4486c946?w=900&q=80",  cat: "life",      title: { zh: "花瓣习作", en: "Petal Study" },       loc: { zh: "工作室", en: "Studio" },           date: "2024.05", privacy: "public" },
];

const CATEGORIES = [
  { key: "all",       zh: "全部",  en: "All" },
  { key: "street",    zh: "街拍",  en: "Street" },
  { key: "landscape", zh: "风景",  en: "Landscape" },
  { key: "life",      zh: "生活",  en: "Life" },
];

const I18N = {
  zh: {
    siteName: "拾光集", siteSub: "Gathered Light",
    range: "2023 — 2025", count: (n) => `共 ${n} 帧`,
    issue: "拾光为集 · 个人影像手记",
    headline1: "拾起那些", headline2: "被光掠过", headline3: "的、", headline4: "缓慢而不起眼的", headline5: "寻常一刻。",
    intro: "「拾光集」——拾起的光，拾起的时辰。街角、天气、窗台上的一杯茶，均被轻轻收录于此，留于多年以后翻看。",
    frames: (n) => `${n} 帧`,
    updated: "更新于 2026.04",
    locked: "已加锁", private: "私密", unlocked: "已解锁", public: "公开",
    lockedTitle: "这是一张私藏",
    lockedHint: "请输入分享给朋友的口令。",
    passcode: "口令", unlock: "解锁", hint: "提示 · 1234",
    privateTitle: "这一帧是私密的", privateSub: "仅本人可见",
    prev: "上一张", next: "下一张", close: "关闭",
    zoomIn: "放大", zoomOut: "缩小", zoomReset: "重置",
    footerNote: "拾光为集，轻轻合上。仅与朋友分享，请勿转载。",
    edition: "© 2026 · 拾光集",
    admin: "管理", login: "登录", logout: "退出",
    panelTitle: "样式调节", layout: "布局", masonry: "瀑布流", grid: "网格",
    density: "密度", tight: "紧凑", default: "默认", spacious: "宽松",
    bg: "背景", pure: "纯白", warm: "暖白", cool: "冷白",
    accent: "强调色", caption: "标题", alwaysShow: "始终显示标题",
    lang: "语言",
  },
  en: {
    siteName: "Gathered Light", siteSub: "拾光集",
    range: "2023 — 2025", count: (n) => `${n} Frames`,
    issue: "Field Notebook · Personal Archive",
    headline1: "Quiet hours, ", headline2: "borrowed light,", headline3: " and ", headline4: "the small ordinary ", headline5: "in between.",
    intro: "“Shiguangji” — a gathering of light. Streets, weather, and the slow afternoons of rooms, kept here to be wandered through.",
    frames: (n) => `${n} frames`,
    updated: "Updated · Apr 2026",
    locked: "Locked", private: "Private", unlocked: "Unlocked", public: "Public",
    lockedTitle: "A quiet one",
    lockedHint: "Enter the passcode shared with friends.",
    passcode: "passcode", unlock: "Unlock", hint: "hint · 1234",
    privateTitle: "This frame is private", privateSub: "Owner only",
    prev: "Previous", next: "Next", close: "Close",
    zoomIn: "Zoom in", zoomOut: "Zoom out", zoomReset: "Reset",
    footerNote: "A small archive of gathered light. Shared with friends — please don't repost.",
    edition: "© 2026 · Shiguangji",
    admin: "Admin", login: "Sign in", logout: "Sign out",
    panelTitle: "Tweaks", layout: "Layout", masonry: "Masonry", grid: "Grid",
    density: "Density", tight: "Tight", default: "Default", spacious: "Spacious",
    bg: "Background", pure: "Pure", warm: "Warm", cool: "Cool",
    accent: "Accent", caption: "Captions", alwaysShow: "Always show",
    lang: "Language",
  }
};

// ─────────────────────────────────────────────────────────────────────────────
// API Client — 对接 Axum 后端；localStorage 只做 token 和离线缓存
// ─────────────────────────────────────────────────────────────────────────────
const API_BASE = window.API_BASE || "";
const ACCESS_TOKEN_KEY = "shiguangji_access_token_v1";
const REFRESH_TOKEN_KEY = "shiguangji_refresh_token_v1";

const Api = {
  async request(path, options = {}, retry = true) {
    const { auth = true, ...fetchOptions } = options;
    const headers = new Headers(fetchOptions.headers || {});
    let body = fetchOptions.body;
    if (body && typeof body !== "string") {
      body = JSON.stringify(body);
      if (!headers.has("content-type")) headers.set("content-type", "application/json");
    }

    const token = localStorage.getItem(ACCESS_TOKEN_KEY);
    if (auth && token && !headers.has("authorization")) headers.set("authorization", `Bearer ${token}`);

    const resp = await fetch(`${API_BASE}${path}`, { ...fetchOptions, headers, body });
    if (resp.status === 401 && retry && auth && localStorage.getItem(REFRESH_TOKEN_KEY)) {
      const refreshed = await Api.refresh();
      if (refreshed) return Api.request(path, options, false);
    }

    const text = await resp.text();
    let data = null;
    if (text) {
      try { data = JSON.parse(text); }
      catch { data = { message: text }; }
    }
    if (!resp.ok) throw new Error(data?.message || resp.statusText || "Request failed");
    return data;
  },
  async login(password) {
    const data = await Api.request("/api/v1/auth/login", {
      method: "POST",
      auth: false,
      body: { email: "admin@gathered.local", password },
    }, false);
    Api.saveTokens(data);
    return data;
  },
  async refresh() {
    const refreshToken = localStorage.getItem(REFRESH_TOKEN_KEY);
    if (!refreshToken) return false;
    try {
      const data = await Api.request("/api/v1/auth/refresh", {
        method: "POST",
        auth: false,
        body: { refresh_token: refreshToken },
      }, false);
      Api.saveTokens(data);
      return true;
    } catch {
      Api.clearTokens();
      return false;
    }
  },
  async logout() {
    try { await Api.request("/api/v1/auth/logout", { method: "POST" }); }
    finally { Api.clearTokens(); }
  },
  saveTokens(data) {
    localStorage.setItem(ACCESS_TOKEN_KEY, data.access_token);
    localStorage.setItem(REFRESH_TOKEN_KEY, data.refresh_token);
    localStorage.setItem(AUTH_KEY, "1");
  },
  clearTokens() {
    localStorage.removeItem(ACCESS_TOKEN_KEY);
    localStorage.removeItem(REFRESH_TOKEN_KEY);
    localStorage.removeItem(AUTH_KEY);
  },
  listPhotos(admin = false) {
    return Api.request(admin ? "/api/v1/admin/photos" : "/api/v1/photos");
  },
  createPhoto(photo) {
    return Api.request("/api/v1/admin/photos", { method: "POST", body: photo });
  },
  updatePhoto(photo) {
    return Api.request(`/api/v1/admin/photos/${photo.id}`, { method: "PUT", body: photo });
  },
  deletePhoto(id) {
    return Api.request(`/api/v1/admin/photos/${id}`, { method: "DELETE" });
  },
  updatePrivacy(id, privacy) {
    return Api.request(`/api/v1/admin/photos/${id}/privacy`, { method: "PATCH", body: { privacy } });
  },
  unlockPhoto(id, passcode) {
    return Api.request(`/api/v1/photos/${id}/unlock`, { method: "POST", auth: false, body: { passcode } });
  },
  resetPhotos() {
    return Api.request("/api/v1/admin/photos/reset", { method: "POST" });
  },
};

const Store = {
  _photos: null,
  _emit() {
    window.dispatchEvent(new CustomEvent("shiguang:photos-changed"));
  },
  _cache(photos) {
    Store._photos = photos;
    localStorage.setItem(STORAGE_KEY, JSON.stringify(photos));
    Store._emit();
  },
  _filter(photos, admin = false) {
    return admin ? photos : photos.filter((p) => p.privacy !== "private");
  },
  load(admin = false) {
    try {
      if (Store._photos) return Store._filter(Store._photos, admin);
      const raw = localStorage.getItem(STORAGE_KEY);
      if (!raw) return Store._filter(DEFAULT_PHOTOS, admin);
      const parsed = JSON.parse(raw);
      if (!Array.isArray(parsed) || parsed.length === 0) return Store._filter(DEFAULT_PHOTOS, admin);
      Store._photos = parsed;
      return Store._filter(parsed, admin);
    } catch { return Store._filter(DEFAULT_PHOTOS, admin); }
  },
  save(photos) {
    Store._cache(photos);
  },
  reset() {
    Store._photos = null;
    localStorage.removeItem(STORAGE_KEY);
    Store._emit();
  },
  async fetchPhotos(admin = false) {
    try {
      const photos = await Api.listPhotos(admin);
      Store._cache(photos);
      return Store._filter(Store._photos, admin);
    } catch (err) {
      console.warn("photos api fallback:", err);
      return Store.load(admin);
    }
  },
  // Auth
  isAuthed() { return !!localStorage.getItem(ACCESS_TOKEN_KEY) || !!localStorage.getItem(REFRESH_TOKEN_KEY); },
  async signIn(pw) {
    try { await Api.login(pw); return true; }
    catch { return false; }
  },
  async signOut() { await Api.logout(); },
  async createPhoto(photo) {
    const created = await Api.createPhoto(photo);
    await Store.fetchPhotos(true);
    return created;
  },
  async updatePhoto(photo) {
    const updated = await Api.updatePhoto(photo);
    await Store.fetchPhotos(true);
    return updated;
  },
  async deletePhoto(id) {
    await Api.deletePhoto(id);
    await Store.fetchPhotos(true);
  },
  async updatePrivacy(id, privacy) {
    const updated = await Api.updatePrivacy(id, privacy);
    await Store.fetchPhotos(true);
    return updated;
  },
  async resetRemote() {
    const photos = await Api.resetPhotos();
    Store._cache(photos);
    return photos;
  },
  async unlock(id, passcode) {
    try {
      const data = await Api.unlockPhoto(id, passcode);
      return !!data?.unlocked;
    } catch {
      return false;
    }
  },
  // Lang
  getLang() { return localStorage.getItem(LANG_KEY) || "zh"; },
  setLang(l) { localStorage.setItem(LANG_KEY, l); window.dispatchEvent(new CustomEvent("shiguang:lang-changed")); },
};

function usePhotos(options = {}) {
  const admin = !!options.admin;
  const enabled = options.enabled !== false;
  const [photos, setPhotos] = React.useState(() => Store.load(admin));
  React.useEffect(() => {
    let alive = true;
    const h = () => setPhotos(Store.load(admin));
    window.addEventListener("shiguang:photos-changed", h);
    window.addEventListener("storage", h);
    if (enabled) Store.fetchPhotos(admin).then((rows) => { if (alive) setPhotos(rows); });
    return () => {
      alive = false;
      window.removeEventListener("shiguang:photos-changed", h);
      window.removeEventListener("storage", h);
    };
  }, [admin, enabled]);
  return photos;
}

function useLang() {
  const [lang, setLangState] = React.useState(() => Store.getLang());
  React.useEffect(() => {
    const h = () => setLangState(Store.getLang());
    window.addEventListener("shiguang:lang-changed", h);
    return () => window.removeEventListener("shiguang:lang-changed", h);
  }, []);
  const set = (l) => { Store.setLang(l); setLangState(l); };
  return [lang, set, I18N[lang]];
}

Object.assign(window, { Api, Store, usePhotos, useLang, CATEGORIES, I18N, DEFAULT_PHOTOS, ADMIN_PASS });
