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
    use featherfly_plugin_sdk::metadata::EVENT_DOCS;

    pub fn all_urls() -> Vec<String> {
        let mut urls = vec![
            "index.html".into(),
            "plugins/index.html".into(),
            "plugins/getting-started.html".into(),
            "plugins/overview.html".into(),
            "plugins/terminology.html".into(),
            "plugins/architecture.html".into(),
            "plugins/hooks-roadmap.html".into(),
            "plugins/host-api.html".into(),
            "plugins/macros.html".into(),
            "plugins/return-codes.html".into(),
            "plugins/source-tree.html".into(),
            "plugins/example.html".into(),
            "plugins/events/index.html".into(),
            "plugins/config-hooks/index.html".into(),
            "plugins/config-hooks/mutate.html".into(),
            "plugins/request-hooks/index.html".into(),
            "plugins/request-hooks/intercept.html".into(),
            "plugins/request-hooks/middleware.html".into(),
            "plugins/routes/index.html".into(),
            "plugins/routes/register.html".into(),
            "plugins/json-hooks/index.html".into(),
            "plugins/json-hooks/response-body.html".into(),
            "plugins/json-hooks/response-actions.html".into(),
        ];

        for doc in EVENT_DOCS {
            urls.push(format!(
                "plugins/events/{}.html",
                doc.name.replace('.', "-")
            ));
        }

        urls
    }
}

#[cfg(test)]
mod tests {
    use super::search;
    use featherfly_plugin_sdk::metadata::EVENT_DOCS;

    #[test]
    fn sitemap_urls_include_every_event_page() {
        let urls = search::all_urls();

        for doc in EVENT_DOCS {
            let expected = format!("plugins/events/{}.html", doc.name.replace('.', "-"));
            assert!(
                urls.iter().any(|url| url == &expected),
                "sitemap missing {expected}"
            );
        }
    }
}
