# themes/ — 可插拔主题包

对标 WordPress 的 `wp-content/themes/<slug>/`。一个目录 = 一个主题。后端启动时扫描本目录装配 `ThemeRegistry`，当前激活主题记录在 `site_settings.active_theme`，用户定制覆盖记录在 `site_settings."theme_mods:<slug>"`。

详见 `docs/CMS_REFACTOR_PROPOSAL.md` §4。

## 包结构

```
themes/<slug>/
├── theme.toml      # 清单：名称/版本/模式/支持特性/模板映射/布局开关
├── DESIGN.md       # 设计真源（可来自 awesome-design-md，注明出处与 MIT 出处）
├── tokens.json     # ★ 机器契约：语义设计令牌（颜色/字体/排版/圆角/间距/阴影）
├── theme.css       # 由 tokens.json 生成的 :root{--gl-*}（可选，运行期也可由前端注入）
└── preview.png     # 后台主题库卡片缩略图（可选）
```

## tokens.json 是「语义令牌」契约

所有主题产出**同一组角色令牌**（不是原始色板）。前端组件只依赖角色（如 `--gl-color-accent`），与具体主题解耦——于是换主题 = 换一组令牌值。字段见 `tokens.schema.json`。

令牌 → CSS 变量命名规则：点路径转连字符并加 `--gl-` 前缀。
例：`color.bg` → `--gl-color-bg`，`type.body-line` → `--gl-type-body-line`。

## 两层主题模型

- **Tier-1 令牌主题**（已支持）：换配色/字体/圆角/阴影/间距 + 少量布局开关（`[layout]`）。纯运行期热切换，零部署。本目录的两个示例即 Tier-1。
- **Tier-2 模板主题**（规划中）：`[templates]` 指向编译期注册的 React 模板组件，换版式结构。

## 从 awesome-design-md 新增一个主题

[awesome-design-md](https://github.com/VoltAgent/awesome-design-md) 提供 ~31 个网站的 `DESIGN.md`（MIT）。导入步骤：

1. 取一个 `DESIGN.md`（如 `design-md/linear.app/DESIGN.md`）放入 `themes/<slug>/DESIGN.md`。
2. 运行 `scripts/import_design_md`（半自动抽 §9 颜色块 + §3 字体表 + §5 间距/圆角）生成 `tokens.json` 草稿。
3. **人工校对一次** tokens.json（自然语言描述无法 100% 自动解析）。
4. 写 `theme.toml` 清单。重启后端即被扫描装配，后台主题库可见、可激活。

## 内置示例

| slug | 风格 | 模式 | 来源 |
| --- | --- | --- | --- |
| `notion-editorial` | 暖中性、编辑感、留白 | light | awesome-design-md/notion |
| `sanity-noir` | 近黑画布、精密工程感、霓虹点缀 | dark | awesome-design-md/sanity |

> 令牌值取自对应 `DESIGN.md` 的 §9 Quick Color Reference、§3 字体表、§5/§6 间距圆角阴影。自定义字体（NotionInter / waldenburgNormal）按 awesome-design-md 的建议用 Inter / Space Grotesk 等替身。
