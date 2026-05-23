use std::path::Path;

#[derive(Debug, Clone, Copy)]
pub enum Section {
    Root,
    Plugins,
    Api,
}

pub fn write(path: &Path, title: &str, section: Section, body: &str) -> std::io::Result<()> {
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
      .prose-docs h1 {{ @apply text-2xl font-semibold mb-4 text-zinc-900 dark:text-zinc-100; }}
      .prose-docs h2 {{ @apply text-xl font-semibold mt-8 mb-3 text-zinc-900 dark:text-zinc-100; }}
      .prose-docs h3 {{ @apply text-lg font-medium mt-6 mb-2 text-zinc-900 dark:text-zinc-100; }}
      .prose-docs h4 {{ @apply text-sm font-medium mt-4 mb-2 text-zinc-700 dark:text-zinc-300; }}
      .prose-docs p {{ @apply mb-3 leading-relaxed text-zinc-700 dark:text-zinc-300; }}
      .prose-docs ul {{ @apply list-disc pl-5 mb-4 space-y-1 text-zinc-700 dark:text-zinc-300; }}
      .prose-docs ol {{ @apply list-decimal pl-5 mb-4 space-y-1 text-zinc-700 dark:text-zinc-300; }}
      .prose-docs a {{ @apply underline text-zinc-900 dark:text-zinc-100; }}
      .prose-docs code:not(pre code) {{ @apply font-mono text-sm px-1 py-0.5 border border-zinc-300 dark:border-zinc-700 rounded bg-zinc-100 dark:bg-zinc-900; }}
      .prose-docs pre {{ @apply font-mono text-sm p-4 mb-4 overflow-x-auto border border-zinc-300 dark:border-zinc-700 rounded bg-zinc-50 dark:bg-zinc-900 text-zinc-800 dark:text-zinc-200; }}
      .prose-docs pre code {{ @apply border-0 p-0 bg-transparent; }}
      .prose-docs table {{ @apply w-full text-sm mb-4 border-collapse; }}
      .prose-docs th {{ @apply border border-zinc-300 dark:border-zinc-700 px-3 py-2 text-left font-medium bg-zinc-100 dark:bg-zinc-900; }}
      .prose-docs td {{ @apply border border-zinc-300 dark:border-zinc-700 px-3 py-2 align-top; }}
      .prose-docs hr {{ @apply my-8 border-zinc-300 dark:border-zinc-700; }}
    }}
  </style>
</head>
<body class="bg-white text-zinc-900 dark:bg-zinc-950 dark:text-zinc-100 min-h-screen">
  {header}
  <main class="mx-auto max-w-3xl px-4 py-8 sm:px-6">
    <article class="prose-docs">{body}</article>
  </main>
  <footer class="border-t border-zinc-200 dark:border-zinc-800 py-6 text-center text-sm text-zinc-500">
    FeatherFly · <a href="https://github.com/MythicalLTD/featherfly">GitHub</a>
  </footer>
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
    }})();
  </script>
</body>
</html>"##,
            header = header(section),
        ),
    )
}

fn header(section: Section) -> String {
    let links = match section {
        Section::Root => root_nav(),
        Section::Plugins => plugin_nav(),
        Section::Api => api_nav(),
    };

    format!(
        r#"<header class="border-b border-zinc-200 dark:border-zinc-800">
  <div class="mx-auto max-w-3xl px-4 py-4 sm:px-6 flex flex-wrap items-center justify-between gap-3">
    <a href="{home}" class="font-semibold no-underline text-zinc-900 dark:text-zinc-100">FeatherFly Docs</a>
    <div class="flex flex-wrap items-center gap-3 text-sm">
      <nav class="flex flex-wrap gap-3">{links}</nav>
      <button id="theme-toggle" type="button" class="border border-zinc-300 dark:border-zinc-700 px-2 py-1 rounded text-sm bg-white dark:bg-zinc-900">Theme</button>
    </div>
  </div>
</header>"#,
        home = home_href(section),
        links = links,
    )
}

fn home_href(section: Section) -> &'static str {
    match section {
        Section::Root => "index.html",
        Section::Plugins => "../index.html",
        Section::Api => "../index.html",
    }
}

fn link(href: &str, label: &str) -> String {
    format!(
        r#"<a href="{href}" class="no-underline text-zinc-600 dark:text-zinc-400 hover:text-zinc-900 dark:hover:text-zinc-100">{label}</a>"#
    )
}

fn root_nav() -> String {
    [
        link("plugins/index.html", "Plugins"),
        link("api/index.html", "API"),
    ]
    .join("")
}

fn plugin_nav() -> String {
    [
        link("index.html", "Guide"),
        link("overview.html", "Overview"),
        link("events.html", "Events"),
        link("json-hooks.html", "JSON"),
        link("host-api.html", "Host API"),
        link("example.html", "Example"),
        link("../api/index.html", "HTTP"),
    ]
    .join("")
}

fn api_nav() -> String {
    [
        link("../plugins/index.html", "Plugins"),
        link("index.html", "Swagger"),
        link("endpoints.html", "Endpoints"),
    ]
    .join("")
}

pub fn page_title(title: &str, subtitle: &str) -> String {
    format!("<h1>{title}</h1><p>{subtitle}</p>")
}

pub fn link_list(items: &[(&str, &str, &str)]) -> String {
    let mut out = String::from("<ul>");
    for (href, title, desc) in items {
        out.push_str(&format!(
            "<li><a href=\"{href}\"><strong>{title}</strong></a> — {desc}</li>"
        ));
    }
    out.push_str("</ul>");
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

pub fn example_block(title: &str, summary: &str, register: &str, handler: &str) -> String {
    format!(
        "<h3><code>{title}</code></h3><p>{summary}</p><h4>Register</h4><pre><code>{}</code></pre><h4>Handler</h4><pre><code>{}</code></pre><hr>",
        html_escape(register),
        html_escape(handler),
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
    format!(
        "<h3><code>{method} {path}</code></h3><p><strong>{operation_id}</strong> — {summary}</p><p>Auth: {auth}</p><pre><code>{}</code></pre><hr>",
        html_escape(example),
    )
}

pub fn html_escape(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}
