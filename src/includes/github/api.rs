//! Interacts with the github api

use crate::{
    github::serde_json_types::{
        AssetsResponseJson, ReleasesResponseJson, RepoResponseJson, SearchResponseJson,
    },
    includes::install::Installer,
};
use core::fmt;
use regex::{self, Regex};
use serde::{Deserialize, Serialize};

use super::serde_json_types::ReleaseResponseJson;

const GITHUB_API_ENTRY_POINT: &str = "https://api.github.com";

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct Repo {
    pub name: String,
    pub full_name: String, // For example if the url is https://github.com/SenZmaKi/Senpwai the full_name is SenZmaKi/Senpwai
    pub url: String,
    pub description: Option<String>,
    pub language: Option<String>,
    pub license: Option<String>,
}

impl fmt::Display for Repo {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let name = &self.name;
        let url = &self.url;
        let language = self.language.to_owned().unwrap_or_default();
        let description = self.description.to_owned().unwrap_or_default();
        let author = self.full_name.split('/').collect::<Vec<&str>>()[0];
        let license = self.license.to_owned().unwrap_or_default();

        write!(
            f,
            "Name: {}\nAuthor: {}\nDescription: {}\nRepository Url: {}\nPrimary Language: {}\nLicense: {}",
            name, author, description, url, language, license
        )
    }
}
impl Repo {
    // static member

    pub fn new(
        name: String,
        full_name: String,
        url: String,
        description: Option<String>,
        language: Option<String>,
        license: Option<String>,
    ) -> Repo {
        Repo {
            url,
            name,
            full_name,
            description,
            language,
            license,
        }
    }

    pub fn generate_version_regex() -> Regex {
        Regex::new(r"(\d+(\.\d+)*)").expect("Valid regex pattern")
    }

    async fn get_assets_url_by_version(
        &self,
        version: &str,
        client: &reqwest::Client,
        version_regex: &Regex,
    ) -> Result<Option<(String, String)>, reqwest::Error> {
        let url = self.generate_endpoint("releases");
        let releases_response_json: ReleasesResponseJson =
            client.get(url).send().await?.json().await?;
        let mut version = version;
        if releases_response_json.is_empty() {
            Ok(None)
        } else {
            version = match Repo::parse_version(version, version_regex) {
                None => return Ok(None),
                Some(v) => v,
            };
            for r in releases_response_json {
                let curr_ver = match Repo::parse_version(&r.tag_name, version_regex) {
                    None => continue,
                    Some(v) => v,
                };
                if version == curr_ver {
                    return Ok(Some((r.assets_url, curr_ver.to_owned())));
                }
            }
            Ok(None)
        }
    }

    fn parse_for_windows_installer(
        &self,
        assets: AssetsResponseJson,
        version: String,
    ) -> Option<Installer> {
        let mut file_extension = "".to_owned();
        let mut url = "".to_owned();
        let self_name_lower = &self.name.to_lowercase();
        for asset in assets {
            let name_lower = asset.name.to_lowercase();
            let inner_file_extension = name_lower.split('.').last().unwrap_or_default();
            let is_msi = inner_file_extension == "msi";
            if is_msi || inner_file_extension == "exe" {
                let is_installer = is_msi || name_lower.contains("installer") || name_lower.contains("setup");
                if url.is_empty() || is_installer
                {
                    url = asset.browser_download_url;
                    file_extension = inner_file_extension.to_owned();
                    let is_perfect_match = is_installer && name_lower.contains(self_name_lower);
                    if is_perfect_match {
                            break;
                    }
                }
            }
        }
        if url.is_empty() {
            return None;
        }
        Some(Installer::new(
            self.name.to_owned(),
            file_extension,
            url,
            version,
        ))
    }
    pub async fn get_installer(
        &self,
        client: &reqwest::Client,
        version: &str,
        version_regex: &Regex,
    ) -> Result<Option<Installer>, reqwest::Error> {
        let (target_assets_url, returned_version) = match self
            .get_assets_url_by_version(version, client, version_regex)
            .await?
        {
            None => return Ok(None),
            Some(asset_url_and_version) => asset_url_and_version,
        };
        let assets = client.get(target_assets_url).send().await?.json().await?;
        Ok(self.parse_for_windows_installer(assets, returned_version))
    }
    pub async fn get_latest_installer(
        &self,
        client: &reqwest::Client,
        version_regex: &Regex,
    ) -> Result<Option<Installer>, reqwest::Error> {
        let url = self.generate_endpoint("releases/latest");
        let response = client.get(url).send().await?;
        if response.status() == 404 {
            return Ok(None);
        }
        let release_response_json: ReleaseResponseJson = response.json().await?;
        if let Some(version) = Repo::parse_version(&release_response_json.tag_name, version_regex) {
            return Ok(
                self.parse_for_windows_installer(release_response_json.assets, version.to_string())
            );
        }
        Ok(None)
    }
    fn generate_endpoint(&self, resource: &str) -> String {
        format!(
            "{}/repos/{}/{}",
            GITHUB_API_ENTRY_POINT, self.full_name, resource
        )
    }
    pub fn parse_version<'a>(text: &'a str, version_regex: &Regex) -> Option<&'a str> {
        let mat: regex::Match = version_regex.find(text)?;
        Some(mat.as_str())
    }
}

pub fn extract_repo(repo_response_json: RepoResponseJson) -> Repo {
    Repo::new(
        repo_response_json.name,
        repo_response_json.full_name,
        repo_response_json.html_url,
        repo_response_json.description,
        repo_response_json.language,
        repo_response_json
            .license
            .map(|l| l.name.unwrap_or_default()),
    )
}
pub async fn search(query: &str, client: &reqwest::Client) -> Result<Vec<Repo>, reqwest::Error> {
    let url = format!("{GITHUB_API_ENTRY_POINT}/search/repositories?q={query}&per_page=10");
    let search_response_json: SearchResponseJson = client.get(url).send().await?.json().await?;
    let mut results = Vec::new();
    for repo_response_json in search_response_json.items {
        results.push(extract_repo(repo_response_json))
    }
    Ok(results)
}

#[cfg(test)]
pub mod tests {

    use crate::includes::{
        github::api::{search, Repo},
        test_utils::{client, senpwai_repo},
    };

    #[tokio::test]
    async fn test_search() {
        let queries = vec!["Senpwai", "empty-repo", "zohofberibp09u0&_+*"];
        let mut results = Vec::new();
        for (idx, query) in queries.iter().enumerate() {
            println!("\nResults of Search {}\n", idx + 1);
            let search_results = search(query, &client()).await.expect("Ok(search_results)");
            for r in search_results.iter() {
                println!("{}\n", r)
            }
            results.push(search_results);
        }
    }

    #[tokio::test]
    async fn test_getting_latest_installer() {
        let installer = senpwai_repo()
            .get_latest_installer(&client(), &Repo::generate_version_regex())
            .await
            .expect("Getting latest installer");
        let installer = installer.expect("Some(installer)");
        println!("\nResults getting latest installer\n");
        println!("{:?}\n", installer);
    }

    #[tokio::test]
    async fn test_getting_installer() {
        let installer = senpwai_repo()
            .get_installer(&client(), "2.0.7", &Repo::generate_version_regex())
            .await
            .expect("Getting installer");
        let installer = installer.expect("Some(installer)");
        assert_eq!(
            installer.url,
            "https://github.com/SenZmaKi/Senpwai/releases/download/v2.0.7/Senpwai-setup.exe"
        );
        assert_eq!(installer.version, "2.0.7");
        println!("\nResults getting installer\n");
        println!("{:?}", installer);
    }
}

