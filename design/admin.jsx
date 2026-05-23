// 后台管理
const { useState: us, useEffect: ue, useMemo: um } = React;

function Login({ onSuccess, lang, setLang, i18n }) {
  const [pw, setPw] = us("");
  const [err, setErr] = us(false);
  const submit = async (e) => {
    e.preventDefault();
    if (await Store.signIn(pw)) onSuccess();
    else { setErr(true); setTimeout(() => setErr(false), 1400); }
  };
  return (
    <div className="min-h-screen flex items-center justify-center px-6 relative" style={{ background: "var(--bg)" }}>
      <div className="absolute top-6 right-6 flex items-center text-[11px] border border-[var(--rule)] rounded-full overflow-hidden">
        <button onClick={() => setLang("zh")} className="px-2.5 py-1" style={{ background: lang === "zh" ? "var(--fg)" : "transparent", color: lang === "zh" ? "var(--bg)" : "var(--muted)" }}>中</button>
        <button onClick={() => setLang("en")} className="px-2.5 py-1" style={{ background: lang === "en" ? "var(--fg)" : "transparent", color: lang === "en" ? "var(--bg)" : "var(--muted)" }}>EN</button>
      </div>
      <form onSubmit={submit} className="w-full max-w-sm">
        <a href="index.html" className="block">
          <div className="font-serif text-3xl text-[var(--fg)] mb-1">{i18n.siteName}</div>
          <div className="text-[10px] tracking-[0.32em] uppercase text-[var(--muted)] mb-12">{i18n.admin}</div>
        </a>
        <label className="block text-[11px] tracking-[0.2em] uppercase text-[var(--muted)] mb-2">{i18n.passcode}</label>
        <input type="password" value={pw} onChange={(e) => setPw(e.target.value)} autoFocus
          className={"w-full bg-transparent border-b py-3 text-[var(--fg)] outline-none transition-colors tracking-widest " + (err ? "border-red-400" : "border-[var(--rule)] focus:border-[var(--fg)]")} />
        <button type="submit" className="mt-6 w-full py-3 text-[12px] tracking-[0.24em] uppercase bg-[var(--fg)] text-[var(--bg)] hover:opacity-90 transition-opacity">{i18n.login}</button>
        <p className="text-[10px] tracking-[0.2em] uppercase text-[var(--muted)] mt-4 text-center">hint · admin</p>
      </form>
    </div>
  );
}

function PhotoForm({ initial, onSave, onCancel, lang }) {
  const [f, setF] = us(initial ? {
    src: initial.src, titleZh: initial.title.zh, titleEn: initial.title.en,
    locZh: initial.loc.zh, locEn: initial.loc.en,
    cat: initial.cat, date: initial.date, privacy: initial.privacy,
  } : { src: "", titleZh: "", titleEn: "", locZh: "", locEn: "", cat: "street", date: "2025.01", privacy: "public" });
  const upd = (k, v) => setF((p) => ({ ...p, [k]: v }));
  const submit = (e) => {
    e.preventDefault();
    if (!f.src || !f.titleZh) return;
    onSave({
      id: initial?.id ?? Date.now(),
      src: f.src, cat: f.cat,
      title: { zh: f.titleZh, en: f.titleEn || f.titleZh },
      loc: { zh: f.locZh, en: f.locEn || f.locZh },
      date: f.date, privacy: f.privacy,
    });
  };
  const inputCls = "w-full bg-transparent border border-[var(--rule)] px-3 py-2 text-[14px] text-[var(--fg)] outline-none focus:border-[var(--fg)] transition-colors";
  const Lbl = ({ children }) => <span className="block text-[10px] tracking-[0.22em] uppercase text-[var(--muted)] mb-1.5">{children}</span>;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/40 backdrop-blur-md" onClick={onCancel}>
      <form onSubmit={submit} className="bg-[var(--bg)] border border-[var(--rule)] w-full max-w-2xl max-h-[90vh] overflow-y-auto" onClick={(e) => e.stopPropagation()}>
        <div className="px-7 py-5 border-b border-[var(--rule)] flex items-center justify-between">
          <div className="font-serif text-xl">{initial ? (lang === "zh" ? "编辑作品" : "Edit") : (lang === "zh" ? "新建作品" : "New frame")}</div>
          <button type="button" onClick={onCancel} className="text-[var(--muted)] hover:text-[var(--fg)]"><CloseIcon size={18} /></button>
        </div>
        <div className="px-7 py-6 space-y-5">
          {f.src && <div className="aspect-[16/9] bg-[var(--card)] overflow-hidden"><img src={f.src} alt="preview" className="w-full h-full object-cover" /></div>}
          <label className="block"><Lbl>{lang === "zh" ? "图片地址" : "Image URL"}</Lbl><input value={f.src} onChange={(e) => upd("src", e.target.value)} className={inputCls} placeholder="https://images.unsplash.com/..." /></label>
          <div className="grid grid-cols-2 gap-4">
            <label className="block"><Lbl>{lang === "zh" ? "标题（中）" : "Title (ZH)"}</Lbl><input value={f.titleZh} onChange={(e) => upd("titleZh", e.target.value)} className={inputCls} /></label>
            <label className="block"><Lbl>{lang === "zh" ? "标题（英）" : "Title (EN)"}</Lbl><input value={f.titleEn} onChange={(e) => upd("titleEn", e.target.value)} className={inputCls} /></label>
            <label className="block"><Lbl>{lang === "zh" ? "地点（中）" : "Location (ZH)"}</Lbl><input value={f.locZh} onChange={(e) => upd("locZh", e.target.value)} className={inputCls} /></label>
            <label className="block"><Lbl>{lang === "zh" ? "地点（英）" : "Location (EN)"}</Lbl><input value={f.locEn} onChange={(e) => upd("locEn", e.target.value)} className={inputCls} /></label>
            <label className="block"><Lbl>{lang === "zh" ? "日期" : "Date"}</Lbl><input value={f.date} onChange={(e) => upd("date", e.target.value)} className={inputCls} placeholder="2024.05" /></label>
            <label className="block"><Lbl>{lang === "zh" ? "分类" : "Category"}</Lbl>
              <select value={f.cat} onChange={(e) => upd("cat", e.target.value)} className={inputCls}>
                {CATEGORIES.filter(c => c.key !== "all").map(c => <option key={c.key} value={c.key}>{c[lang]}</option>)}
              </select>
            </label>
          </div>
          <div><Lbl>{lang === "zh" ? "可见性" : "Privacy"}</Lbl>
            <div className="grid grid-cols-3 gap-2">
              {[
                { k: "public", zh: "公开", en: "Public", desc: { zh: "所有访客可见", en: "Everyone" } },
                { k: "locked", zh: "加锁", en: "Locked", desc: { zh: "需要口令", en: "Passcode" } },
                { k: "private", zh: "私密", en: "Private", desc: { zh: "仅本人可见", en: "Owner only" } },
              ].map(opt => (
                <button key={opt.k} type="button" onClick={() => upd("privacy", opt.k)} className="text-left px-3 py-3 border transition-colors"
                  style={{ borderColor: f.privacy === opt.k ? "var(--fg)" : "var(--rule)", background: f.privacy === opt.k ? "var(--card)" : "transparent" }}>
                  <div className="text-[13px]">{opt[lang]}</div>
                  <div className="text-[10px] text-[var(--muted)] mt-0.5">{opt.desc[lang]}</div>
                </button>
              ))}
            </div>
          </div>
        </div>
        <div className="px-7 py-5 border-t border-[var(--rule)] flex items-center justify-end gap-3">
          <button type="button" onClick={onCancel} className="px-5 py-2 text-[12px] tracking-[0.18em] uppercase text-[var(--muted)] hover:text-[var(--fg)]">{lang === "zh" ? "取消" : "Cancel"}</button>
          <button type="submit" className="px-6 py-2 text-[12px] tracking-[0.18em] uppercase bg-[var(--fg)] text-[var(--bg)] hover:opacity-90">{lang === "zh" ? "保存" : "Save"}</button>
        </div>
      </form>
    </div>
  );
}

function AdminApp() {
  const [authed, setAuthed] = us(() => Store.isAuthed());
  const [lang, setLang, i18n] = useLang();
  const photos = usePhotos({ admin: true, enabled: authed });
  const [filter, setFilter] = us("all");
  const [editing, setEditing] = us(null);
  const [creating, setCreating] = us(false);

  ue(() => {
    const r = document.documentElement;
    r.style.setProperty("--bg", "#FAF8F4");
    r.style.setProperty("--card", "#F4F1EA");
    r.style.setProperty("--fg", "#1A1814");
    r.style.setProperty("--muted", "#9b958a");
    r.style.setProperty("--muted-2", "#5d574d");
    r.style.setProperty("--rule", "#e8e3d8");
  }, []);

  if (!authed) return <Login onSuccess={() => setAuthed(true)} lang={lang} setLang={setLang} i18n={i18n} />;

  const filtered = um(() => {
    if (filter === "all") return photos;
    if (["public","locked","private"].includes(filter)) return photos.filter(p => p.privacy === filter);
    return photos.filter(p => p.cat === filter);
  }, [filter, photos]);
  const stats = um(() => ({
    total: photos.length,
    public: photos.filter(p => p.privacy === "public").length,
    locked: photos.filter(p => p.privacy === "locked").length,
    private: photos.filter(p => p.privacy === "private").length,
  }), [photos]);

  const save = async (photo) => {
    try {
      if (Store.load(true).some(p => p.id === photo.id)) await Store.updatePhoto(photo);
      else await Store.createPhoto(photo);
      setEditing(null); setCreating(false);
    } catch (err) {
      alert(err.message || (lang === "zh" ? "保存失败" : "Save failed"));
    }
  };
  const remove = async (id) => {
    if (!confirm(lang === "zh" ? "确认删除这一帧？" : "Delete this frame?")) return;
    try { await Store.deletePhoto(id); }
    catch (err) { alert(err.message || (lang === "zh" ? "删除失败" : "Delete failed")); }
  };
  const togglePrivacy = async (id) => {
    const current = Store.load(true).find(p => p.id === id);
    if (!current) return;
    const order = ["public", "locked", "private"];
    try { await Store.updatePrivacy(id, order[(order.indexOf(current.privacy) + 1) % 3]); }
    catch (err) { alert(err.message || (lang === "zh" ? "更新失败" : "Update failed")); }
  };
  const resetAll = async () => {
    if (!confirm(lang === "zh" ? "重置为初始数据？所有更改将丢失。" : "Reset all data?")) return;
    try { await Store.resetRemote(); }
    catch (err) { alert(err.message || (lang === "zh" ? "重置失败" : "Reset failed")); }
  };
  const signOut = async () => { await Store.signOut(); setAuthed(false); };

  const Chip = ({ k, label, count }) => (
    <button onClick={() => setFilter(k)} className="px-3.5 py-1.5 text-[12px] tracking-[0.1em] border transition-colors whitespace-nowrap"
      style={{ borderColor: filter === k ? "var(--fg)" : "var(--rule)", background: filter === k ? "var(--fg)" : "transparent", color: filter === k ? "var(--bg)" : "var(--muted-2)" }}>
      {label}<span className="opacity-60 ml-1.5">{count}</span>
    </button>
  );

  const privacyDot = (p) => p === "public" ? "#5e7a64" : p === "locked" ? "#9a5a3c" : "#9a9a9f";
  const privacyLabel = (p) => p === "public" ? i18n.public : p === "locked" ? i18n.locked : i18n.private;

  return (
    <div className="min-h-screen" style={{ background: "var(--bg)", color: "var(--fg)" }}>
      <style>{`
        body { font-family: 'Noto Sans SC', 'Inter', ui-sans-serif, system-ui, -apple-system, sans-serif; -webkit-font-smoothing: antialiased; }
        .font-serif { font-family: 'Noto Serif SC', 'Cormorant Garamond', Georgia, serif; font-weight: 500; }
      `}</style>

      <header className="sticky top-0 z-30 backdrop-blur-md bg-[var(--bg)]/90 border-b border-[var(--rule)]">
        <div className="max-w-[1280px] mx-auto px-5 sm:px-8 h-16 flex items-center justify-between gap-3">
          <a href="index.html" className="flex items-baseline gap-3 shrink-0">
            <span className="font-serif text-[22px] text-[var(--fg)]">{i18n.siteName}</span>
            <span className="hidden sm:inline text-[10px] tracking-[0.28em] uppercase text-[var(--muted)]">{i18n.admin}</span>
          </a>
          <div className="flex items-center gap-3 sm:gap-4">
            <div className="flex items-center text-[11px] border border-[var(--rule)] rounded-full overflow-hidden">
              <button onClick={() => setLang("zh")} className="px-2.5 py-1" style={{ background: lang === "zh" ? "var(--fg)" : "transparent", color: lang === "zh" ? "var(--bg)" : "var(--muted)" }}>中</button>
              <button onClick={() => setLang("en")} className="px-2.5 py-1" style={{ background: lang === "en" ? "var(--fg)" : "transparent", color: lang === "en" ? "var(--bg)" : "var(--muted)" }}>EN</button>
            </div>
            <a href="index.html" className="text-[11px] tracking-[0.18em] uppercase text-[var(--muted)] hover:text-[var(--fg)]">{lang === "zh" ? "前台" : "Site"}</a>
            <button onClick={signOut} className="text-[11px] tracking-[0.18em] uppercase text-[var(--muted)] hover:text-[var(--fg)]">{i18n.logout}</button>
          </div>
        </div>
      </header>

      <main className="max-w-[1280px] mx-auto px-5 sm:px-8 py-8 sm:py-12">
        {/* 统计 */}
        <section className="grid grid-cols-2 sm:grid-cols-4 gap-px bg-[var(--rule)] border border-[var(--rule)] mb-10">
          {[
            { k: "total", label: lang === "zh" ? "总数" : "Total", v: stats.total },
            { k: "public", label: i18n.public, v: stats.public },
            { k: "locked", label: i18n.locked, v: stats.locked },
            { k: "private", label: i18n.private, v: stats.private },
          ].map(s => (
            <div key={s.k} className="bg-[var(--bg)] px-5 py-5">
              <div className="text-[10px] tracking-[0.24em] uppercase text-[var(--muted)]">{s.label}</div>
              <div className="font-serif text-3xl mt-1">{s.v}</div>
            </div>
          ))}
        </section>

        {/* 工具栏 */}
        <div className="flex flex-col lg:flex-row lg:items-center lg:justify-between gap-4 mb-6">
          <div className="flex items-center gap-2 flex-wrap">
            <Chip k="all" label={lang === "zh" ? "全部" : "All"} count={stats.total} />
            <Chip k="public" label={i18n.public} count={stats.public} />
            <Chip k="locked" label={i18n.locked} count={stats.locked} />
            <Chip k="private" label={i18n.private} count={stats.private} />
            <span className="w-px h-5 bg-[var(--rule)] mx-1" />
            {CATEGORIES.filter(c => c.key !== "all").map(c => (
              <Chip key={c.key} k={c.key} label={c[lang]} count={photos.filter(p => p.cat === c.key).length} />
            ))}
          </div>
          <div className="flex items-center gap-3 shrink-0">
            <button onClick={resetAll} className="text-[11px] tracking-[0.18em] uppercase text-[var(--muted)] hover:text-[var(--fg)]">{lang === "zh" ? "重置" : "Reset"}</button>
            <button onClick={() => setCreating(true)} className="px-5 py-2.5 text-[12px] tracking-[0.2em] uppercase bg-[var(--fg)] text-[var(--bg)] hover:opacity-90 transition-opacity">+ {lang === "zh" ? "新建" : "New"}</button>
          </div>
        </div>

        {/* 列表 */}
        <div className="border border-[var(--rule)]">
          <div className="hidden md:grid grid-cols-[80px_1fr_120px_140px_120px_120px] gap-4 px-5 py-3 border-b border-[var(--rule)] text-[10px] tracking-[0.22em] uppercase text-[var(--muted)] bg-[var(--card)]/40">
            <div>{lang === "zh" ? "图片" : "Image"}</div>
            <div>{lang === "zh" ? "标题" : "Title"}</div>
            <div>{lang === "zh" ? "分类" : "Category"}</div>
            <div>{lang === "zh" ? "日期" : "Date"}</div>
            <div>{lang === "zh" ? "可见性" : "Privacy"}</div>
            <div className="text-right">{lang === "zh" ? "操作" : "Actions"}</div>
          </div>
          {filtered.length === 0 ? (
            <div className="px-5 py-16 text-center text-[var(--muted)] text-[13px]">{lang === "zh" ? "暂无作品" : "No frames"}</div>
          ) : filtered.map(p => (
            <div key={p.id} className="grid grid-cols-[60px_1fr_auto] md:grid-cols-[80px_1fr_120px_140px_120px_120px] gap-4 px-5 py-3.5 border-b border-[var(--rule)] last:border-b-0 items-center hover:bg-[var(--card)]/30 transition-colors">
              <div className="w-[60px] md:w-[80px] aspect-[4/3] bg-[var(--card)] overflow-hidden">
                <img src={p.src} alt="" className="w-full h-full object-cover" loading="lazy" />
              </div>
              <div className="min-w-0">
                <div className="font-serif text-[16px] truncate">{p.title[lang]}</div>
                <div className="text-[11px] text-[var(--muted)] truncate">{p.loc[lang]}</div>
              </div>
              <div className="hidden md:block text-[12px] text-[var(--muted-2)]">{CATEGORIES.find(c => c.key === p.cat)?.[lang]}</div>
              <div className="hidden md:block text-[12px] text-[var(--muted-2)] tabular-nums">{p.date}</div>
              <div className="hidden md:block">
                <button onClick={() => togglePrivacy(p.id)} className="flex items-center gap-2 text-[11px] tracking-[0.14em] hover:text-[var(--fg)] text-[var(--muted-2)]">
                  <span className="w-1.5 h-1.5 rounded-full" style={{ background: privacyDot(p.privacy) }} />
                  {privacyLabel(p.privacy)}
                </button>
              </div>
              <div className="flex items-center justify-end gap-3 md:gap-2 col-span-1">
                <button onClick={() => setEditing(p)} className="text-[11px] tracking-[0.16em] uppercase text-[var(--muted)] hover:text-[var(--fg)] px-2 py-1">{lang === "zh" ? "编辑" : "Edit"}</button>
                <button onClick={() => remove(p.id)} className="text-[11px] tracking-[0.16em] uppercase text-[var(--muted)] hover:text-red-500 px-2 py-1">{lang === "zh" ? "删除" : "Delete"}</button>
              </div>
            </div>
          ))}
        </div>

        <p className="text-[10px] tracking-[0.2em] uppercase text-[var(--muted)] mt-6">{lang === "zh" ? "数据已通过 API 保存。" : "Data saved through API."}</p>
      </main>

      {(editing || creating) && (
        <PhotoForm initial={editing} onSave={save} onCancel={() => { setEditing(null); setCreating(false); }} lang={lang} />
      )}
    </div>
  );
}

ReactDOM.createRoot(document.getElementById("root")).render(<AdminApp />);
