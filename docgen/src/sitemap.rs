use std::path::Path;

pub fn write(output: &Path) -> std::io::Result<()> {
    let site = std::env::var("DOCS_SITE_URL")
        .unwrap_or_else(|_| "https://mythicaltld.github.io/featherfly".into());
    let site = site.trim_end_matches('/');

    let urls = search::all_urls();
    let mut body = String::from(r#"<?xml version="1.0" encoding="UTF-8"?>"#);
    body.push_str(r#"<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">"#);

    for url in urls {
        body.push_str(&format!(
            "<url><loc>{site}/{path}</loc><changefreq>weekly</changefreq></url>",
            path = url.trim_start_matches('/')
        ));
    }

    body.push_str("</urlset>");
    std::fs::write(output.join("sitemap.xml"), body)
}

mod search {
    pub fn all_urls() -> Vec<&'static str> {
        vec![
            "index.html",
            "plugins/index.html",
            "plugins/getting-started.html",
            "plugins/overview.html",
            "plugins/terminology.html",
            "plugins/architecture.html",
            "plugins/hooks-roadmap.html",
            "plugins/host-api.html",
            "plugins/macros.html",
            "plugins/return-codes.html",
            "plugins/source-tree.html",
            "plugins/example.html",
            "plugins/events/index.html",
            "plugins/events/config-loaded.html",
            "plugins/events/plugin-loaded.html",
            "plugins/events/daemon-starting.html",
            "plugins/events/daemon-started.html",
            "plugins/events/daemon-stopping.html",
            "plugins/config-hooks/index.html",
            "plugins/config-hooks/mutate.html",
            "plugins/request-hooks/index.html",
            "plugins/request-hooks/intercept.html",
            "plugins/request-hooks/middleware.html",
            "plugins/routes/index.html",
            "plugins/routes/register.html",
            "plugins/json-hooks/index.html",
            "plugins/json-hooks/response-body.html",
            "plugins/json-hooks/response-actions.html",
            "api/index.html",
            "api/health.html",
            "api/system.html",
            "api/system-overview.html",
            "api/system-plugins.html",
            "api/system-config.html",
            "api/system-restart.html",
            "api/system-update.html",
            "api/system-upgrade.html",
            "tests/index.html",
        ]
    }
}
