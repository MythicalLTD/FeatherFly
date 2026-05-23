use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Section {
    Root,
    Plugins,
    PluginsNested,
    Api,
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

    pub const fn plugins_nested(active: &'static str) -> Self {
        Self {
            section: Section::PluginsNested,
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

struct NavLink {
    id: &'static str,
    label: &'static str,
    href: &'static str,
}

struct NavGroup {
    label: &'static str,
    links: &'static [NavLink],
}

pub fn write(path: &Path, title: &str, ctx: PageContext, body: &str) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    std::fs::write(
        path,
        format!(
            r##"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>{title} · FeatherFly</title>
  <script src="https://cdn.tailwindcss.com"></script>
  <script>tailwind.config = {{ darkMode: 'class' }};</script>
  <style type="text/tailwindcss">
    @layer components {{
      .prose-docs h1 {{ @apply text-2xl font-semibold mb-2 text-zinc-900 dark:text-zinc-100; }}
      .prose-docs .lead {{ @apply text-base text-zinc-600 dark:text-zinc-400 mb-6; }}
      .prose-docs h2 {{ @apply text-lg font-semibold mt-10 mb-3 text-zinc-900 dark:text-zinc-100 border-b border-zinc-200 dark:border-zinc-800 pb-2; }}
      .prose-docs h3 {{ @apply text-base font-medium mt-6 mb-2 text-zinc-900 dark:text-zinc-100; }}
      .prose-docs h4 {{ @apply text-sm font-medium mt-4 mb-2 text-zinc-700 dark:text-zinc-300; }}
      .prose-docs p {{ @apply mb-3 leading-relaxed text-zinc-700 dark:text-zinc-300 text-sm; }}
      .prose-docs ul {{ @apply list-disc pl-5 mb-4 space-y-1 text-zinc-700 dark:text-zinc-300 text-sm; }}
      .prose-docs ol {{ @apply list-decimal pl-5 mb-4 space-y-1 text-zinc-700 dark:text-zinc-300 text-sm; }}
      .prose-docs a {{ @apply underline text-zinc-900 dark:text-zinc-100; }}
      .prose-docs code:not(pre code) {{ @apply font-mono text-xs px-1 py-0.5 border border-zinc-300 dark:border-zinc-700 rounded bg-zinc-100 dark:bg-zinc-900; }}
      .prose-docs pre {{ @apply font-mono text-xs p-3 mb-4 overflow-x-auto border border-zinc-300 dark:border-zinc-700 rounded bg-zinc-50 dark:bg-zinc-900 text-zinc-800 dark:text-zinc-200; }}
      .prose-docs pre code {{ @apply border-0 p-0 bg-transparent text-xs; }}
      .prose-docs table {{ @apply w-full text-sm mb-6 border-collapse; }}
      .prose-docs th {{ @apply border border-zinc-300 dark:border-zinc-700 px-3 py-2 text-left text-xs font-medium bg-zinc-100 dark:bg-zinc-900; }}
      .prose-docs td {{ @apply border border-zinc-300 dark:border-zinc-700 px-3 py-2 align-top text-xs; }}
      .prose-docs hr {{ @apply my-8 border-zinc-300 dark:border-zinc-700; }}
      .prose-docs details {{ @apply mb-4 border border-zinc-300 dark:border-zinc-700 rounded; }}
      .prose-docs summary {{ @apply cursor-pointer px-3 py-2 text-sm font-medium bg-zinc-50 dark:bg-zinc-900 rounded; }}
      .prose-docs details[open] summary {{ @apply border-b border-zinc-300 dark:border-zinc-700 rounded-b-none; }}
      .prose-docs details .details-body {{ @apply p-3; }}
      .sidebar-link {{ @apply block px-3 py-1.5 text-sm rounded no-underline text-zinc-600 dark:text-zinc-400 hover:bg-zinc-100 dark:hover:bg-zinc-900 hover:text-zinc-900 dark:hover:text-zinc-100; }}
      .sidebar-link-active {{ @apply bg-zinc-100 dark:bg-zinc-900 text-zinc-900 dark:text-zinc-100 font-medium; }}
      .sidebar-group {{ @apply text-xs font-semibold uppercase tracking-wide text-zinc-500 px-3 pt-4 pb-1; }}
      .doc-card {{ @apply block p-4 border border-zinc-300 dark:border-zinc-700 rounded no-underline hover:bg-zinc-50 dark:hover:bg-zinc-900; }}
      .meta-grid {{ @apply grid grid-cols-1 sm:grid-cols-2 gap-3 mb-6; }}
      .meta-item {{ @apply border border-zinc-300 dark:border-zinc-700 rounded p-3; }}
      .meta-label {{ @apply text-xs font-medium text-zinc-500 uppercase tracking-wide mb-1; }}
      .meta-value {{ @apply text-sm text-zinc-800 dark:text-zinc-200; }}
    }}
  </style>
</head>
<body class="bg-white text-zinc-900 dark:bg-zinc-950 dark:text-zinc-100 min-h-screen">
  <div class="flex min-h-screen">
    <aside id="sidebar" class="fixed inset-y-0 left-0 z-40 w-64 border-r border-zinc-200 dark:border-zinc-800 bg-white dark:bg-zinc-950 transform -translate-x-full lg:translate-x-0 transition-transform duration-200 overflow-y-auto">
      <div class="p-4 border-b border-zinc-200 dark:border-zinc-800">
        <a href="{home}" class="font-semibold no-underline text-zinc-900 dark:text-zinc-100">FeatherFly</a>
        <p class="text-xs text-zinc-500 mt-1">Documentation</p>
      </div>
      <nav class="p-2 pb-8">{sidebar}</nav>
    </aside>
    <div id="sidebar-backdrop" class="fixed inset-0 z-30 bg-black/40 hidden lg:hidden"></div>
    <div class="flex-1 flex flex-col min-w-0 lg:ml-64">
      <header class="sticky top-0 z-20 border-b border-zinc-200 dark:border-zinc-800 bg-white/95 dark:bg-zinc-950/95 backdrop-blur px-4 py-3 flex items-center justify-between gap-3">
        <button id="sidebar-toggle" type="button" class="lg:hidden border border-zinc-300 dark:border-zinc-700 px-2 py-1 rounded text-sm">Menu</button>
        <div class="flex-1"></div>
        <button id="theme-toggle" type="button" class="border border-zinc-300 dark:border-zinc-700 px-2 py-1 rounded text-sm bg-white dark:bg-zinc-900">Theme</button>
      </header>
      <main class="flex-1 px-4 py-8 sm:px-8 max-w-3xl">
        <article class="prose-docs">{body}</article>
      </main>
      <footer class="border-t border-zinc-200 dark:border-zinc-800 py-4 px-4 text-center text-xs text-zinc-500">
        FeatherFly · <a href="https://github.com/MythicalLTD/featherfly">GitHub</a>
      </footer>
    </div>
  </div>
  <script>
    (function () {{
      const root = document.documentElement;
      const btn = document.getElementById('theme-toggle');
      function apply(theme) {{
        root.classList.toggle('dark', theme === 'dark');
        localStorage.setItem('featherfly-theme', theme);
        if (btn) btn.textContent = theme === 'dark' ? 'Light' : 'Dark';
      }}
      const saved = localStorage.getItem('featherfly-theme');
      if (saved === 'dark' || saved === 'light') apply(saved);
      else if (window.matchMedia('(prefers-color-scheme: dark)').matches) apply('dark');
      else apply('light');
      if (btn) btn.addEventListener('click', function () {{
        apply(root.classList.contains('dark') ? 'light' : 'dark');
      }});

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
    }})();
  </script>
</body>
</html>"##,
            home = home_href(ctx.section),
            sidebar = sidebar(ctx),
        ),
    )
}

fn home_href(section: Section) -> &'static str {
    match section {
        Section::Root => "index.html",
        Section::Plugins => "../index.html",
        Section::PluginsNested => "../../index.html",
        Section::Api => "../index.html",
    }
}

fn sidebar(ctx: PageContext) -> String {
    let groups = nav_groups(ctx.section);
    let mut out = String::new();

    for group in groups {
        out.push_str(&format!(
            r#"<div class="sidebar-group">{label}</div>"#,
            label = group.label
        ));
        for link in group.links {
            let class = if link.id == ctx.active {
                "sidebar-link sidebar-link-active"
            } else {
                "sidebar-link"
            };
            out.push_str(&format!(
                r#"<a href="{href}" class="{class}">{label}</a>"#,
                href = link.href,
                class = class,
                label = link.label,
            ));
        }
    }

    out
}

fn nav_groups(section: Section) -> &'static [NavGroup] {
    match section {
        Section::Root => ROOT_NAV,
        Section::Plugins => PLUGINS_NAV,
        Section::PluginsNested => PLUGINS_NESTED_NAV,
        Section::Api => API_NAV,
    }
}

const ROOT_NAV: &[NavGroup] = &[
    NavGroup {
        label: "Start",
        links: &[NavLink {
            id: "home",
            label: "Home",
            href: "index.html",
        }],
    },
    NavGroup {
        label: "Plugins",
        links: &[
            NavLink {
                id: "plugins",
                label: "Introduction",
                href: "plugins/index.html",
            },
            NavLink {
                id: "getting-started",
                label: "Getting started",
                href: "plugins/getting-started.html",
            },
            NavLink {
                id: "overview",
                label: "Overview",
                href: "plugins/overview.html",
            },
            NavLink {
                id: "events",
                label: "Lifecycle events",
                href: "plugins/events/index.html",
            },
            NavLink {
                id: "json-hooks",
                label: "JSON hooks",
                href: "plugins/json-hooks/index.html",
            },
            NavLink {
                id: "host-api",
                label: "Host API",
                href: "plugins/host-api.html",
            },
            NavLink {
                id: "example",
                label: "Full example",
                href: "plugins/example.html",
            },
        ],
    },
    NavGroup {
        label: "HTTP API",
        links: &[
            NavLink {
                id: "api-swagger",
                label: "Swagger UI",
                href: "api/index.html",
            },
            NavLink {
                id: "api-endpoints",
                label: "Endpoints",
                href: "api/endpoints.html",
            },
        ],
    },
];

const PLUGINS_NAV: &[NavGroup] = &[
    NavGroup {
        label: "Start",
        links: &[NavLink {
            id: "home",
            label: "Home",
            href: "../index.html",
        }],
    },
    NavGroup {
        label: "Plugins",
        links: &[
            NavLink {
                id: "plugins",
                label: "Introduction",
                href: "index.html",
            },
            NavLink {
                id: "getting-started",
                label: "Getting started",
                href: "getting-started.html",
            },
            NavLink {
                id: "overview",
                label: "Overview",
                href: "overview.html",
            },
            NavLink {
                id: "events",
                label: "Lifecycle events",
                href: "events/index.html",
            },
            NavLink {
                id: "event-config-loaded",
                label: "config.loaded",
                href: "events/config-loaded.html",
            },
            NavLink {
                id: "event-plugin-loaded",
                label: "plugin.loaded",
                href: "events/plugin-loaded.html",
            },
            NavLink {
                id: "event-daemon-starting",
                label: "daemon.starting",
                href: "events/daemon-starting.html",
            },
            NavLink {
                id: "event-daemon-started",
                label: "daemon.started",
                href: "events/daemon-started.html",
            },
            NavLink {
                id: "event-daemon-stopping",
                label: "daemon.stopping",
                href: "events/daemon-stopping.html",
            },
            NavLink {
                id: "json-hooks",
                label: "JSON hooks",
                href: "json-hooks/index.html",
            },
            NavLink {
                id: "json-response",
                label: "json.response",
                href: "json-hooks/response-body.html",
            },
            NavLink {
                id: "json-actions",
                label: "json.actions",
                href: "json-hooks/response-actions.html",
            },
            NavLink {
                id: "host-api",
                label: "Host API",
                href: "host-api.html",
            },
            NavLink {
                id: "example",
                label: "Full example",
                href: "example.html",
            },
        ],
    },
    NavGroup {
        label: "HTTP API",
        links: &[
            NavLink {
                id: "api-swagger",
                label: "Swagger UI",
                href: "../api/index.html",
            },
            NavLink {
                id: "api-endpoints",
                label: "Endpoints",
                href: "../api/endpoints.html",
            },
        ],
    },
];

const PLUGINS_NESTED_NAV: &[NavGroup] = &[
    NavGroup {
        label: "Start",
        links: &[NavLink {
            id: "home",
            label: "Home",
            href: "../../index.html",
        }],
    },
    NavGroup {
        label: "Plugins",
        links: &[
            NavLink {
                id: "plugins",
                label: "Introduction",
                href: "../index.html",
            },
            NavLink {
                id: "getting-started",
                label: "Getting started",
                href: "../getting-started.html",
            },
            NavLink {
                id: "overview",
                label: "Overview",
                href: "../overview.html",
            },
            NavLink {
                id: "events",
                label: "Lifecycle events",
                href: "../events/index.html",
            },
            NavLink {
                id: "event-config-loaded",
                label: "config.loaded",
                href: "../events/config-loaded.html",
            },
            NavLink {
                id: "event-plugin-loaded",
                label: "plugin.loaded",
                href: "../events/plugin-loaded.html",
            },
            NavLink {
                id: "event-daemon-starting",
                label: "daemon.starting",
                href: "../events/daemon-starting.html",
            },
            NavLink {
                id: "event-daemon-started",
                label: "daemon.started",
                href: "../events/daemon-started.html",
            },
            NavLink {
                id: "event-daemon-stopping",
                label: "daemon.stopping",
                href: "../events/daemon-stopping.html",
            },
            NavLink {
                id: "json-hooks",
                label: "JSON hooks",
                href: "../json-hooks/index.html",
            },
            NavLink {
                id: "json-response",
                label: "json.response",
                href: "../json-hooks/response-body.html",
            },
            NavLink {
                id: "json-actions",
                label: "json.actions",
                href: "../json-hooks/response-actions.html",
            },
            NavLink {
                id: "host-api",
                label: "Host API",
                href: "../host-api.html",
            },
            NavLink {
                id: "example",
                label: "Full example",
                href: "../example.html",
            },
        ],
    },
    NavGroup {
        label: "HTTP API",
        links: &[
            NavLink {
                id: "api-swagger",
                label: "Swagger UI",
                href: "../../api/index.html",
            },
            NavLink {
                id: "api-endpoints",
                label: "Endpoints",
                href: "../../api/endpoints.html",
            },
        ],
    },
];

const API_NAV: &[NavGroup] = &[
    NavGroup {
        label: "Start",
        links: &[NavLink {
            id: "home",
            label: "Home",
            href: "../index.html",
        }],
    },
    NavGroup {
        label: "Plugins",
        links: &[
            NavLink {
                id: "plugins",
                label: "Introduction",
                href: "../plugins/index.html",
            },
            NavLink {
                id: "getting-started",
                label: "Getting started",
                href: "../plugins/getting-started.html",
            },
            NavLink {
                id: "events",
                label: "Lifecycle events",
                href: "../plugins/events/index.html",
            },
            NavLink {
                id: "json-hooks",
                label: "JSON hooks",
                href: "../plugins/json-hooks/index.html",
            },
        ],
    },
    NavGroup {
        label: "HTTP API",
        links: &[
            NavLink {
                id: "api-swagger",
                label: "Swagger UI",
                href: "index.html",
            },
            NavLink {
                id: "api-endpoints",
                label: "Endpoints",
                href: "endpoints.html",
            },
        ],
    },
];

pub fn page_header(title: &str, subtitle: &str) -> String {
    format!("<h1>{title}</h1><p class=\"lead\">{subtitle}</p>")
}

pub fn card_grid(items: &[(&str, &str, &str)]) -> String {
    let mut out = String::from(r#"<div class="grid grid-cols-1 sm:grid-cols-2 gap-3 mb-6">"#);
    for (href, title, desc) in items {
        out.push_str(&format!(
            r#"<a href="{href}" class="doc-card"><strong class="block mb-1 text-zinc-900 dark:text-zinc-100">{title}</strong><span class="text-sm text-zinc-600 dark:text-zinc-400">{desc}</span></a>"#
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

pub fn endpoint_block(
    method: &str,
    path: &str,
    operation_id: &str,
    summary: &str,
    auth: &str,
    example: &str,
) -> String {
    details_block(
        &format!("{method} {path}"),
        &format!(
            "<p><strong>{operation_id}</strong> — {summary}</p><p>Auth: {auth}</p>{}",
            code_block(example),
        ),
    )
}

pub fn event_slug(name: &str) -> String {
    name.replace('.', "-")
}

pub fn html_escape(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}
