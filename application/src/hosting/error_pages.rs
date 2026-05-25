use std::path::{Path, PathBuf};

use anyhow::Context;

use crate::config::InnerConfig;

pub const ERRORS_CONTAINER: &str = "featherfly-errors";
pub const ERRORS_DYNAMIC_FILE: &str = "featherfly-errors.yml";

pub fn error_pages_directory(inner: &InnerConfig) -> PathBuf {
    PathBuf::from(&inner.system.data).join("error-pages")
}

pub fn traefik_dynamic_directory(inner: &InnerConfig) -> PathBuf {
    PathBuf::from(&inner.system.data).join("traefik-dynamic")
}

struct ErrorPageSpec {
    file: &'static str,
    code: u16,
    title: &'static str,
    message: &'static str,
}

const DEFAULT_PAGES: [ErrorPageSpec; 4] = [
    ErrorPageSpec {
        file: "404.html",
        code: 404,
        title: "Page not found",
        message: "The page you requested does not exist or may have been moved.",
    },
    ErrorPageSpec {
        file: "500.html",
        code: 500,
        title: "Server error",
        message: "Something went wrong on our side. Please try again in a moment.",
    },
    ErrorPageSpec {
        file: "502.html",
        code: 502,
        title: "Bad gateway",
        message: "The upstream service is temporarily unavailable.",
    },
    ErrorPageSpec {
        file: "503.html",
        code: 503,
        title: "Service unavailable",
        message: "We are performing maintenance or the service is overloaded. Please try again shortly.",
    },
];

pub fn ensure_error_page_files(inner: &InnerConfig) -> Result<PathBuf, anyhow::Error> {
    let dir = error_pages_directory(inner);
    std::fs::create_dir_all(&dir)?;

    for page in DEFAULT_PAGES {
        let path = dir.join(page.file);
        if let Ok(custom) = load_custom_template(inner, page.file) {
            std::fs::write(&path, custom)?;
        } else {
            std::fs::write(
                &path,
                render_error_page(page.code, page.title, page.message, &inner.app_name),
            )?;
        }
    }

    Ok(dir)
}

fn render_error_page(code: u16, title: &str, message: &str, app_name: &str) -> String {
    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>{code} · {title}</title>
  <script src="https://cdn.tailwindcss.com"></script>
  <script>
    tailwind.config = {{
      theme: {{
        extend: {{
          colors: {{
            feather: {{
              400: '#a78bfa',
              500: '#8b5cf6',
              600: '#7c3aed',
              700: '#6d28d9',
            }}
          }},
          fontFamily: {{
            sans: ['Inter', 'ui-sans-serif', 'system-ui', 'sans-serif'],
          }}
        }}
      }}
    }}
  </script>
  <link rel="preconnect" href="https://fonts.googleapis.com">
  <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
  <link href="https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700&display=swap" rel="stylesheet">
</head>
<body class="min-h-screen bg-gradient-to-br from-slate-950 via-indigo-950 to-violet-950 text-slate-100 font-sans antialiased flex flex-col">
  <div class="flex-1 flex items-center justify-center px-6 py-16">
    <div class="w-full max-w-lg text-center">
      <div class="inline-flex items-center justify-center w-16 h-16 rounded-2xl bg-feather-600/20 border border-feather-500/30 mb-8">
        <svg class="w-8 h-8 text-feather-400" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5">
          <path stroke-linecap="round" stroke-linejoin="round" d="M12 19.5v-15m0 0l-6.75 6.75M12 4.5l6.75 6.75" />
        </svg>
      </div>
      <p class="text-7xl font-bold tracking-tight text-white mb-2">{code}</p>
      <h1 class="text-2xl font-semibold text-white mb-3">{title}</h1>
      <p class="text-slate-400 text-base leading-relaxed mb-8">{message}</p>
      <div class="rounded-xl border border-white/10 bg-white/5 backdrop-blur px-5 py-4 text-sm text-slate-300">
        This website was deployed using <span class="font-medium text-feather-400">{app_name}</span>
      </div>
    </div>
  </div>
  <footer class="pb-8 text-center text-xs text-slate-500">
    <p>&copy; <a href="https://featherpanel.com" class="hover:text-feather-400 transition-colors">featherpanel.com</a> &middot; Powered by FeatherPanel</p>
  </footer>
</body>
</html>
"#
    )
}

fn load_custom_template(inner: &InnerConfig, name: &str) -> Result<String, anyhow::Error> {
    let custom_dir = Path::new(&inner.hosting.error_pages_directory);
    if !custom_dir.exists() {
        anyhow::bail!("custom error page directory missing");
    }
    let path = custom_dir.join(name);
    if !path.exists() {
        anyhow::bail!("custom page {name} missing");
    }
    std::fs::read_to_string(&path).with_context(|| format!("failed to read {}", path.display()))
}

pub fn write_traefik_error_middleware(inner: &InnerConfig) -> Result<(), anyhow::Error> {
    let dynamic_dir = traefik_dynamic_directory(inner);
    std::fs::create_dir_all(&dynamic_dir)?;

    let yaml = r#"http:
  middlewares:
    featherfly-errors:
      errors:
        status:
          - "400-599"
        service: featherfly-error-pages
        query: "/{status}.html"
  services:
    featherfly-error-pages:
      loadBalancer:
        servers:
          - url: "http://featherfly-errors:80"
"#;

    std::fs::write(dynamic_dir.join(ERRORS_DYNAMIC_FILE), yaml)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_page_includes_app_name_and_featherpanel() {
        let html = render_error_page(404, "Page not found", "Missing.", "FeatherFly");
        assert!(html.contains("FeatherFly"));
        assert!(html.contains("featherpanel.com"));
        assert!(html.contains("tailwindcss.com"));
    }

    #[test]
    fn all_default_error_codes_render() {
        for spec in DEFAULT_PAGES {
            let html = render_error_page(spec.code, spec.title, spec.message, "TestApp");
            assert!(html.contains(spec.title));
            assert!(html.contains(&spec.code.to_string()));
        }
    }

    #[test]
    fn traefik_middleware_file_name_is_stable() {
        assert_eq!(ERRORS_DYNAMIC_FILE, "featherfly-errors.yml");
        assert_eq!(ERRORS_CONTAINER, "featherfly-errors");
    }
}
