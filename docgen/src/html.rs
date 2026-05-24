use std::path::Path;

pub const GITHUB_TREE: &str = "https://github.com/MythicalLTD/featherfly/tree/main";
pub const GITHUB_BLOB: &str = "https://github.com/MythicalLTD/featherfly/blob/main";
pub const GITHUB_REPO: &str = "https://github.com/MythicalLTD/featherfly";
pub const ICON_URL: &str = "https://github.com/featherpanel-com.png";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Section {
    Root,
    Plugins,
    PluginEvents,
    PluginJsonHooks,
    PluginConfigHooks,
    PluginRequestHooks,
    PluginRoutes,
    Api,
    Tests,
}

#[derive(Debug, Clone, Copy)]
pub struct PageContext {
    pub section: Section,
    pub active: &'static str,
}

impl PageContext {
    pub const fn root(active: &'static str) -> Self {
        Self {
            section: Section::Root,
            active,
        }
    }

    pub const fn plugins(active: &'static str) -> Self {
        Self {
            section: Section::Plugins,
            active,
        }
    }

    pub const fn plugin_events(active: &'static str) -> Self {
        Self {
            section: Section::PluginEvents,
            active,
        }
    }

    pub const fn plugin_json_hooks(active: &'static str) -> Self {
        Self {
            section: Section::PluginJsonHooks,
            active,
        }
    }

    pub const fn plugin_config_hooks(active: &'static str) -> Self {
        Self {
            section: Section::PluginConfigHooks,
            active,
        }
    }

    pub const fn plugin_request_hooks(active: &'static str) -> Self {
        Self {
            section: Section::PluginRequestHooks,
            active,
        }
    }

    pub const fn plugin_routes(active: &'static str) -> Self {
        Self {
            section: Section::PluginRoutes,
            active,
        }
    }

    pub const fn tests(active: &'static str) -> Self {
        Self {
            section: Section::Tests,
            active,
        }
    }

    pub const fn api(active: &'static str) -> Self {
        Self {
            section: Section::Api,
            active,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PageMeta {
    pub description: String,
    pub canonical_path: String,
    pub source_links: Vec<(&'static str, &'static str)>,
}

impl PageMeta {
    pub fn new(description: impl Into<String>, canonical_path: impl Into<String>) -> Self {
        Self {
            description: description.into(),
            canonical_path: canonical_path.into(),
            source_links: Vec::new(),
        }
    }

    pub fn with_source(mut self, label: &'static str, repo_path: &'static str) -> Self {
        self.source_links.push((label, repo_path));
        self
    }
}

struct NavItem {
    id: &'static str,
    label: &'static str,
    path: &'static str,
}

struct NavGroup {
    overview: Option<NavItem>,
    children: &'static [NavItem],
}

fn section_dir(section: Section) -> &'static str {
    match section {
        Section::Root => "",
        Section::Plugins => "plugins/",
        Section::PluginEvents => "plugins/events/",
        Section::PluginJsonHooks => "plugins/json-hooks/",
        Section::PluginConfigHooks => "plugins/config-hooks/",
        Section::PluginRequestHooks => "plugins/request-hooks/",
        Section::PluginRoutes => "plugins/routes/",
        Section::Api => "api/",
        Section::Tests => "tests/",
    }
}

fn relative_href(from_dir: &str, to_path: &str) -> String {
    fn split_path(path: &str) -> Vec<&str> {
        path.trim_end_matches('/')
            .split('/')
            .filter(|s| !s.is_empty())
            .collect()
    }

    let from_parts = split_path(from_dir);
    let to_parts = split_path(to_path);

    let mut common = 0usize;
    while common < from_parts.len()
        && common < to_parts.len()
        && from_parts[common] == to_parts[common]
    {
        common += 1;
    }

    let ups = from_parts.len().saturating_sub(common);
    let mut result = std::iter::repeat_n("..", ups).collect::<Vec<_>>().join("/");
    let rest = &to_parts[common..];

    if rest.is_empty() {
        return if result.is_empty() {
            ".".into()
        } else {
            result
        };
    }

    if !result.is_empty() {
        result.push('/');
    }
    result.push_str(&rest.join("/"));
    result
}

fn build_id() -> String {
    std::env::var("DOCS_BUILD_ID").unwrap_or_else(|_| "dev".into())
}

fn docs_base() -> String {
    std::env::var("DOCS_BASE_PATH").unwrap_or_else(|_| "/featherfly/".into())
}

fn site_url() -> String {
    std::env::var("DOCS_SITE_URL")
        .unwrap_or_else(|_| "https://mythicaltld.github.io/featherfly".into())
}

pub fn write_page(
    path: &Path,
    title: &str,
    ctx: PageContext,
    meta: &PageMeta,
    body: &str,
) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let from_dir = section_dir(ctx.section);
    let home = relative_href(from_dir, "index.html");
    let search_index = relative_href(from_dir, "search-index.json");
    let build = build_id();
    let base = docs_base();
    let site = site_url().trim_end_matches('/').to_string();
    let canonical_url = format!("{site}/{}", meta.canonical_path.trim_start_matches('/'));
    let og_title = format!("{title} · FeatherFly");
    let source_bar = source_bar_html(&meta.source_links);
    let breadcrumbs = breadcrumb_html(from_dir, title, ctx.section);

    std::fs::write(
        path,
        format!(
            r##"<!DOCTYPE html>
<html lang="en" class="dark" data-docs-base="{base}" data-build="{build}">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <meta name="description" content="{description}">
  <meta name="robots" content="index, follow">
  <meta name="theme-color" content="#000000">
  <meta name="author" content="FeatherFly">
  <link rel="canonical" href="{canonical_url}">
  <link rel="icon" type="image/png" href="{icon}">
  <link rel="apple-touch-icon" href="{icon}">
  <meta property="og:type" content="website">
  <meta property="og:site_name" content="FeatherFly">
  <meta property="og:title" content="{og_title}">
  <meta property="og:description" content="{description}">
  <meta property="og:url" content="{canonical_url}">
  <meta property="og:image" content="{icon}">
  <meta name="twitter:card" content="summary">
  <meta name="twitter:title" content="{og_title}">
  <meta name="twitter:description" content="{description}">
  <meta name="twitter:image" content="{icon}">
  <meta http-equiv="Cache-Control" content="no-cache, no-store, must-revalidate">
  <title>{og_title}</title>
  <script src="https://cdn.tailwindcss.com?v={build}"></script>
  <style>
    :root {{
      --bg: #000000;
      --surface: #0a0a0a;
      --border: #1a1a1a;
      --text: #ffffff;
      --muted: #888888;
      --dim: #555555;
    }}
    * {{ scrollbar-width: thin; scrollbar-color: #ffffff #111111; }}
    *::-webkit-scrollbar {{ width: 8px; height: 8px; }}
    *::-webkit-scrollbar-track {{ background: #111111; }}
    *::-webkit-scrollbar-thumb {{ background: #ffffff; border-radius: 4px; border: 2px solid #111111; }}
    *::-webkit-scrollbar-thumb:hover {{ background: #cccccc; }}
    body {{ background: var(--bg); color: var(--text); }}
    .prose-docs h1 {{ font-size: 1.75rem; font-weight: 600; margin-bottom: 0.5rem; color: var(--text); letter-spacing: -0.02em; }}
    .prose-docs .lead {{ font-size: 1rem; color: var(--muted); margin-bottom: 1.5rem; line-height: 1.6; }}
    .prose-docs h2 {{ font-size: 1.125rem; font-weight: 600; margin-top: 2.5rem; margin-bottom: 0.75rem; color: var(--text); border-bottom: 1px solid var(--border); padding-bottom: 0.5rem; }}
    .prose-docs h3 {{ font-size: 0.95rem; font-weight: 600; margin-top: 1.5rem; margin-bottom: 0.5rem; color: var(--text); }}
    .prose-docs h4 {{ font-size: 0.85rem; font-weight: 600; margin-top: 1rem; margin-bottom: 0.5rem; color: var(--muted); text-transform: uppercase; letter-spacing: 0.05em; }}
    .prose-docs p {{ margin-bottom: 0.75rem; line-height: 1.7; color: #cccccc; font-size: 0.875rem; }}
    .prose-docs ul {{ list-style: disc; padding-left: 1.25rem; margin-bottom: 1rem; color: #cccccc; font-size: 0.875rem; }}
    .prose-docs ol {{ list-style: decimal; padding-left: 1.25rem; margin-bottom: 1rem; color: #cccccc; font-size: 0.875rem; }}
    .prose-docs li {{ margin-bottom: 0.25rem; }}
    .prose-docs a {{ color: var(--text); text-decoration: underline; text-underline-offset: 2px; }}
    .prose-docs a:hover {{ color: var(--muted); }}
    .prose-docs code:not(pre code) {{ font-family: ui-monospace, monospace; font-size: 0.75rem; padding: 0.125rem 0.375rem; border: 1px solid var(--border); border-radius: 0.25rem; background: var(--surface); color: var(--text); }}
    .prose-docs pre {{ font-family: ui-monospace, monospace; font-size: 0.75rem; padding: 1rem; margin-bottom: 1rem; overflow-x: auto; border: 1px solid var(--border); border-radius: 0.375rem; background: var(--surface); color: var(--text); line-height: 1.5; }}
    .prose-docs pre code {{ border: 0; padding: 0; background: transparent; }}
    .prose-docs table {{ width: 100%; font-size: 0.8125rem; margin-bottom: 1.5rem; border-collapse: collapse; }}
    .prose-docs th {{ border: 1px solid var(--border); padding: 0.5rem 0.75rem; text-align: left; font-weight: 600; background: var(--surface); color: var(--text); }}
    .prose-docs td {{ border: 1px solid var(--border); padding: 0.5rem 0.75rem; vertical-align: top; color: #cccccc; }}
    .prose-docs hr {{ margin: 2rem 0; border-color: var(--border); }}
    .prose-docs details {{ margin-bottom: 1rem; border: 1px solid var(--border); border-radius: 0.375rem; }}
    .prose-docs summary {{ cursor: pointer; padding: 0.625rem 0.875rem; font-size: 0.875rem; font-weight: 500; background: var(--surface); border-radius: 0.375rem; }}
    .prose-docs details[open] summary {{ border-bottom: 1px solid var(--border); border-radius: 0.375rem 0.375rem 0 0; }}
    .prose-docs details .details-body {{ padding: 0.875rem; }}
    .sidebar-link {{ display: block; padding: 0.375rem 0.75rem; font-size: 0.8125rem; border-radius: 0.25rem; text-decoration: none; color: var(--muted); }}
    .sidebar-link:hover {{ background: var(--surface); color: var(--text); }}
    .sidebar-link-active {{ background: var(--surface); color: var(--text); font-weight: 500; border-left: 2px solid var(--text); }}
    .sidebar-sublink {{ display: block; padding: 0.25rem 0.75rem 0.25rem 1.5rem; font-size: 0.75rem; border-radius: 0.25rem; text-decoration: none; color: var(--dim); }}
    .sidebar-sublink:hover {{ background: var(--surface); color: var(--text); }}
    .sidebar-sublink-active {{ background: var(--surface); color: var(--text); font-weight: 500; }}
    .sidebar-group {{ font-size: 0.625rem; font-weight: 700; text-transform: uppercase; letter-spacing: 0.1em; color: var(--dim); padding: 1rem 0.75rem 0.25rem; }}
    .doc-card {{ display: block; padding: 1rem; border: 1px solid var(--border); border-radius: 0.375rem; text-decoration: none; transition: border-color 0.15s; }}
    .doc-card:hover {{ border-color: #444444; background: var(--surface); }}
    .meta-grid {{ display: grid; grid-template-columns: repeat(auto-fit, minmax(180px, 1fr)); gap: 0.75rem; margin-bottom: 1.5rem; }}
    .meta-item {{ border: 1px solid var(--border); border-radius: 0.375rem; padding: 0.75rem; background: var(--surface); }}
    .meta-label {{ font-size: 0.625rem; font-weight: 700; text-transform: uppercase; letter-spacing: 0.08em; color: var(--dim); margin-bottom: 0.25rem; }}
    .meta-value {{ font-size: 0.8125rem; color: var(--text); }}
    .search-input {{ width: 100%; border: 1px solid var(--border); border-radius: 0.375rem; padding: 0.5rem 0.625rem; font-size: 0.8125rem; background: var(--surface); color: var(--text); }}
    .search-input:focus {{ outline: 1px solid var(--text); }}
    .search-results {{ margin-top: 0.5rem; max-height: 12rem; overflow-y: auto; }}
    .search-hit {{ display: block; padding: 0.375rem 0.5rem; font-size: 0.75rem; border-radius: 0.25rem; text-decoration: none; color: var(--muted); }}
    .search-hit:hover {{ background: var(--surface); color: var(--text); }}
    .source-bar {{ display: flex; flex-wrap: wrap; gap: 0.5rem 1rem; align-items: center; padding: 0.625rem 0.875rem; margin-bottom: 1.5rem; border: 1px solid var(--border); border-radius: 0.375rem; background: var(--surface); font-size: 0.75rem; }}
    .source-bar-label {{ color: var(--dim); font-weight: 600; text-transform: uppercase; letter-spacing: 0.06em; }}
    .source-link {{ color: var(--text); text-decoration: none; border-bottom: 1px solid var(--border); }}
    .source-link:hover {{ border-color: var(--text); }}
    .breadcrumbs {{ display: flex; flex-wrap: wrap; gap: 0.25rem; font-size: 0.75rem; color: var(--dim); margin-bottom: 1rem; }}
    .breadcrumbs a {{ color: var(--muted); text-decoration: none; }}
    .breadcrumbs a:hover {{ color: var(--text); text-decoration: underline; }}
    .breadcrumbs span {{ color: var(--dim); }}
    .top-link {{ font-size: 0.75rem; color: var(--muted); text-decoration: none; padding: 0.375rem 0.625rem; border: 1px solid var(--border); border-radius: 0.25rem; }}
    .top-link:hover {{ color: var(--text); border-color: #444444; }}
  </style>
</head>
<body class="min-h-screen">
  <div class="flex min-h-screen">
    <aside id="sidebar" class="fixed inset-y-0 left-0 z-40 w-72 border-r border-[#1a1a1a] bg-black transform -translate-x-full lg:translate-x-0 transition-transform duration-200 overflow-y-auto">
      <div class="p-4 border-b border-[#1a1a1a] flex items-center gap-3">
        <img src="{icon}" alt="FeatherFly" width="36" height="36" class="rounded-full border border-[#1a1a1a]">
        <div>
          <a href="{home}" class="font-semibold no-underline text-white block">FeatherFly</a>
          <p class="text-xs text-[#555555]">Documentation</p>
        </div>
      </div>
      <div class="p-3 border-b border-[#1a1a1a]">
        <input id="doc-search" type="search" placeholder="Search docs…" class="search-input" autocomplete="off" aria-label="Search documentation">
        <div id="search-results" class="search-results hidden"></div>
      </div>
      <nav class="p-2 pb-8" aria-label="Documentation">{sidebar}</nav>
    </aside>
    <div id="sidebar-backdrop" class="fixed inset-0 z-30 bg-black/80 hidden lg:hidden"></div>
    <div class="flex-1 flex flex-col min-w-0 lg:ml-72">
      <header class="sticky top-0 z-20 border-b border-[#1a1a1a] bg-black/90 backdrop-blur px-4 py-3 flex items-center justify-between gap-3">
        <button id="sidebar-toggle" type="button" class="lg:hidden top-link" aria-label="Open menu">Menu</button>
        <div class="flex items-center gap-2 ml-auto">
          <a href="{github_repo}" class="top-link" target="_blank" rel="noopener">GitHub</a>
          <a href="{github_tree}/application/src" class="top-link hidden sm:inline" target="_blank" rel="noopener">Source</a>
        </div>
      </header>
      <main class="flex-1 px-4 py-8 sm:px-10 max-w-4xl">
        {breadcrumbs}
        {source_bar}
        <article class="prose-docs">{body}</article>
      </main>
      <footer class="border-t border-[#1a1a1a] py-6 px-4 text-center text-xs text-[#555555]">
        <img src="{icon}" alt="" width="20" height="20" class="inline-block rounded-full mr-2 opacity-60">
        FeatherFly ·
        <a href="{github_repo}" class="text-[#888888] underline">GitHub</a> ·
        <a href="{github_tree}/featherfly-plugin-sdk" class="text-[#888888] underline">Plugin SDK</a> ·
        <a href="{github_tree}/application/src/daemon.rs" class="text-[#888888] underline">Daemon</a>
      </footer>
    </div>
  </div>
  <script>
    (function () {{
      const fromDir = {from_dir_json};
      const searchIndexUrl = {search_index_json};

      const sidebar = document.getElementById('sidebar');
      const backdrop = document.getElementById('sidebar-backdrop');
      const toggle = document.getElementById('sidebar-toggle');
      function openSidebar() {{
        sidebar.classList.remove('-translate-x-full');
        backdrop.classList.remove('hidden');
      }}
      function closeSidebar() {{
        sidebar.classList.add('-translate-x-full');
        backdrop.classList.add('hidden');
      }}
      if (toggle) toggle.addEventListener('click', openSidebar);
      if (backdrop) backdrop.addEventListener('click', closeSidebar);

      function relativeHref(toPath) {{
        const fromParts = fromDir.replace(/\/$/, '').split('/').filter(Boolean);
        const toParts = toPath.split('/').filter(Boolean);
        let common = 0;
        while (common < fromParts.length && common < toParts.length && fromParts[common] === toParts[common]) {{
          common++;
        }}
        const ups = fromParts.length - common;
        const prefix = ups ? Array(ups + 1).join('../') : '';
        return prefix + toParts.slice(common).join('/');
      }}

      let searchIndex = null;
      const searchInput = document.getElementById('doc-search');
      const searchResults = document.getElementById('search-results');
      fetch(searchIndexUrl + '?v={build}', {{ cache: 'no-store' }})
        .then(r => r.json())
        .then(data => {{ searchIndex = data; }})
        .catch(() => {{}});

      if (searchInput && searchResults) {{
        searchInput.addEventListener('input', function () {{
          const q = searchInput.value.trim().toLowerCase();
          if (!q || !searchIndex) {{
            searchResults.classList.add('hidden');
            searchResults.innerHTML = '';
            return;
          }}
          const hits = searchIndex.filter(function (e) {{
            return (e.title + ' ' + e.text).toLowerCase().includes(q);
          }}).slice(0, 10);
          if (!hits.length) {{
            searchResults.innerHTML = '<p class="px-2 py-1 text-xs text-[#555555]">No results</p>';
          }} else {{
            searchResults.innerHTML = hits.map(function (e) {{
              return '<a class="search-hit" href="' + relativeHref(e.url) + '"><strong class="text-white">' + e.title + '</strong><br><span class="text-[#555555]">' + (e.text.slice(0, 60)) + '</span></a>';
            }}).join('');
          }}
          searchResults.classList.remove('hidden');
        }});
      }}
    }})();
  </script>
</body>
</html>"##,
            sidebar = sidebar(ctx),
            icon = ICON_URL,
            github_repo = GITHUB_REPO,
            github_tree = GITHUB_TREE,
            from_dir_json = serde_json::to_string(from_dir).unwrap_or_else(|_| "\"\"".into()),
            search_index_json =
                serde_json::to_string(&search_index).unwrap_or_else(|_| "\"\"".into()),
            description = html_escape(&meta.description),
            breadcrumbs = breadcrumbs,
            source_bar = source_bar,
        ),
    )
}

fn breadcrumb_html(from_dir: &str, title: &str, section: Section) -> String {
    let home = relative_href(from_dir, "index.html");
    let mut parts = vec![format!(r#"<a href="{home}">Home</a>"#)];

    match section {
        Section::Plugins
        | Section::PluginEvents
        | Section::PluginJsonHooks
        | Section::PluginConfigHooks
        | Section::PluginRequestHooks
        | Section::PluginRoutes => {
            let plugins = relative_href(from_dir, "plugins/index.html");
            parts.push(format!(r#"<span>/</span><a href="{plugins}">Plugins</a>"#));
        }
        Section::Api => {
            let api = relative_href(from_dir, "api/index.html");
            parts.push(format!(r#"<span>/</span><a href="{api}">HTTP API</a>"#));
        }
        Section::Tests => {
            parts.push(format!(
                r#"<span>/</span><a href="{}">Tests</a>"#,
                relative_href(from_dir, "tests/index.html")
            ));
        }
        Section::Root => {}
    }

    parts.push(format!(r#"<span>/</span><span>{title}</span>"#));
    format!(
        r#"<nav class="breadcrumbs" aria-label="Breadcrumb">{}</nav>"#,
        parts.join(" ")
    )
}

pub fn github_source(repo_path: &str, label: &str) -> String {
    let url = if repo_path.is_empty() {
        GITHUB_REPO.to_string()
    } else if repo_path.starts_with("http") {
        repo_path.to_string()
    } else {
        format!("{GITHUB_BLOB}/{repo_path}")
    };
    format!(r#"<a href="{url}" class="source-link" target="_blank" rel="noopener">{label} ↗</a>"#)
}

fn source_bar_html(links: &[(&'static str, &'static str)]) -> String {
    if links.is_empty() {
        return String::new();
    }
    let mut items = vec![r#"<span class="source-bar-label">Source</span>"#.to_string()];
    for (label, path) in links {
        items.push(github_source(path, label));
    }
    format!(r#"<div class="source-bar">{}</div>"#, items.join(" "))
}

fn sidebar(ctx: PageContext) -> String {
    let from_dir = section_dir(ctx.section);
    let mut out = String::new();

    out.push_str(r#"<div class="sidebar-group">Start</div>"#);
    out.push_str(&nav_link(
        from_dir,
        ctx.active,
        "home",
        "Home",
        "index.html",
        false,
    ));

    out.push_str(r#"<div class="sidebar-group">Plugins</div>"#);
    for item in PLUGIN_TOP {
        out.push_str(&nav_link(
            from_dir, ctx.active, item.id, item.label, item.path, false,
        ));
    }
    for group in PLUGIN_GROUPS {
        out.push_str(&nav_group(from_dir, ctx.active, group));
    }
    for item in PLUGIN_BOTTOM {
        out.push_str(&nav_link(
            from_dir, ctx.active, item.id, item.label, item.path, false,
        ));
    }

    out.push_str(r#"<div class="sidebar-group">HTTP API</div>"#);
    for item in API_TOP {
        out.push_str(&nav_link(
            from_dir, ctx.active, item.id, item.label, item.path, false,
        ));
    }
    for item in API_EXTRA {
        out.push_str(&nav_link(
            from_dir, ctx.active, item.id, item.label, item.path, false,
        ));
    }
    for group in API_GROUPS {
        out.push_str(&nav_group(from_dir, ctx.active, group));
    }

    out.push_str(r#"<div class="sidebar-group">Reference</div>"#);
    for item in REFERENCE_LINKS {
        out.push_str(&nav_link(
            from_dir, ctx.active, item.id, item.label, item.path, false,
        ));
    }

    out
}

fn nav_link(from_dir: &str, active: &str, id: &str, label: &str, path: &str, sub: bool) -> String {
    let href = relative_href(from_dir, path);
    let class = if id == active {
        if sub {
            "sidebar-sublink sidebar-sublink-active"
        } else {
            "sidebar-link sidebar-link-active"
        }
    } else if sub {
        "sidebar-sublink"
    } else {
        "sidebar-link"
    };
    format!(r#"<a href="{href}" class="{class}">{label}</a>"#)
}

fn nav_group(from_dir: &str, active: &str, group: &NavGroup) -> String {
    let mut out = String::new();
    if let Some(ref o) = group.overview {
        out.push_str(&nav_link(from_dir, active, o.id, o.label, o.path, false));
    }
    for child in group.children {
        out.push_str(&nav_link(
            from_dir,
            active,
            child.id,
            child.label,
            child.path,
            true,
        ));
    }
    out
}

const PLUGIN_TOP: &[NavItem] = &[
    NavItem {
        id: "plugins",
        label: "Introduction",
        path: "plugins/index.html",
    },
    NavItem {
        id: "getting-started",
        label: "Getting started",
        path: "plugins/getting-started.html",
    },
    NavItem {
        id: "overview",
        label: "Overview",
        path: "plugins/overview.html",
    },
    NavItem {
        id: "terminology",
        label: "Terminology",
        path: "plugins/terminology.html",
    },
    NavItem {
        id: "architecture",
        label: "Architecture",
        path: "plugins/architecture.html",
    },
    NavItem {
        id: "hooks-roadmap",
        label: "Hooks roadmap",
        path: "plugins/hooks-roadmap.html",
    },
];

const PLUGIN_GROUPS: &[NavGroup] = &[
    NavGroup {
        overview: Some(NavItem {
            id: "events",
            label: "Events (28)",
            path: "plugins/events/index.html",
        }),
        children: &[],
    },
    NavGroup {
        overview: Some(NavItem {
            id: "config-hooks",
            label: "Config hooks",
            path: "plugins/config-hooks/index.html",
        }),
        children: &[NavItem {
            id: "config-mutate",
            label: "config.mutate",
            path: "plugins/config-hooks/mutate.html",
        }],
    },
    NavGroup {
        overview: Some(NavItem {
            id: "request-hooks",
            label: "Request hooks",
            path: "plugins/request-hooks/index.html",
        }),
        children: &[
            NavItem {
                id: "request-intercept",
                label: "request.intercept",
                path: "plugins/request-hooks/intercept.html",
            },
            NavItem {
                id: "middleware-inject",
                label: "middleware.inject",
                path: "plugins/request-hooks/middleware.html",
            },
        ],
    },
    NavGroup {
        overview: Some(NavItem {
            id: "plugin-routes",
            label: "Plugin routes",
            path: "plugins/routes/index.html",
        }),
        children: &[NavItem {
            id: "route-register",
            label: "route.register",
            path: "plugins/routes/register.html",
        }],
    },
    NavGroup {
        overview: Some(NavItem {
            id: "json-hooks",
            label: "JSON hooks",
            path: "plugins/json-hooks/index.html",
        }),
        children: &[
            NavItem {
                id: "json-response",
                label: "json.response",
                path: "plugins/json-hooks/response-body.html",
            },
            NavItem {
                id: "json-actions",
                label: "json.actions",
                path: "plugins/json-hooks/response-actions.html",
            },
        ],
    },
];

const PLUGIN_BOTTOM: &[NavItem] = &[
    NavItem {
        id: "host-api",
        label: "Host API",
        path: "plugins/host-api.html",
    },
    NavItem {
        id: "macros",
        label: "Macros",
        path: "plugins/macros.html",
    },
    NavItem {
        id: "return-codes",
        label: "Return codes",
        path: "plugins/return-codes.html",
    },
    NavItem {
        id: "source-tree",
        label: "Source tree",
        path: "plugins/source-tree.html",
    },
    NavItem {
        id: "example",
        label: "Full example",
        path: "plugins/example.html",
    },
];

const API_TOP: &[NavItem] = &[NavItem {
    id: "api-overview",
    label: "HTTP API",
    path: "api/index.html",
}];

const API_EXTRA: &[NavItem] = &[NavItem {
    id: "api-health",
    label: "Health",
    path: "api/health.html",
}];

const API_GROUPS: &[NavGroup] = &[NavGroup {
    overview: Some(NavItem {
        id: "api-system",
        label: "System",
        path: "api/system.html",
    }),
    children: &[
        NavItem {
            id: "api-system-overview",
            label: "Overview metrics",
            path: "api/system-overview.html",
        },
        NavItem {
            id: "api-system-plugins",
            label: "Plugins",
            path: "api/system-plugins.html",
        },
        NavItem {
            id: "api-system-update",
            label: "Updates",
            path: "api/system-update.html",
        },
        NavItem {
            id: "api-system-upgrade",
            label: "Upgrade",
            path: "api/system-upgrade.html",
        },
    ],
}];

const REFERENCE_LINKS: &[NavItem] = &[NavItem {
    id: "tests",
    label: "Unit tests",
    path: "tests/index.html",
}];

pub fn page_header(title: &str, subtitle: &str) -> String {
    format!("<h1>{title}</h1><p class=\"lead\">{subtitle}</p>")
}

pub fn card_grid(items: &[(&str, &str, &str)]) -> String {
    let mut out = String::from(r#"<div class="grid grid-cols-1 sm:grid-cols-2 gap-3 mb-6">"#);
    for (href, title, desc) in items {
        out.push_str(&format!(
            r#"<a href="{href}" class="doc-card"><strong class="block mb-1 text-white">{title}</strong><span class="text-sm text-[#888888]">{desc}</span></a>"#
        ));
    }
    out.push_str("</div>");
    out
}

pub fn meta_grid(items: &[(&str, &str)]) -> String {
    let mut out = String::from(r#"<div class="meta-grid">"#);
    for (label, value) in items {
        out.push_str(&format!(
            r#"<div class="meta-item"><div class="meta-label">{label}</div><div class="meta-value">{value}</div></div>"#
        ));
    }
    out.push_str("</div>");
    out
}

pub fn markdown(text: &str) -> String {
    use pulldown_cmark::{Options, Parser, html};

    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TABLES);

    let parser = Parser::new_ext(text, options);
    let mut output = String::new();
    html::push_html(&mut output, parser);
    output
}

pub fn text_block(text: &str) -> String {
    markdown(text)
}

pub fn details_block(title: &str, content: &str) -> String {
    format!(
        r#"<details><summary>{title}</summary><div class="details-body">{content}</div></details>"#
    )
}

pub fn code_block(code: &str) -> String {
    format!("<pre><code>{}</code></pre>", html_escape(code))
}

pub fn example_block(title: &str, register: &str, handler: &str) -> String {
    details_block(
        title,
        &format!(
            "<h4>Register</h4>{}<h4>Handler</h4>{}",
            code_block(register),
            code_block(handler),
        ),
    )
}

pub fn use_cases_list(items: &[&str]) -> String {
    let mut out = String::from("<ul>");
    for item in items {
        out.push_str(&format!("<li>{item}</li>"));
    }
    out.push_str("</ul>");
    out
}

pub fn event_slug(name: &str) -> String {
    name.replace('.', "-")
}

pub fn html_escape(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
