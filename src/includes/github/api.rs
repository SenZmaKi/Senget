//! Interacts with the github api
use core::fmt;

use crate::{
    github::serde_json_types::{
        AssetsResponseJson, ReleasesResponseJson, RepoResponseJson, SearchResponseJson,
    },
    includes::install::Installer,
};
use regex::{self, Regex};
use serde::{Deserialize, Serialize};

use super::serde_json_types::ReleaseResponseJson;

const GITHUB_HOME_URL: &str = "https://github.com";
const GITHUB_API_ENTRY_POINT: &str = "https://api.github.com";

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct Repo {
    pub name: String,
    pub full_name: String, // For example if the url is https://github.com/SenZmaKi/Senpwai the full_name is SenZmaKi/Senpwai
    pub url: String,
    pub description: Option<String>,
    pub language: Option<String>,
}

impl fmt::Display for Repo {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let name = &self.name;
        let url = &self.url;
        let language = self.language.to_owned().unwrap_or("Unknown".to_owned());
        let description = self.description.to_owned().unwrap_or("Unknown".to_owned());
        let author = self.full_name.split("/").collect::<Vec<&str>>()[0];

        write!(
            f,
            "Name: {}\nAuthor: {}\nRepository Url: {}\nPrimary Language: {}\nDescription: {}",
            name, author, url, language, description
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
    ) -> Repo {
        Repo {
            url,
            name,
            full_name,
            description,
            language,
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
            return Ok(None);
        }
    }

    fn parse_for_windows_installer(
        &self,
        assets: AssetsResponseJson,
        version: String,
    ) -> Option<Installer> {
        let mut file_extension = "".to_owned();
        let mut url = "".to_owned();
        for asset in assets {
            let name = asset.name;
            let inner_file_extension = name.split(".").last().unwrap_or_default();
            if inner_file_extension == "exe" || inner_file_extension == "msi" {
                file_extension = inner_file_extension.to_owned();
                url = asset.browser_download_url;
            }
        }
        if url == "" {
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
        let release_response_json: ReleaseResponseJson =
            client.get(url).send().await?.json().await?;
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

    fn repos() -> Vec<Repo> {
        vec![
            senpwai_repo(),
            Repo::new(
                "NyakaMwizi".to_owned(),
                "SenZmaKi/NyakaMwizi".to_owned(),
                "https://github.com/senzmaki/nyakamwizi".to_owned(),
                Some("A credit card fraud detection machine learning model".to_owned()),
                Some("Jupyter Notebook".to_owned()),
            ),
            Repo::new(
                "Hatt".to_owned(),
                "Frenchgithubuser/Hatt".to_owned(),
                "https://github.com/frenchgithubuser/hatt".to_owned(),
                Some("DDL Meta search engine".to_owned()),
                Some("Go".to_owned()),
            ),
            Repo::new(
                "Gin-Swagger".to_owned(),
                "Swaggo/Gin-Swagger".to_owned(),
                "https://github.com/swaggo/gin-swagger".to_owned(),
                None,
                Some("Go".to_owned()),
            ),
        ]
    }

    #[tokio::test]
    async fn test_search() {
        let queries = vec!["Senpwai", "empty-repo", "zohofberibp09u0&_+*"];
        let mut results = Vec::new();
        for (idx, query) in queries.iter().enumerate() {
            println!("\nResults of Search {}\n", idx + 1);
            let search_results = search(query, &client()).await.expect("Ok(search_results)");
            for r in search_results.iter() {
                println!("{}", r)
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
        println!("{:?}", installer);
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
