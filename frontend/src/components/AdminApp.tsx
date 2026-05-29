import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { adminCategories, adminSettings, adminTags, adminUsers, api, clearSession, hasSession, login, logout, settings } from "../api/client";
import type { HeroCopy } from "../api/client";
import { categories, type Dictionary } from "../i18n";
import { useDebounce } from "../hooks/useDebounce";
import { useLang } from "../hooks/useLang";
import type {
  BulkResp,
  BulkTagMode,
  CategoryAdminDto,
  DashboardResp,
  NewCategoryReq,
  NewTagReq,
  NewUserReq,
  Photo,
  PhotoPayload,
  Privacy,
  Tag,
  UpdateCategoryReq,
  UpdateTagReq,
  UpdateUserReq,
  UserListItem,
  UserRole
} from "../types";
import {
  CloseIcon,
  DashboardIcon,
  ExternalLinkIcon,
  FolderIcon,
  MenuIcon,
  PaintBucketIcon,
  PhotosIcon,
  PlusIcon,
  SearchIcon,
  SignOutIcon,
  TagIcon,
  UsersIcon
} from "./Icons";
import type { User } from "../types";

type AdminTab = "dashboard" | "photos" | "users" | "tags" | "categories" | "appearance";
type ThemeName = "warm" | "cool" | "bold";

type FormState = {
  slug: string;
  src: string;
  titleZh: string;
  titleEn: string;
  locZh: string;
  locEn: string;
  captionZh: string;
  captionEn: string;
  altTextZh: string;
  altTextEn: string;
  cat: string;
  date: string;
  privacy: Privacy;
  tagIds: number[];
  passcode: string;
};

const emptyForm: FormState = {
  slug: "",
  src: "",
  titleZh: "",
  titleEn: "",
  locZh: "",
  locEn: "",
  captionZh: "",
  captionEn: "",
  altTextZh: "",
  altTextEn: "",
  cat: "street",
  date: "2026.05",
  privacy: "public",
  tagIds: [],
  passcode: ""
};

const themeOptions: Array<{
  id: ThemeName;
  nameKey: keyof Dictionary;
  descKey: keyof Dictionary;
  colors: string[];
}> = [
  {
    id: "warm",
    nameKey: "themeWarmName",
    descKey: "themeWarmDesc",
    colors: ["#fbfaf7", "#f4f1ea", "#1a1814", "#3f5f4a"]
  },
  {
    id: "cool",
    nameKey: "themeCoolName",
    descKey: "themeCoolDesc",
    colors: ["#ffffff", "#f7f7f8", "#18181b", "#18181b"]
  },
  {
    id: "bold",
    nameKey: "themeBoldName",
    descKey: "themeBoldDesc",
    colors: ["#ffffff", "#f9f9f9", "#121212", "#b91c1c"]
  }
];

function isTheme(value: string): value is ThemeName {
  return themeOptions.some((option) => option.id === value);
}

function toForm(photo?: Photo): FormState {
  if (!photo) return emptyForm;
  return {
    slug: photo.slug,
    src: photo.src,
    titleZh: photo.title.zh,
    titleEn: photo.title.en,
    locZh: photo.loc.zh,
    locEn: photo.loc.en,
    captionZh: photo.caption?.zh ?? "",
    captionEn: photo.caption?.en ?? "",
    altTextZh: photo.alt_text?.zh ?? "",
    altTextEn: photo.alt_text?.en ?? "",
    cat: photo.cat,
    date: photo.date,
    privacy: photo.privacy,
    tagIds: photo.tags.map((tag) => tag.id),
    passcode: photo.passcode ?? ""
  };
}

function toPayload(form: FormState): PhotoPayload {
  const payload: PhotoPayload = {
    src: form.src,
    cat: form.cat,
    title: {
      zh: form.titleZh,
      en: form.titleEn || form.titleZh
    },
    loc: {
      zh: form.locZh,
      en: form.locEn || form.locZh
    },
    date: form.date,
    privacy: form.privacy,
    tag_ids: form.tagIds
  };
  if (form.slug.trim()) payload.slug = form.slug.trim();
  if (form.captionZh.trim() || form.captionEn.trim()) {
    payload.caption = {
      zh: form.captionZh,
      en: form.captionEn || form.captionZh
    };
  }
  if (form.altTextZh.trim() || form.altTextEn.trim()) {
    payload.alt_text = {
      zh: form.altTextZh,
      en: form.altTextEn || form.altTextZh
    };
  }
  // Locked 才传 passcode；非 locked 不传，后端 new_photo_from_req 会自动清空
  if (form.privacy === "locked" && form.passcode.trim()) {
    payload.passcode = form.passcode.trim();
  }
  return payload;
}

export function AdminApp() {
  const { lang, setLang, t } = useLang();
  const [authed, setAuthed] = useState(hasSession);
  const [activeTab, setActiveTab] = useState<AdminTab>("dashboard");
  const [me, setMe] = useState<User | null>(null);
  const [sidebarOpen, setSidebarOpen] = useState(false);
  const [photos, setPhotos] = useState<Photo[]>([]);
  const [totalPhotos, setTotalPhotos] = useState(0);
  const [page, setPage] = useState(1);
  const [loading, setLoading] = useState(authed);
  const [error, setError] = useState<string | null>(null);
  const [filter, setFilter] = useState("all");
  const [search, setSearch] = useState("");
  const [selected, setSelected] = useState<Set<number>>(() => new Set());
  const [bulkAction, setBulkAction] = useState<"delete" | "privacy" | "tags" | null>(null);
  const [editing, setEditing] = useState<Photo | null>(null);
  const [creating, setCreating] = useState(false);
  const [notice, setNotice] = useState<string | null>(null);
  const [recoveringId, setRecoveringId] = useState<number | null>(null);
  const debouncedSearch = useDebounce(search, 300);
  const pageSize = 20;

  const categoryFilter = useMemo(() => {
    if (filter === "all" || ["public", "locked", "private"].includes(filter)) return "";
    return filter;
  }, [filter]);

  const privacyFilter = useMemo(() => {
    return ["public", "locked", "private"].includes(filter) ? filter : "";
  }, [filter]);

  const totalPages = Math.max(1, Math.ceil(totalPhotos / pageSize));

  const loadPhotos = useCallback(async () => {
    if (!authed) return;
    setLoading(true);
    setError(null);
    try {
      const list = await api.adminPhotos.list(
        page,
        pageSize,
        debouncedSearch.trim(),
        categoryFilter,
        privacyFilter
      );
      setPhotos(list.items);
      setTotalPhotos(list.total);
      if (list.page > 1 && list.items.length === 0) setPage(1);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to load photos");
    } finally {
      setLoading(false);
    }
  }, [authed, categoryFilter, debouncedSearch, page, privacyFilter]);

  useEffect(() => {
    void loadPhotos();
  }, [loadPhotos]);

  useEffect(() => {
    setPage(1);
  }, [debouncedSearch, filter]);

  useEffect(() => {
    setSelected((prev) => new Set(photos.filter((photo) => prev.has(photo.id)).map((photo) => photo.id)));
  }, [photos]);

  useEffect(() => {
    if (!authed) {
      setMe(null);
      return;
    }
    let alive = true;
    api.me().then((resp) => { if (alive) setMe(resp.user); }).catch(() => {});
    return () => { alive = false; };
  }, [authed]);

  useEffect(() => {
    setSidebarOpen(false);
  }, [activeTab]);

  const stats = useMemo(() => ({
    total: totalPhotos,
    public: photos.filter((photo) => photo.privacy === "public").length,
    locked: photos.filter((photo) => photo.privacy === "locked").length,
    private: photos.filter((photo) => photo.privacy === "private").length
  }), [photos, totalPhotos]);

  const closeForm = () => {
    setEditing(null);
    setCreating(false);
  };

  const toggleSelected = (id: number) => {
    setSelected((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  };

  const selectAll = () => setSelected(new Set(photos.map((photo) => photo.id)));
  const clearSelection = () => setSelected(new Set());
  const invertSelection = () => {
    setSelected(new Set(photos.filter((photo) => !selected.has(photo.id)).map((photo) => photo.id)));
  };

  const onBulkDone = async (resp: BulkResp) => {
    setBulkAction(null);
    clearSelection();
    setNotice(bulkMessage(resp, lang));
    await loadPhotos();
  };

  const save = async (payload: PhotoPayload) => {
    if (editing) await api.adminPhotos.update(editing.id, payload);
    else await api.adminPhotos.create(payload);
    await loadPhotos();
    closeForm();
  };

  const remove = async (photo: Photo) => {
    if (!window.confirm(lang === "zh" ? "确认删除这一帧？" : "Delete this frame?")) return;
    await api.adminPhotos.remove(photo.id);
    await loadPhotos();
  };

  const cyclePrivacy = async (photo: Photo) => {
    const order: Privacy[] = ["public", "locked", "private"];
    const next = order[(order.indexOf(photo.privacy) + 1) % order.length];
    await api.adminPhotos.updatePrivacy(photo.id, next);
    await loadPhotos();
  };

  const recoverUrl = async (photo: Photo) => {
    setRecoveringId(photo.id);
    setError(null);
    try {
      const recovered = await api.adminPhotos.recoverUrl(photo.id);
      setPhotos((prev) => prev.map((item) => item.id === recovered.id ? recovered : item));
      setNotice(t.recoverUrlDone as string);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Recover failed");
    } finally {
      setRecoveringId(null);
    }
  };

  const reset = async () => {
    if (!window.confirm(lang === "zh" ? "重置为种子数据？所有改动将丢失。" : "Reset to seed data?")) return;
    await api.adminPhotos.resetSeeds();
    setPage(1);
    await loadPhotos();
  };

  const signOut = async () => {
    await logout();
    setAuthed(false);
  };

  if (!authed) {
    return <LoginPanel onAuthed={() => setAuthed(true)} />;
  }

  const tabMeta: Record<AdminTab, { title: string; subtitle: string }> = {
    dashboard: { title: t.dashboard as string, subtitle: t.dashboardSub as string },
    photos: { title: t.photos as string, subtitle: t.photosSub as string },
    users: { title: t.users as string, subtitle: t.usersSub as string },
    tags: { title: t.tags as string, subtitle: t.tagsSub as string },
    categories: { title: t.categoriesAdmin as string, subtitle: t.categoriesSub as string },
    appearance: { title: t.appearance as string, subtitle: t.appearanceSub as string }
  };

  const currentMeta = tabMeta[activeTab];

  return (
    <div className={`admin-shell sidebar${sidebarOpen ? " open" : ""}`}>
      <aside className="admin-sidebar" aria-label="Admin navigation">
        <div className="sidebar-brand">
          <a href="/" className="brand stacked">
            <span>{t.siteName}</span>
            <small>{t.admin}</small>
          </a>
        </div>

        <nav className="sidebar-nav">
          <div className="nav-group">
            <span className="nav-group-label">{t.navContent as string}</span>
            <SidebarItem icon={<DashboardIcon />} label={t.dashboard as string} active={activeTab === "dashboard"} onClick={() => setActiveTab("dashboard")} />
            <SidebarItem icon={<PhotosIcon />} label={t.photos as string} active={activeTab === "photos"} onClick={() => setActiveTab("photos")} />
          </div>

          <div className="nav-group">
            <span className="nav-group-label">{t.navTaxonomy as string}</span>
            <SidebarItem icon={<TagIcon />} label={t.tags as string} active={activeTab === "tags"} onClick={() => setActiveTab("tags")} />
            <SidebarItem icon={<FolderIcon />} label={t.categoriesAdmin as string} active={activeTab === "categories"} onClick={() => setActiveTab("categories")} />
          </div>

          <div className="nav-group">
            <span className="nav-group-label">{t.navAccess as string}</span>
            <SidebarItem icon={<UsersIcon />} label={t.users as string} active={activeTab === "users"} onClick={() => setActiveTab("users")} />
          </div>

          <div className="nav-group">
            <span className="nav-group-label">{t.navSystem as string}</span>
            <SidebarItem icon={<PaintBucketIcon />} label={t.appearance as string} active={activeTab === "appearance"} onClick={() => setActiveTab("appearance")} />
          </div>
        </nav>

        <div className="sidebar-footer">
          {me && (
            <div className="sidebar-user">
              <div className="avatar">{(me.display_name || me.email).slice(0, 1).toUpperCase()}</div>
              <div className="sidebar-user-text">
                <strong>{me.display_name || me.email}</strong>
                <span>{me.role}</span>
              </div>
            </div>
          )}
          <div className="sidebar-tools">
            <div className="lang-switch">
              <button className={lang === "zh" ? "active" : ""} onClick={() => setLang("zh")}>中</button>
              <button className={lang === "en" ? "active" : ""} onClick={() => setLang("en")}>EN</button>
            </div>
            <a className="sidebar-link" href="/"><ExternalLinkIcon /> <span>{t.viewSite as string}</span></a>
            <button className="sidebar-link" onClick={() => void signOut()}><SignOutIcon /> <span>{t.logout}</span></button>
          </div>
        </div>
      </aside>

      {sidebarOpen && <div className="sidebar-overlay" onClick={() => setSidebarOpen(false)} />}

      <main className="admin-workspace">
        <header className="workspace-topbar">
          <button className="icon-only mobile-only" aria-label="Menu" onClick={() => setSidebarOpen((v) => !v)}>
            <MenuIcon />
          </button>
          <div className="crumbs">
            <span>{t.admin}</span>
            <span className="crumbs-sep">/</span>
            <span className="crumbs-current">{currentMeta.title}</span>
          </div>
          <div className="workspace-tools">
            {me && <span className="hello">{t.welcomeBack as string}, {me.display_name || me.email.split("@")[0]}</span>}
          </div>
        </header>

        <header className="page-header">
          <div className="page-header-text">
            <h1>{currentMeta.title}</h1>
            <p>{currentMeta.subtitle}</p>
          </div>
          <div className="page-header-actions">
            {activeTab === "photos" && (
              <>
                <button className="secondary" onClick={() => void reset()}>{t.reset}</button>
                <button className="primary" onClick={() => setCreating(true)}><PlusIcon /> {t.newPhoto}</button>
              </>
            )}
          </div>
        </header>

        <div className="workspace-body">
          {activeTab === "dashboard" ? (
            <DashboardView />
          ) : activeTab === "photos" ? (
            <>
              <section className="stats-grid">
                <Stat label={t.total as string} value={stats.total} />
                <Stat label={t.public as string} value={stats.public} />
                <Stat label={t.locked as string} value={stats.locked} />
                <Stat label={t.private as string} value={stats.private} />
              </section>

              <section className="toolbar card">
                <div className="filters chips">
                  <FilterButton active={filter === "all"} onClick={() => setFilter("all")} label="All" count={stats.total} />
                  <FilterButton active={filter === "public"} onClick={() => setFilter("public")} label={t.public as string} count={stats.public} />
                  <FilterButton active={filter === "locked"} onClick={() => setFilter("locked")} label={t.locked as string} count={stats.locked} />
                  <FilterButton active={filter === "private"} onClick={() => setFilter("private")} label={t.private as string} count={stats.private} />
                  {categories.filter((item) => item.key !== "all").map((item) => (
                    <FilterButton key={item.key} active={filter === item.key} onClick={() => setFilter(item.key)} label={item[lang]} count={photos.filter((photo) => photo.cat === item.key).length} />
                  ))}
                </div>
                <div className="toolbar-actions">
                  <div className="search-input">
                    <SearchIcon />
                    <input
                      type="search"
                      value={search}
                      onChange={(event) => setSearch(event.target.value)}
                      placeholder={t.searchPlaceholder as string}
                    />
                  </div>
                </div>
              </section>

              {selected.size > 0 && (
                <section className="bulk-toolbar">
                  <span>{(t.bulkSelected as (count: number) => string)(selected.size)}</span>
                  <button onClick={selectAll}>{t.selectAll as string}</button>
                  <button onClick={invertSelection}>{t.invertSelection as string}</button>
                  <button onClick={clearSelection}>{t.clearSelection as string}</button>
                  <span className="bulk-sep" />
                  <button className="danger" onClick={() => setBulkAction("delete")}>{t.bulkDelete as string}</button>
                  <button onClick={() => setBulkAction("privacy")}>{t.bulkPrivacy as string}</button>
                  <button onClick={() => setBulkAction("tags")}>{t.bulkTags as string}</button>
                </section>
              )}

              {loading && <div className="state-line">Loading...</div>}
              {error && <div className="state-line error">{error}</div>}
              {notice && <div className="state-line success">{notice}</div>}

              <section className="admin-table card">
                <div className="table-head">
                  <span>
                    <input
                      className="row-checkbox"
                      type="checkbox"
                      checked={photos.length > 0 && selected.size === photos.length}
                      onChange={(event) => event.target.checked ? selectAll() : clearSelection()}
                    />
                  </span>
                  <span>Image</span>
                  <span>{t.title as string}</span>
                  <span>{t.category as string}</span>
                  <span>{t.date as string}</span>
                  <span>{t.privacy as string}</span>
                  <span>{t.actions as string}</span>
                </div>
                {photos.length === 0 ? (
                  <div className="empty-state">{t.noFrames as string}</div>
                ) : photos.map((photo) => (
                  <article className="table-row" key={photo.id}>
                    <input
                      className="row-checkbox"
                      type="checkbox"
                      checked={selected.has(photo.id)}
                      onChange={() => toggleSelected(photo.id)}
                    />
                    <img src={photo.src} alt="" loading="lazy" decoding="async" />
                    <div className="row-title">
                      <strong>{photo.title[lang]}</strong>
                      <span>{photo.loc[lang]}</span>
                    </div>
                    <span>{categories.find((item) => item.key === photo.cat)?.[lang]}</span>
                    <span>{photo.date}</span>
                    <div className="privacy-cell">
                      <button className={`privacy-pill ${photo.privacy}`} onClick={() => void cyclePrivacy(photo)}>{privacyLabel(photo.privacy, lang)}</button>
                      {photo.privacy === "locked" && photo.passcode && (
                        <code
                          className="passcode-chip"
                          title={t.passcodeHint as string}
                          onClick={() => {
                            void navigator.clipboard.writeText(photo.passcode!);
                            setNotice(t.passcodeCopied as string);
                          }}
                        >
                          {photo.passcode}
                        </code>
                      )}
                    </div>
                    <div className="row-actions">
                      <button onClick={() => setEditing(photo)}>{t.edit}</button>
                      <button onClick={() => void recoverUrl(photo)} disabled={recoveringId === photo.id}>
                        {recoveringId === photo.id ? t.recoveringUrl as string : t.recoverUrl as string}
                      </button>
                      <button onClick={() => void remove(photo)}>{t.delete}</button>
                    </div>
                  </article>
                ))}
              </section>

              <nav className="toolbar pagination">
                <div className="page-info">{(t.pageOf as (page: number, total: number) => string)(page, totalPages)} · {totalPhotos}</div>
                <div className="toolbar-actions">
                  <button className="secondary" disabled={page <= 1} onClick={() => setPage((value) => Math.max(1, value - 1))}>{t.prevPage as string}</button>
                  <button className="secondary" disabled={page >= totalPages} onClick={() => setPage((value) => value + 1)}>{t.nextPage as string}</button>
                </div>
              </nav>

              <p className="admin-footnote">{t.apiSaved as string}</p>
            </>
          ) : activeTab === "appearance" ? (
            <AppearanceView onNotice={setNotice} />
          ) : activeTab === "users" ? (
            <UsersView />
          ) : activeTab === "tags" ? (
            <TagsView />
          ) : (
            <CategoriesView />
          )}
        </div>
      </main>

      {(creating || editing) && (
        <PhotoDialog
          initial={editing ?? undefined}
          onClose={closeForm}
          onSave={save}
          onNotice={setNotice}
        />
      )}

      {bulkAction === "delete" && (
        <BulkDeleteModal ids={[...selected]} onClose={() => setBulkAction(null)} onDone={onBulkDone} />
      )}
      {bulkAction === "privacy" && (
        <BulkPrivacyModal ids={[...selected]} onClose={() => setBulkAction(null)} onDone={onBulkDone} />
      )}
      {bulkAction === "tags" && (
        <BulkTagsModal ids={[...selected]} onClose={() => setBulkAction(null)} onDone={onBulkDone} />
      )}
    </div>
  );
}

function bulkMessage(resp: BulkResp, lang: "zh" | "en") {
  return lang === "zh"
    ? `已处理 ${resp.affected} 张，跳过 ${resp.skipped.length} 张`
    : `Processed ${resp.affected}, skipped ${resp.skipped.length}`;
}

function BulkDeleteModal({ ids, onClose, onDone }: { ids: number[]; onClose: () => void; onDone: (resp: BulkResp) => Promise<void> }) {
  const { t } = useLang();
  const [submitting, setSubmitting] = useState(false);
  const submit = async () => {
    setSubmitting(true);
    try {
      await onDone(await api.adminPhotos.bulkDelete({ ids }));
    } finally {
      setSubmitting(false);
    }
  };
  return (
    <ConfirmShell title={t.bulkDelete as string} onClose={onClose}>
      <p>{(t.bulkDeleteConfirm as (count: number) => string)(ids.length)}</p>
      <footer className="dialog-actions">
        <button type="button" className="secondary" onClick={onClose}>{t.cancel}</button>
        <button type="button" className="primary danger" disabled={submitting} onClick={() => void submit()}>{submitting ? "..." : t.save}</button>
      </footer>
    </ConfirmShell>
  );
}

function BulkPrivacyModal({ ids, onClose, onDone }: { ids: number[]; onClose: () => void; onDone: (resp: BulkResp) => Promise<void> }) {
  const { lang, t } = useLang();
  const [privacy, setPrivacy] = useState<Privacy>("locked");
  const [submitting, setSubmitting] = useState(false);
  const submit = async () => {
    setSubmitting(true);
    try {
      await onDone(await api.adminPhotos.bulkPrivacy({ ids, privacy }));
    } finally {
      setSubmitting(false);
    }
  };
  return (
    <ConfirmShell title={t.bulkPrivacy as string} onClose={onClose}>
      <div className="bulk-options">
        {(["public", "locked", "private"] as Privacy[]).map((item) => (
          <label key={item}><input type="radio" checked={privacy === item} onChange={() => setPrivacy(item)} /> {privacyLabel(item, lang)}</label>
        ))}
      </div>
      <footer className="dialog-actions">
        <button type="button" className="secondary" onClick={onClose}>{t.cancel}</button>
        <button type="button" className="primary" disabled={submitting} onClick={() => void submit()}>{submitting ? "..." : t.save}</button>
      </footer>
    </ConfirmShell>
  );
}

function BulkTagsModal({ ids, onClose, onDone }: { ids: number[]; onClose: () => void; onDone: (resp: BulkResp) => Promise<void> }) {
  const { lang, t } = useLang();
  const [mode, setMode] = useState<BulkTagMode>("replace");
  const [tagIds, setTagIds] = useState<Set<number>>(() => new Set());
  const [tags, setTags] = useState<Tag[]>([]);
  const [submitting, setSubmitting] = useState(false);
  useEffect(() => {
    void adminTags.list(1, 200).then((resp) => setTags(resp.items));
  }, []);
  const toggle = (id: number) => setTagIds((prev) => {
    const next = new Set(prev);
    if (next.has(id)) next.delete(id);
    else next.add(id);
    return next;
  });
  const submit = async () => {
    setSubmitting(true);
    try {
      await onDone(await api.adminPhotos.bulkSetTags({ ids, tag_ids: [...tagIds], mode }));
    } finally {
      setSubmitting(false);
    }
  };
  return (
    <ConfirmShell title={t.bulkTags as string} onClose={onClose}>
      <div className="bulk-options">
        <label><input type="radio" checked={mode === "replace"} onChange={() => setMode("replace")} /> {t.replaceMode as string}</label>
        <label><input type="radio" checked={mode === "append"} onChange={() => setMode("append")} /> {t.appendMode as string}</label>
        {tags.map((tag) => (
          <label key={tag.id}><input type="checkbox" checked={tagIds.has(tag.id)} onChange={() => toggle(tag.id)} /> {tag.name[lang]} · {tag.slug}</label>
        ))}
      </div>
      <footer className="dialog-actions">
        <button type="button" className="secondary" onClick={onClose}>{t.cancel}</button>
        <button type="button" className="primary" disabled={submitting} onClick={() => void submit()}>{submitting ? "..." : t.save}</button>
      </footer>
    </ConfirmShell>
  );
}

function ConfirmShell({ title, onClose, children }: { title: string; onClose: () => void; children: React.ReactNode }) {
  const { t } = useLang();
  return (
    <div className="modal-backdrop" onMouseDown={onClose}>
      <section className="photo-dialog bulk-dialog" onMouseDown={(event) => event.stopPropagation()}>
        <header>
          <h2>{title}</h2>
          <button type="button" className="icon-only" onClick={onClose} aria-label={t.close as string}><CloseIcon /></button>
        </header>
        {children}
      </section>
    </div>
  );
}

function DashboardView() {
  const { lang, t } = useLang();
  const [data, setData] = useState<DashboardResp | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let alive = true;
    setLoading(true);
    setError(null);
    api.dashboard()
      .then((resp) => {
        if (alive) setData(resp);
      })
      .catch((err) => {
        if (alive) setError(err instanceof Error ? err.message : "Load dashboard failed");
      })
      .finally(() => {
        if (alive) setLoading(false);
      });
    return () => {
      alive = false;
    };
  }, []);

  if (loading) return <div className="state-line">Loading...</div>;
  if (error) return <div className="state-line error">{error}</div>;
  if (!data) return <div className="empty-state">{t.dashboard as string}</div>;

  const privacy = data.photos.by_privacy;
  const privacyRows = [
    { label: t.public as string, value: privacy.public },
    { label: t.locked as string, value: privacy.locked },
    { label: t.private as string, value: privacy.private }
  ];
  const maxPrivacy = Math.max(...privacyRows.map((row) => row.value), 1);
  const maxCategory = Math.max(...data.photos.by_category.map((row) => row.count), 1);

  return (
    <>
      <section className="stats-grid">
        <Stat label={t.photos as string} value={data.photos.total} />
        <Stat label={t.users as string} value={data.users_total} />
        <Stat label={t.tags as string} value={data.tags_total} />
        <Stat label={t.categoriesAdmin as string} value={data.categories_total} />
      </section>

      <section className="dashboard-grid">
        <div className="dashboard-panel">
          <h2>{t.byPrivacy as string}</h2>
          {privacyRows.map((row) => (
            <Bar key={row.label} label={row.label} value={row.value} max={maxPrivacy} />
          ))}
        </div>

        <div className="dashboard-panel">
          <h2>{t.byCategory as string}</h2>
          {data.photos.by_category.map((row) => (
            <Bar key={row.slug} label={row.name[lang]} value={row.count} max={maxCategory} />
          ))}
        </div>
      </section>

      <section className="dashboard-panel">
        <h2>{t.recentUploads as string}</h2>
        <div className="dashboard-recent">
          {data.photos.recent.map((photo) => (
            <article key={photo.id}>
              <img src={photo.src} alt="" loading="lazy" decoding="async" />
              <strong>{photo.title[lang]}</strong>
              <span>{formatDate(photo.created_at)}</span>
            </article>
          ))}
        </div>
      </section>

      <section className="dashboard-panel">
        <h2>{t.mediaQueue as string}</h2>
        <p className="dashboard-media-line">
          pending {data.media.pending} · processing {data.media.processing} · ready {data.media.ready} · failed {data.media.failed} · total {data.media.total}
        </p>
      </section>
    </>
  );
}

function AppearanceView({ onNotice }: { onNotice: (message: string) => void }) {
  const { t } = useLang();
  const defaultRange = t.range as string;
  const [current, setCurrent] = useState<ThemeName>("warm");
  const [range, setRange] = useState(defaultRange);
  const [rangeDraft, setRangeDraft] = useState(defaultRange);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState<ThemeName | null>(null);
  const [savingRange, setSavingRange] = useState(false);
  const [hero, setHero] = useState<HeroCopy | null>(null);
  const [savingHero, setSavingHero] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [notice, setLocalNotice] = useState<string | null>(null);

  useEffect(() => {
    let alive = true;
    setLoading(true);
    setError(null);
    settings
      .get()
      .then((resp) => {
        if (!alive) return;
        if (isTheme(resp.theme)) {
          setCurrent(resp.theme);
          document.documentElement.dataset.theme = resp.theme;
        }
        const nextRange = resp.range?.trim();
        if (nextRange) {
          setRange(nextRange);
          setRangeDraft(nextRange);
          document.documentElement.dataset.range = nextRange;
        }
        if (resp.hero) setHero(resp.hero);
      })
      .catch((err) => {
        if (alive) setError(err instanceof Error ? err.message : "Load theme failed");
      })
      .finally(() => {
        if (alive) setLoading(false);
      });
    return () => {
      alive = false;
    };
  }, []);

  const applyTheme = async (theme: ThemeName) => {
    setSaving(theme);
    setError(null);
    setLocalNotice(null);
    try {
      const resp = await adminSettings.updateTheme(theme);
      const nextTheme = isTheme(resp.theme) ? resp.theme : theme;
      const option = themeOptions.find((item) => item.id === nextTheme) ?? themeOptions[0];
      const themeName = t[option.nameKey] as string;
      const message = (t.themeApplied as (name: string) => string)(themeName);
      document.documentElement.dataset.theme = nextTheme;
      setCurrent(nextTheme);
      setLocalNotice(message);
      onNotice(message);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Update theme failed");
    } finally {
      setSaving(null);
    }
  };

  const saveRange = async () => {
    const nextRange = rangeDraft.trim();
    if (!nextRange || nextRange === range) return;

    setSavingRange(true);
    setError(null);
    setLocalNotice(null);
    try {
      const resp = await adminSettings.updateRange(nextRange);
      const savedRange = resp.range?.trim() || nextRange;
      if (isTheme(resp.theme)) {
        setCurrent(resp.theme);
        document.documentElement.dataset.theme = resp.theme;
      }
      setRange(savedRange);
      setRangeDraft(savedRange);
      document.documentElement.dataset.range = savedRange;
      setLocalNotice(t.siteRangeSaved as string);
      onNotice(t.siteRangeSaved as string);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Update range failed");
    } finally {
      setSavingRange(false);
    }
  };

  const saveHero = async () => {
    if (!hero) return;
    const cleaned: HeroCopy = {
      issue: { zh: hero.issue.zh.trim(), en: hero.issue.en.trim() },
      headline: { zh: hero.headline.zh.trim(), en: hero.headline.en.trim() },
      introLines: {
        zh: hero.introLines.zh.map((line) => line.trim()).filter(Boolean),
        en: hero.introLines.en.map((line) => line.trim()).filter(Boolean)
      }
    };
    if (
      !cleaned.issue.zh || !cleaned.issue.en ||
      !cleaned.headline.zh || !cleaned.headline.en ||
      !cleaned.introLines.zh.length || !cleaned.introLines.en.length
    ) {
      setError(t.heroLinesHint as string);
      return;
    }

    setSavingHero(true);
    setError(null);
    setLocalNotice(null);
    try {
      const resp = await adminSettings.updateHero(cleaned);
      if (resp.hero) setHero(resp.hero);
      setLocalNotice(t.heroSaved as string);
      onNotice(t.heroSaved as string);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Update hero failed");
    } finally {
      setSavingHero(false);
    }
  };

  return (
    <>
      {loading && <div className="state-line">Loading...</div>}
      {error && <div className="state-line error">{error}</div>}
      {notice && <div className="state-line success">{notice}</div>}

      <section className="theme-picker">
        {themeOptions.map((option) => {
          const active = option.id === current;
          return (
            <button
              key={option.id}
              type="button"
              className={`theme-card${active ? " active" : ""}`}
              disabled={saving !== null}
              onClick={() => void applyTheme(option.id)}
            >
              <span className="theme-card-top">
                <span className="theme-card-copy">
                  <strong>{t[option.nameKey] as string}</strong>
                  <span>{t[option.descKey] as string}</span>
                </span>
                {active && <span className="theme-check" aria-hidden="true">✓</span>}
              </span>
              <span className="theme-swatches" aria-hidden="true">
                {option.colors.map((color, index) => (
                  <span key={`${option.id}-${color}-${index}`} className="theme-swatch" style={{ background: color }} />
                ))}
              </span>
              {saving === option.id && <span className="theme-status">Saving...</span>}
            </button>
          );
        })}
      </section>

      <form
        className="appearance-setting"
        onSubmit={(event) => {
          event.preventDefault();
          void saveRange();
        }}
      >
        <div className="appearance-setting-copy">
          <strong>{t.siteRangeLabel as string}</strong>
          <span>{t.siteRangeHint as string}</span>
        </div>
        <div className="appearance-inline-form">
          <input
            value={rangeDraft}
            maxLength={32}
            onChange={(event) => setRangeDraft(event.target.value)}
          />
          <button
            type="submit"
            className="primary"
            disabled={savingRange || !rangeDraft.trim() || rangeDraft.trim() === range}
          >
            {savingRange ? "Saving..." : (t.save as string)}
          </button>
        </div>
      </form>

      {hero && (
        <form
          className="appearance-setting hero-setting"
          onSubmit={(event) => {
            event.preventDefault();
            void saveHero();
          }}
        >
          <div className="appearance-setting-copy">
            <strong>{t.heroLabel as string}</strong>
            <span>{t.heroHint as string}</span>
          </div>
          <div className="hero-fields">
            <label className="hero-field">
              <span>{t.heroIssueZh as string}</span>
              <input
                value={hero.issue.zh}
                maxLength={200}
                onChange={(event) =>
                  setHero({ ...hero, issue: { ...hero.issue, zh: event.target.value } })
                }
              />
            </label>
            <label className="hero-field">
              <span>{t.heroIssueEn as string}</span>
              <input
                value={hero.issue.en}
                maxLength={200}
                onChange={(event) =>
                  setHero({ ...hero, issue: { ...hero.issue, en: event.target.value } })
                }
              />
            </label>
            <label className="hero-field">
              <span>{t.heroHeadlineZh as string}</span>
              <input
                value={hero.headline.zh}
                maxLength={200}
                onChange={(event) =>
                  setHero({ ...hero, headline: { ...hero.headline, zh: event.target.value } })
                }
              />
            </label>
            <label className="hero-field">
              <span>{t.heroHeadlineEn as string}</span>
              <input
                value={hero.headline.en}
                maxLength={200}
                onChange={(event) =>
                  setHero({ ...hero, headline: { ...hero.headline, en: event.target.value } })
                }
              />
            </label>
            <label className="hero-field hero-field-wide">
              <span>{t.heroLinesZh as string}</span>
              <textarea
                rows={3}
                value={hero.introLines.zh.join("\n")}
                onChange={(event) =>
                  setHero({ ...hero, introLines: { ...hero.introLines, zh: event.target.value.split("\n") } })
                }
              />
            </label>
            <label className="hero-field hero-field-wide">
              <span>{t.heroLinesEn as string}</span>
              <textarea
                rows={3}
                value={hero.introLines.en.join("\n")}
                onChange={(event) =>
                  setHero({ ...hero, introLines: { ...hero.introLines, en: event.target.value.split("\n") } })
                }
              />
            </label>
          </div>
          <div className="appearance-inline-form">
            <span className="hero-lines-hint">{t.heroLinesHint as string}</span>
            <button type="submit" className="primary" disabled={savingHero}>
              {savingHero ? "Saving..." : (t.save as string)}
            </button>
          </div>
        </form>
      )}
    </>
  );
}

function Bar({ label, value, max }: { label: string; value: number; max: number }) {
  const pct = max > 0 ? (value / max) * 100 : 0;
  return (
    <div className="dashboard-bar-row">
      <span className="dashboard-bar-label">{label}</span>
      <div className="dashboard-bar-track">
        <div className="dashboard-bar-fill" style={{ width: `${pct}%` }} />
      </div>
      <span className="dashboard-bar-value">{value}</span>
    </div>
  );
}

function LoginPanel({ onAuthed }: { onAuthed: () => void }) {
  const { lang, setLang, t } = useLang();
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [error, setError] = useState<string | null>(null);

  const submit = async (event: React.FormEvent) => {
    event.preventDefault();
    setError(null);
    try {
      await login(email, password);
      onAuthed();
    } catch (err) {
      clearSession();
      setError(err instanceof Error ? err.message : "Login failed");
    }
  };

  return (
    <div className="login-screen">
      <div className="lang-switch floating">
        <button className={lang === "zh" ? "active" : ""} onClick={() => setLang("zh")}>中</button>
        <button className={lang === "en" ? "active" : ""} onClick={() => setLang("en")}>EN</button>
      </div>
      <form className="login-card" onSubmit={(event) => void submit(event)}>
        <a className="brand stacked" href="/">
          <span>{t.siteName}</span>
          <small>{t.admin}</small>
        </a>
        <label>
          <span>{t.email as string}</span>
          <input autoFocus type="email" value={email} onChange={(event) => setEmail(event.target.value)} placeholder="owner@local.dev" />
        </label>
        <label>
          <span>{t.password as string}</span>
          <input type="password" value={password} onChange={(event) => setPassword(event.target.value)} />
        </label>
        {error && <span className="form-error">{error}</span>}
        <button className="primary" type="submit">{t.login}</button>
      </form>
    </div>
  );
}

function UsersView() {
  const { lang, t } = useLang();
  const [users, setUsers] = useState<UserListItem[]>([]);
  const [total, setTotal] = useState(0);
  const [currentId, setCurrentId] = useState<number | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [creating, setCreating] = useState(false);
  const [editing, setEditing] = useState<UserListItem | null>(null);
  const [resetting, setResetting] = useState<UserListItem | null>(null);
  const userGrid = { gridTemplateColumns: "minmax(220px, 1fr) 170px 110px 190px 190px" };

  const loadUsers = async () => {
    setLoading(true);
    setError(null);
    try {
      const [list, me] = await Promise.all([adminUsers.list(), api.me()]);
      setUsers(list.items);
      setTotal(list.total);
      setCurrentId(me.user.id);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Load users failed");
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    void loadUsers();
  }, []);

  const remove = async (user: UserListItem) => {
    if (user.id === currentId) return;
    const label = user.display_name || user.email;
    if (!window.confirm(lang === "zh" ? `确认删除 ${label}？` : `Delete ${label}?`)) return;
    try {
      await adminUsers.remove(user.id);
      await loadUsers();
    } catch (err) {
      setError(err instanceof Error ? err.message : "Delete user failed");
    }
  };

  const closeDialogs = () => {
    setCreating(false);
    setEditing(null);
    setResetting(null);
  };

  return (
    <>
      <section className="stats-grid">
        <Stat label={t.total as string} value={total} />
        <Stat label={t.owner as string} value={users.filter((user) => user.role === "owner").length} />
        <Stat label={t.editor as string} value={users.filter((user) => user.role === "editor").length} />
        <Stat label={t.viewer as string} value={users.filter((user) => user.role === "viewer").length} />
      </section>

      <section className="toolbar">
        <div className="filters">
          <FilterButton active onClick={() => undefined} label={t.users as string} count={total} />
        </div>
        <div className="toolbar-actions">
          <button className="secondary" onClick={() => void loadUsers()}>{t.reset}</button>
          <button className="primary" onClick={() => setCreating(true)}>+ {t.newUser as string}</button>
        </div>
      </section>

      {loading && <div className="state-line">Loading...</div>}
      {error && <div className="state-line error">{error}</div>}

      <section className="admin-table">
        <div className="table-head" style={userGrid}>
          <span>{t.email as string}</span>
          <span>{t.displayName as string}</span>
          <span>{t.role as string}</span>
          <span>{t.createdAt as string}</span>
          <span>{t.actions as string}</span>
        </div>
        {users.length === 0 ? (
          <div className="empty-state">{t.users as string}</div>
        ) : users.map((user) => (
          <article className="table-row" style={userGrid} key={user.id}>
            <div className="row-title">
              <strong>{user.email}</strong>
              {user.id === currentId && <span>{lang === "zh" ? "当前用户" : "Current user"}</span>}
            </div>
            <span>{user.display_name || "-"}</span>
            <span>{roleLabel(user.role, lang)}</span>
            <span>{formatDate(user.created_at)}</span>
            <div className="row-actions">
              <button onClick={() => setEditing(user)}>{t.edit}</button>
              <button onClick={() => setResetting(user)}>{t.resetPassword as string}</button>
              <button disabled={user.id === currentId} onClick={() => void remove(user)}>{t.delete}</button>
            </div>
          </article>
        ))}
      </section>

      {(creating || editing) && (
        <UserDialog
          initial={editing ?? undefined}
          currentId={currentId}
          onClose={closeDialogs}
          onSaved={async () => {
            closeDialogs();
            await loadUsers();
          }}
        />
      )}

      {resetting && (
        <PasswordDialog
          user={resetting}
          onClose={closeDialogs}
          onSaved={async () => {
            closeDialogs();
            await loadUsers();
          }}
        />
      )}
    </>
  );
}

function TagsView() {
  const { lang, t } = useLang();
  const [tags, setTags] = useState<Tag[]>([]);
  const [total, setTotal] = useState(0);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [creating, setCreating] = useState(false);
  const [editing, setEditing] = useState<Tag | null>(null);
  const tagGrid = { gridTemplateColumns: "90px minmax(180px, 1fr) minmax(220px, 1.5fr) 170px" };

  const loadTags = async () => {
    setLoading(true);
    setError(null);
    try {
      const list = await adminTags.list(1, 200);
      setTags(list.items);
      setTotal(list.total);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Load tags failed");
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    void loadTags();
  }, []);

  const remove = async (tag: Tag) => {
    if (!window.confirm(lang === "zh" ? `确认删除 ${tag.slug}？` : `Delete ${tag.slug}?`)) return;
    try {
      await adminTags.remove(tag.id);
      await loadTags();
    } catch (err) {
      setError(err instanceof Error ? err.message : "Delete tag failed");
    }
  };

  const closeDialog = () => {
    setCreating(false);
    setEditing(null);
  };

  return (
    <>
      <section className="stats-grid">
        <Stat label={t.total as string} value={total} />
        <Stat label={t.tags as string} value={tags.length} />
      </section>

      <section className="toolbar">
        <div className="filters">
          <FilterButton active onClick={() => undefined} label={t.tags as string} count={total} />
        </div>
        <div className="toolbar-actions">
          <button className="secondary" onClick={() => void loadTags()}>{t.reset}</button>
          <button className="primary" onClick={() => setCreating(true)}>+ {t.addTag as string}</button>
        </div>
      </section>

      {loading && <div className="state-line">Loading...</div>}
      {error && <div className="state-line error">{error}</div>}

      <section className="admin-table">
        <div className="table-head" style={tagGrid}>
          <span>ID</span>
          <span>{t.tagSlug as string}</span>
          <span>{t.tags as string}</span>
          <span>{t.actions as string}</span>
        </div>
        {tags.length === 0 ? (
          <div className="empty-state">{t.noTags as string}</div>
        ) : tags.map((tag) => (
          <article className="table-row" style={tagGrid} key={tag.id}>
            <span>{tag.id}</span>
            <strong>{tag.slug}</strong>
            <span>{tag.name[lang]}</span>
            <div className="row-actions">
              <button onClick={() => setEditing(tag)}>{t.edit}</button>
              <button onClick={() => void remove(tag)}>{t.delete}</button>
            </div>
          </article>
        ))}
      </section>

      {(creating || editing) && (
        <TagDialog
          initial={editing ?? undefined}
          onClose={closeDialog}
          onSaved={async () => {
            closeDialog();
            await loadTags();
          }}
        />
      )}
    </>
  );
}

function CategoriesView() {
  const { lang, t } = useLang();
  const [items, setItems] = useState<CategoryAdminDto[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [creating, setCreating] = useState(false);
  const [editing, setEditing] = useState<CategoryAdminDto | null>(null);
  const grid = { gridTemplateColumns: "90px minmax(160px, 1fr) minmax(180px, 1fr) 110px 110px 170px" };

  const loadCategories = async () => {
    setLoading(true);
    setError(null);
    try {
      const list = await adminCategories.list();
      setItems(list);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Load categories failed");
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    void loadCategories();
  }, []);

  const remove = async (category: CategoryAdminDto) => {
    if (!window.confirm(lang === "zh" ? `确认删除 ${category.slug}？` : `Delete ${category.slug}?`)) return;
    try {
      await adminCategories.remove(category.id);
      await loadCategories();
    } catch (err) {
      setError(err instanceof Error ? err.message : "Delete category failed");
    }
  };

  const closeDialog = () => {
    setCreating(false);
    setEditing(null);
  };

  return (
    <>
      <section className="stats-grid">
        <Stat label={t.total as string} value={items.length} />
        <Stat label={t.photoCount as string} value={items.reduce((sum, item) => sum + item.photo_count, 0)} />
      </section>

      <section className="toolbar">
        <div className="filters">
          <FilterButton active onClick={() => undefined} label={t.categoriesAdmin as string} count={items.length} />
        </div>
        <div className="toolbar-actions">
          <button className="secondary" onClick={() => void loadCategories()}>{t.reset}</button>
          <button className="primary" onClick={() => setCreating(true)}>+ {t.addCategory as string}</button>
        </div>
      </section>

      {loading && <div className="state-line">Loading...</div>}
      {error && <div className="state-line error">{error}</div>}

      <section className="admin-table">
        <div className="table-head" style={grid}>
          <span>ID</span>
          <span>{t.categorySlug as string}</span>
          <span>{t.categoriesAdmin as string}</span>
          <span>{t.sortOrder as string}</span>
          <span>{t.photoCount as string}</span>
          <span>{t.actions as string}</span>
        </div>
        {items.length === 0 ? (
          <div className="empty-state">{t.noCategories as string}</div>
        ) : items.map((category) => (
          <article className="table-row" style={grid} key={category.id}>
            <span>{category.id}</span>
            <strong>{category.slug}</strong>
            <span>{category.name[lang]}</span>
            <span>{category.sort_order}</span>
            <span>{category.photo_count}</span>
            <div className="row-actions">
              <button onClick={() => setEditing(category)}>{t.edit}</button>
              <button onClick={() => void remove(category)}>{t.delete}</button>
            </div>
          </article>
        ))}
      </section>

      {(creating || editing) && (
        <CategoryDialog
          initial={editing ?? undefined}
          onClose={closeDialog}
          onSaved={async () => {
            closeDialog();
            await loadCategories();
          }}
        />
      )}
    </>
  );
}

function CategoryDialog({
  initial,
  onClose,
  onSaved
}: {
  initial?: CategoryAdminDto;
  onClose: () => void;
  onSaved: () => Promise<void>;
}) {
  const { t } = useLang();
  const [slug, setSlug] = useState(initial?.slug ?? "");
  const [nameZh, setNameZh] = useState(initial?.name.zh ?? "");
  const [nameEn, setNameEn] = useState(initial?.name.en ?? "");
  const [sortOrder, setSortOrder] = useState(String(initial?.sort_order ?? 0));
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const submit = async (event: React.FormEvent) => {
    event.preventDefault();
    setSubmitting(true);
    setError(null);
    const parsedSort = Number.parseInt(sortOrder, 10);
    try {
      if (initial) {
        const req: UpdateCategoryReq = {
          slug,
          name: { zh: nameZh, en: nameEn || nameZh },
          sort_order: Number.isNaN(parsedSort) ? 0 : parsedSort
        };
        await adminCategories.update(initial.id, req);
      } else {
        const req: NewCategoryReq = {
          slug,
          name: { zh: nameZh, en: nameEn || nameZh },
          sort_order: Number.isNaN(parsedSort) ? 0 : parsedSort
        };
        await adminCategories.create(req);
      }
      await onSaved();
    } catch (err) {
      setError(err instanceof Error ? err.message : "Save category failed");
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <div className="modal-backdrop" onMouseDown={onClose}>
      <form className="photo-dialog" onSubmit={(event) => void submit(event)} onMouseDown={(event) => event.stopPropagation()}>
        <header>
          <h2>{initial ? t.editCategory : t.addCategory}</h2>
          <button type="button" className="icon-only" onClick={onClose} aria-label={t.close as string}><CloseIcon /></button>
        </header>

        <label className="span-2">
          <span>{t.categorySlug as string}</span>
          <input value={slug} onChange={(event) => setSlug(event.target.value)} placeholder="portrait" required />
        </label>
        {initial && <p className="upload-hint span-2">{t.slugChangeWarning as string}</p>}
        <label>
          <span>{t.categoryNameZh as string}</span>
          <input value={nameZh} onChange={(event) => setNameZh(event.target.value)} required />
        </label>
        <label>
          <span>{t.categoryNameEn as string}</span>
          <input value={nameEn} onChange={(event) => setNameEn(event.target.value)} />
        </label>
        <label className="span-2">
          <span>{t.sortOrder as string}</span>
          <input type="number" value={sortOrder} onChange={(event) => setSortOrder(event.target.value)} />
        </label>

        {error && <span className="form-error span-2">{error}</span>}

        <footer className="dialog-actions span-2">
          <button type="button" className="secondary" onClick={onClose}>{t.cancel}</button>
          <button className="primary" disabled={submitting}>{submitting ? "..." : t.save}</button>
        </footer>
      </form>
    </div>
  );
}

function TagDialog({
  initial,
  onClose,
  onSaved
}: {
  initial?: Tag;
  onClose: () => void;
  onSaved: () => Promise<void>;
}) {
  const { t } = useLang();
  const [slug, setSlug] = useState(initial?.slug ?? "");
  const [nameZh, setNameZh] = useState(initial?.name.zh ?? "");
  const [nameEn, setNameEn] = useState(initial?.name.en ?? "");
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const submit = async (event: React.FormEvent) => {
    event.preventDefault();
    setSubmitting(true);
    setError(null);
    try {
      if (initial) {
        const req: UpdateTagReq = {
          slug,
          name: { zh: nameZh, en: nameEn || nameZh }
        };
        await adminTags.update(initial.id, req);
      } else {
        const req: NewTagReq = {
          slug,
          name: { zh: nameZh, en: nameEn || nameZh }
        };
        await adminTags.create(req);
      }
      await onSaved();
    } catch (err) {
      setError(err instanceof Error ? err.message : "Save tag failed");
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <div className="modal-backdrop" onMouseDown={onClose}>
      <form className="photo-dialog" onSubmit={(event) => void submit(event)} onMouseDown={(event) => event.stopPropagation()}>
        <header>
          <h2>{initial ? t.editTag : t.addTag}</h2>
          <button type="button" className="icon-only" onClick={onClose} aria-label={t.close as string}><CloseIcon /></button>
        </header>

        <label className="span-2">
          <span>{t.tagSlug as string}</span>
          <input value={slug} onChange={(event) => setSlug(event.target.value)} placeholder="morning" required />
        </label>
        <label>
          <span>{t.tagNameZh as string}</span>
          <input value={nameZh} onChange={(event) => setNameZh(event.target.value)} required />
        </label>
        <label>
          <span>{t.tagNameEn as string}</span>
          <input value={nameEn} onChange={(event) => setNameEn(event.target.value)} />
        </label>

        {error && <span className="form-error span-2">{error}</span>}

        <footer className="dialog-actions span-2">
          <button type="button" className="secondary" onClick={onClose}>{t.cancel}</button>
          <button className="primary" disabled={submitting}>{submitting ? "..." : t.save}</button>
        </footer>
      </form>
    </div>
  );
}

function UserDialog({
  initial,
  currentId,
  onClose,
  onSaved
}: {
  initial?: UserListItem;
  currentId: number | null;
  onClose: () => void;
  onSaved: () => Promise<void>;
}) {
  const { lang, t } = useLang();
  const editing = Boolean(initial);
  const self = initial?.id === currentId;
  const [email, setEmail] = useState(initial?.email ?? "");
  const [password, setPassword] = useState("");
  const [displayName, setDisplayName] = useState(initial?.display_name ?? "");
  const [role, setRole] = useState<UserRole>(initial?.role ?? "editor");
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const submit = async (event: React.FormEvent) => {
    event.preventDefault();
    setSubmitting(true);
    setError(null);
    try {
      if (initial) {
        const req: UpdateUserReq = { display_name: displayName };
        if (!self) req.role = role;
        await adminUsers.update(initial.id, req);
      } else {
        const req: NewUserReq = {
          email,
          password,
          display_name: displayName || null,
          role
        };
        await adminUsers.create(req);
      }
      await onSaved();
    } catch (err) {
      setError(err instanceof Error ? err.message : "Save user failed");
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <div className="modal-backdrop" onMouseDown={onClose}>
      <form className="photo-dialog" onSubmit={(event) => void submit(event)} onMouseDown={(event) => event.stopPropagation()}>
        <header>
          <h2>{editing ? t.editUser : t.newUser}</h2>
          <button type="button" className="icon-only" onClick={onClose} aria-label={t.close as string}><CloseIcon /></button>
        </header>

        <label className="span-2">
          <span>{t.email as string}</span>
          <input type="email" value={email} onChange={(event) => setEmail(event.target.value)} disabled={editing} required />
        </label>

        {!editing && (
          <label className="span-2">
            <span>{t.password as string}</span>
            <input type="password" value={password} onChange={(event) => setPassword(event.target.value)} minLength={8} required />
          </label>
        )}

        <label>
          <span>{t.displayName as string}</span>
          <input value={displayName} onChange={(event) => setDisplayName(event.target.value)} />
        </label>
        <label>
          <span>{t.role as string}</span>
          <select value={role} onChange={(event) => setRole(event.target.value as UserRole)} disabled={self}>
            {(["owner", "editor", "viewer"] as UserRole[]).map((item) => (
              <option key={item} value={item}>{roleLabel(item, lang)}</option>
            ))}
          </select>
        </label>

        {error && <span className="form-error span-2">{error}</span>}

        <footer className="dialog-actions span-2">
          <button type="button" className="secondary" onClick={onClose}>{t.cancel}</button>
          <button className="primary" disabled={submitting}>{submitting ? "..." : t.save}</button>
        </footer>
      </form>
    </div>
  );
}

function PasswordDialog({
  user,
  onClose,
  onSaved
}: {
  user: UserListItem;
  onClose: () => void;
  onSaved: () => Promise<void>;
}) {
  const { t } = useLang();
  const [password, setPassword] = useState("");
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const submit = async (event: React.FormEvent) => {
    event.preventDefault();
    setSubmitting(true);
    setError(null);
    try {
      await adminUsers.resetPassword(user.id, password);
      await onSaved();
    } catch (err) {
      setError(err instanceof Error ? err.message : "Reset password failed");
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <div className="modal-backdrop" onMouseDown={onClose}>
      <form className="photo-dialog" onSubmit={(event) => void submit(event)} onMouseDown={(event) => event.stopPropagation()}>
        <header>
          <h2>{t.resetPassword as string}</h2>
          <button type="button" className="icon-only" onClick={onClose} aria-label={t.close as string}><CloseIcon /></button>
        </header>

        <label className="span-2">
          <span>{user.email}</span>
          <input type="password" value={password} onChange={(event) => setPassword(event.target.value)} minLength={8} placeholder={t.newPassword as string} required />
        </label>

        {error && <span className="form-error span-2">{error}</span>}

        <footer className="dialog-actions span-2">
          <button type="button" className="secondary" onClick={onClose}>{t.cancel}</button>
          <button className="primary" disabled={submitting}>{submitting ? "..." : t.save}</button>
        </footer>
      </form>
    </div>
  );
}

function PhotoDialog({
  initial,
  onClose,
  onSave,
  onNotice
}: {
  initial?: Photo;
  onClose: () => void;
  onSave: (payload: PhotoPayload) => Promise<void>;
  onNotice: (message: string | null) => void;
}) {
  const { lang, t } = useLang();
  const [form, setForm] = useState<FormState>(() => toForm(initial));
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [tags, setTags] = useState<Tag[]>([]);
  const [uploadState, setUploadState] = useState<"idle" | "uploading" | "failed">("idle");
  const fileInputRef = useRef<HTMLInputElement>(null);
  const previewSrc = form.src.startsWith("http://") || form.src.startsWith("https://") ? form.src : null;

  useEffect(() => {
    let alive = true;
    adminTags
      .list(1, 200)
      .then((list) => {
        if (alive) setTags(list.items);
      })
      .catch((err) => {
        if (alive) setError(err instanceof Error ? err.message : "Load tags failed");
      });
    return () => {
      alive = false;
    };
  }, []);

  const update = <K extends keyof FormState>(key: K, value: FormState[K]) => {
    setForm((prev) => ({ ...prev, [key]: value }));
  };

  const toggleTag = (id: number) => {
    setForm((prev) => {
      const selected = prev.tagIds.includes(id);
      return {
        ...prev,
        tagIds: selected ? prev.tagIds.filter((item) => item !== id) : [...prev.tagIds, id]
      };
    });
  };

  const submit = async (event: React.FormEvent) => {
    event.preventDefault();
    setSubmitting(true);
    setError(null);
    try {
      await onSave(toPayload(form));
      onNotice(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Save failed");
    } finally {
      setSubmitting(false);
    }
  };

  const onFileSelected = async (event: React.ChangeEvent<HTMLInputElement>) => {
    const file = event.target.files?.[0];
    if (!file) return;
    setUploadState("uploading");
    try {
      const presign = await api.presign({
        file_name: file.name,
        mime_type: file.type,
        byte_size: file.size
      });
      const put = await fetch(presign.upload_url, {
        method: "PUT",
        headers: { "content-type": file.type },
        body: file
      });
      if (!put.ok) throw new Error(`upload failed: HTTP ${put.status}`);
      const complete = await api.complete({ storage_key: presign.storage_key });
      update("src", complete.storage_key);
      setUploadState("idle");
      onNotice(t.uploadDone as string);
    } catch (err) {
      setUploadState("failed");
      onNotice(`${t.uploadFailed}: ${err}`);
    } finally {
      if (fileInputRef.current) fileInputRef.current.value = "";
    }
  };

  return (
    <div className="modal-backdrop" onMouseDown={onClose}>
      <form className="photo-dialog" onSubmit={(event) => void submit(event)} onMouseDown={(event) => event.stopPropagation()}>
        <header>
          <h2>{initial ? t.edit : t.newPhoto}</h2>
          <button type="button" className="icon-only" onClick={onClose} aria-label={t.close as string}><CloseIcon /></button>
        </header>

        {previewSrc && <img className="form-preview" src={previewSrc} alt="" decoding="async" />}

        <label className="span-2">
          <span>{t.imageUrl as string}</span>
          <div className="inline-input">
            <input value={form.src} onChange={(event) => update("src", event.target.value)} placeholder="https://... 或上传后自动填" />
          </div>
        </label>
        <label className="span-2">
          <span>{t.chooseFile as string}</span>
          <input
            ref={fileInputRef}
            type="file"
            accept="image/*"
            onChange={(event) => void onFileSelected(event)}
            disabled={uploadState === "uploading"}
          />
        </label>
        {uploadState === "uploading" && <p className="upload-hint span-2">{t.uploading as string}</p>}
        {uploadState === "failed" && <p className="upload-hint span-2">{t.uploadFailed as string}</p>}

        <label className="span-2">
          <span>{t.slug as string}</span>
          <input value={form.slug} onChange={(event) => update("slug", event.target.value)} placeholder="tokyo-crosswalk" />
        </label>
        <label>
          <span>{t.title as string} ZH</span>
          <input value={form.titleZh} onChange={(event) => update("titleZh", event.target.value)} required />
        </label>
        <label>
          <span>{t.title as string} EN</span>
          <input value={form.titleEn} onChange={(event) => update("titleEn", event.target.value)} />
        </label>
        <label>
          <span>{t.location as string} ZH</span>
          <input value={form.locZh} onChange={(event) => update("locZh", event.target.value)} />
        </label>
        <label>
          <span>{t.location as string} EN</span>
          <input value={form.locEn} onChange={(event) => update("locEn", event.target.value)} />
        </label>
        <label>
          <span>{t.caption as string} ZH</span>
          <input value={form.captionZh} onChange={(event) => update("captionZh", event.target.value)} />
        </label>
        <label>
          <span>{t.caption as string} EN</span>
          <input value={form.captionEn} onChange={(event) => update("captionEn", event.target.value)} />
        </label>
        <label>
          <span>{t.altText as string} ZH</span>
          <input value={form.altTextZh} onChange={(event) => update("altTextZh", event.target.value)} />
        </label>
        <label>
          <span>{t.altText as string} EN</span>
          <input value={form.altTextEn} onChange={(event) => update("altTextEn", event.target.value)} />
        </label>
        <label>
          <span>{t.date as string}</span>
          <input value={form.date} onChange={(event) => update("date", event.target.value)} placeholder="2026.05" />
        </label>
        <label>
          <span>{t.category as string}</span>
          <select value={form.cat} onChange={(event) => update("cat", event.target.value)}>
            {categories.filter((item) => item.key !== "all").map((item) => (
              <option key={item.key} value={item.key}>{item[lang]}</option>
            ))}
          </select>
        </label>

        <div className="privacy-options span-2">
          {(["public", "locked", "private"] as Privacy[]).map((privacy) => (
            <button
              type="button"
              key={privacy}
              className={form.privacy === privacy ? "active" : ""}
              onClick={() => update("privacy", privacy)}
            >
              {privacyLabel(privacy, lang)}
            </button>
          ))}
        </div>

        {form.privacy === "locked" && (
          <label className="span-2">
            <span>{t.passcodeField as string}</span>
            <input
              value={form.passcode}
              onChange={(event) => update("passcode", event.target.value)}
              placeholder={t.passcodePlaceholder as string}
              autoComplete="off"
            />
            <p className="upload-hint" style={{ margin: "4px 0 0" }}>{t.passcodeHint as string}</p>
          </label>
        )}

        <fieldset className="span-2" style={{ border: "1px solid rgba(0, 0, 0, 0.12)", borderRadius: 8, padding: 12 }}>
          <legend>{t.tags as string}</legend>
          {tags.length === 0 ? (
            <span className="upload-hint">{t.noTags as string}</span>
          ) : tags.map((tag) => (
            <label key={tag.id} style={{ alignItems: "center", display: "flex", flexDirection: "row", gap: 8, margin: "6px 0" }}>
              <input
                type="checkbox"
                checked={form.tagIds.includes(tag.id)}
                onChange={() => toggleTag(tag.id)}
                style={{ width: "auto" }}
              />
              <span>{tag.name[lang]} · {tag.slug}</span>
            </label>
          ))}
        </fieldset>

        {error && <span className="form-error span-2">{error}</span>}

        <footer className="dialog-actions span-2">
          <button type="button" className="secondary" onClick={onClose}>{t.cancel}</button>
          <button className="primary" disabled={submitting}>{submitting ? "..." : t.save}</button>
        </footer>
      </form>
    </div>
  );
}

function Stat({ label, value }: { label: string; value: number }) {
  return (
    <div className="stat-card">
      <span>{label}</span>
      <strong>{value}</strong>
    </div>
  );
}

function SidebarItem({ icon, label, active, onClick }: { icon: React.ReactNode; label: string; active: boolean; onClick: () => void }) {
  return (
    <button type="button" className={`sidebar-item${active ? " active" : ""}`} onClick={onClick}>
      <span className="sidebar-item-icon">{icon}</span>
      <span className="sidebar-item-label">{label}</span>
    </button>
  );
}

function FilterButton({ active, onClick, label, count }: { active: boolean; onClick: () => void; label: string; count: number }) {
  return (
    <button className={active ? "active" : ""} onClick={onClick}>
      {label}
      <span>{count}</span>
    </button>
  );
}

function roleLabel(role: UserRole, lang: "zh" | "en") {
  const labels: Record<UserRole, { zh: string; en: string }> = {
    owner: { zh: "Owner", en: "Owner" },
    editor: { zh: "Editor", en: "Editor" },
    viewer: { zh: "Viewer", en: "Viewer" }
  };
  return labels[role][lang];
}

function formatDate(value: string) {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  return date.toLocaleString();
}

function privacyLabel(privacy: Privacy, lang: "zh" | "en") {
  const labels = {
    public: { zh: "公开", en: "Public" },
    locked: { zh: "加锁", en: "Locked" },
    private: { zh: "私密", en: "Private" }
  };
  return labels[privacy][lang];
}
