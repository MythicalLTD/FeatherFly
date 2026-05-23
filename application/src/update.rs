use anyhow::{Context, bail};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

const USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));
const STABLE_TAG_PREFIX: &str = "release-";
const NIGHTLY_TAG: &str = "nightly";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum UpdateChannel {
    #[default]
    Stable,
    Nightly,
    Disabled,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpdateStatus {
    pub channel: UpdateChannel,
    pub current_version: String,
    pub current_commit: String,
    pub latest_version: Option<String>,
    pub latest_commit: Option<String>,
    pub update_available: bool,
    pub download_url: Option<String>,
    pub download_name: Option<String>,
    pub release_url: Option<String>,
    pub published_at: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GithubRelease {
    tag_name: String,
    html_url: String,
    published_at: Option<String>,
    target_commitish: Option<String>,
    assets: Vec<GithubAsset>,
    prerelease: bool,
    draft: bool,
}

#[derive(Debug, Deserialize)]
struct GithubAsset {
    name: String,
    browser_download_url: String,
}

pub fn platform_asset_name() -> &'static str {
    match std::env::consts::ARCH {
        "x86_64" => "featherfly-x86_64-linux",
        "aarch64" => "featherfly-aarch64-linux",
        "riscv64" => "featherfly-riscv64-linux",
        "powerpc64" => "featherfly-ppc64le-linux",
        arch => {
            tracing::warn!(arch, "unknown architecture for update asset lookup");
            "featherfly-x86_64-linux"
        }
    }
}

pub fn releases_page_url() -> String {
    format!("{}/releases", crate::GITHUB_REPOSITORY)
}

pub fn nightly_download_page_url() -> String {
    format!("{}/releases/tag/nightly", crate::GITHUB_REPOSITORY)
}

pub async fn check_update(channel: UpdateChannel) -> Result<UpdateStatus, anyhow::Error> {
    if channel == UpdateChannel::Disabled {
        return Ok(base_status(channel));
    }

    let release = match channel {
        UpdateChannel::Stable => fetch_stable_release().await?,
        UpdateChannel::Nightly => fetch_release_by_tag(NIGHTLY_TAG).await?,
        UpdateChannel::Disabled => unreachable!(),
    };

    let asset_name = platform_asset_name();
    let asset = release.assets.iter().find(|asset| asset.name == asset_name);

    let latest_version = parse_release_version(&release.tag_name);
    let latest_commit = release.target_commitish.clone();

    let update_available = match channel {
        UpdateChannel::Stable => latest_version
            .as_deref()
            .is_some_and(|latest| version_is_newer(latest, crate::VERSION)),
        UpdateChannel::Nightly => latest_commit
            .as_deref()
            .is_some_and(|latest| !commit_matches(latest, crate::GIT_COMMIT)),
        UpdateChannel::Disabled => false,
    };

    Ok(UpdateStatus {
        channel,
        current_version: crate::VERSION.to_string(),
        current_commit: crate::GIT_COMMIT.to_string(),
        latest_version,
        latest_commit,
        update_available,
        download_url: asset.map(|asset| asset.browser_download_url.clone()),
        download_name: asset.map(|asset| asset.name.clone()),
        release_url: Some(release.html_url),
        published_at: release.published_at,
    })
}

pub async fn fetch_checksums_for_channel(channel: UpdateChannel) -> Result<String, anyhow::Error> {
    let release = match channel {
        UpdateChannel::Stable => fetch_stable_release().await?,
        UpdateChannel::Nightly => fetch_release_by_tag(NIGHTLY_TAG).await?,
        UpdateChannel::Disabled => bail!("updates are disabled"),
    };

    fetch_checksums(&release).await
}

async fn fetch_checksums(release: &GithubRelease) -> Result<String, anyhow::Error> {
    let checksum_asset = release
        .assets
        .iter()
        .find(|asset| asset.name == "SHA256SUMS")
        .context("release is missing SHA256SUMS asset")?;

    let client = github_client()?;
    let response = client
        .get(&checksum_asset.browser_download_url)
        .send()
        .await
        .context("failed to download SHA256SUMS")?
        .error_for_status()
        .context("GitHub returned an error while downloading SHA256SUMS")?;

    response
        .text()
        .await
        .context("failed to read SHA256SUMS response body")
}

pub fn checksum_for_asset(checksums: &str, asset_name: &str) -> Option<String> {
    checksums.lines().find_map(|line| {
        let mut parts = line.split_whitespace();
        let hash = parts.next()?;
        let name = parts.next()?;
        (name == asset_name).then(|| hash.to_string())
    })
}

pub async fn download_release_asset(
    url: &str,
    destination: &std::path::Path,
) -> Result<(), anyhow::Error> {
    let client = github_client()?;
    let response = client
        .get(url)
        .send()
        .await
        .context("failed to download release asset")?
        .error_for_status()
        .context("GitHub returned an error while downloading release asset")?;

    let bytes = response
        .bytes()
        .await
        .context("failed to read release asset body")?;

    if let Some(parent) = destination.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }

    tokio::fs::write(destination, bytes)
        .await
        .with_context(|| format!("failed to write {}", destination.display()))?;

    Ok(())
}

pub fn verify_file_sha256(path: &std::path::Path, expected: &str) -> Result<(), anyhow::Error> {
    use sha2::Digest;

    let bytes =
        std::fs::read(path).with_context(|| format!("failed to read {}", path.display()))?;
    let digest = hex::encode(sha2::Sha256::digest(bytes));

    if digest.eq_ignore_ascii_case(expected) {
        Ok(())
    } else {
        bail!("checksum mismatch for {}", path.display())
    }
}

fn base_status(channel: UpdateChannel) -> UpdateStatus {
    UpdateStatus {
        channel,
        current_version: crate::VERSION.to_string(),
        current_commit: crate::GIT_COMMIT.to_string(),
        latest_version: None,
        latest_commit: None,
        update_available: false,
        download_url: None,
        download_name: None,
        release_url: None,
        published_at: None,
    }
}

async fn fetch_stable_release() -> Result<GithubRelease, anyhow::Error> {
    let release = fetch_release_by_tag("latest").await?;

    if release.tag_name.starts_with(STABLE_TAG_PREFIX) && !release.draft {
        return Ok(release);
    }

    let releases = fetch_all_releases().await?;
    releases
        .into_iter()
        .find(|release| {
            release.tag_name.starts_with(STABLE_TAG_PREFIX) && !release.draft && !release.prerelease
        })
        .context("no stable FeatherFly release was found on GitHub")
}

async fn fetch_release_by_tag(tag: &str) -> Result<GithubRelease, anyhow::Error> {
    let url = if tag == "latest" {
        format!(
            "https://api.github.com/repos/{}/releases/latest",
            crate::GITHUB_REPO
        )
    } else {
        format!(
            "https://api.github.com/repos/{}/releases/tags/{tag}",
            crate::GITHUB_REPO
        )
    };

    github_client()?
        .get(url)
        .send()
        .await
        .context("failed to query GitHub releases API")?
        .error_for_status()
        .context("GitHub releases API returned an error")?
        .json::<GithubRelease>()
        .await
        .context("failed to decode GitHub release response")
}

async fn fetch_all_releases() -> Result<Vec<GithubRelease>, anyhow::Error> {
    github_client()?
        .get(format!(
            "https://api.github.com/repos/{}/releases?per_page=20",
            crate::GITHUB_REPO
        ))
        .send()
        .await
        .context("failed to query GitHub releases API")?
        .error_for_status()
        .context("GitHub releases API returned an error")?
        .json::<Vec<GithubRelease>>()
        .await
        .context("failed to decode GitHub releases response")
}

fn github_client() -> Result<reqwest::Client, anyhow::Error> {
    reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .context("failed to build HTTP client")
}

fn parse_release_version(tag_name: &str) -> Option<String> {
    tag_name
        .strip_prefix(STABLE_TAG_PREFIX)
        .map(str::to_string)
        .or_else(|| (tag_name != NIGHTLY_TAG).then(|| tag_name.to_string()))
}

fn version_is_newer(latest: &str, current: &str) -> bool {
    match (
        semver::Version::parse(latest),
        semver::Version::parse(current),
    ) {
        (Ok(latest), Ok(current)) => latest > current,
        _ => latest != current,
    }
}

fn commit_matches(latest: &str, current: &str) -> bool {
    if latest == current {
        return true;
    }

    latest.starts_with(current) || current.starts_with(latest)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_common_platform_asset_names() {
        assert!(platform_asset_name().starts_with("featherfly-"));
    }

    #[test]
    fn parses_stable_release_tags() {
        assert_eq!(
            parse_release_version("release-0.2.0").as_deref(),
            Some("0.2.0")
        );
        assert_eq!(parse_release_version("nightly").as_deref(), None);
    }

    #[test]
    fn detects_newer_semver_versions() {
        assert!(version_is_newer("0.2.0", "0.1.0"));
        assert!(!version_is_newer("0.1.0", "0.1.0"));
        assert!(!version_is_newer("0.1.0", "0.2.0"));
    }

    #[test]
    fn matches_short_and_full_commits() {
        assert!(commit_matches("abcdef1234567890", "abcdef1"));
        assert!(!commit_matches("abcdef1", "1234567"));
    }

    #[test]
    fn reads_checksum_for_asset() {
        let checksums = "deadbeef  featherfly-x86_64-linux\n";
        assert_eq!(
            checksum_for_asset(checksums, "featherfly-x86_64-linux").as_deref(),
            Some("deadbeef")
        );
    }
}
