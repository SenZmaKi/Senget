//! Interacts with the github api
use crate::{
    github::serde_json_types::{
        AssetsResponseJson, ReleasesResponseJson, RepoResponseJson, SearchResponseJson,
    },
    includes::install::Installer,
};
use lazy_static::lazy_static;
use regex::{self, Regex};
use serde::{Serialize, Deserialize};

const GITHUB_HOME_URL: &str = "https://github.com";
const GITHUB_API_ENTRY_POINT: &str = "https://api.github.com";
lazy_static! {
    static ref VERSION_REGEX: Regex = Regex::new(r"(\d+(\.\d+)*)").unwrap();
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct Repo {
    pub name: String,
    pub full_name: String, // For example if the url is https://github.com/SenZmaKi/Senpwai the full_name is SenZmaKi/Senpwai
    pub url: String,
    pub description: Option<String>,
    pub language: Option<String>,
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

    async fn get_assets_url_by_version(
        &self,
        version: &str,
        client: &reqwest::Client,
    ) -> Result<Option<(String, String)>, reqwest::Error> {
        let url = self.generate_endpoint("releases");
        let releases_response_json: ReleasesResponseJson =
            client.get(url).send().await?.json().await?;
        let mut version = version;
        if releases_response_json.is_empty() {
            Ok(None)
        } else {
            if version != "latest" {
                version = match Repo::parse_version(version) {
                    None => return Ok(None),
                    Some(v) => v,
                };
            }
            for r in releases_response_json {
                let curr_ver = match Repo::parse_version(&r.tag_name) {
                    None => continue,
                    Some(v) => v,
                };
                if (version == curr_ver) || (version == "latest") {
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
            let inner_file_extension = name.split(".").last().unwrap();
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
        is_update: bool,
    ) -> Result<Option<Installer>, reqwest::Error> {
        let (target_assets_url, returned_version) =
            match self.get_assets_url_by_version(version, client).await? {
                None => return Ok(None),
                Some(asset_url_and_version) => asset_url_and_version,
            };
        if is_update && returned_version == version {
            return Ok(None);
        }

        let assets = client.get(target_assets_url).send().await?.json().await?;
        let rel = self.parse_for_windows_installer(assets, returned_version);
        Ok(rel)
    }
    pub async fn get_latest_installer(
        &self,
        client: &reqwest::Client,
    ) -> Result<Option<Installer>, reqwest::Error> {
        self.get_installer(client, "latest", false).await
    }
    fn generate_endpoint(&self, resource: &str) -> String {
        format!(
            "{}/repos/{}/{}",
            GITHUB_API_ENTRY_POINT, self.full_name, resource
        )
    }
    pub fn parse_version(text: &str) -> Option<&str> {
        let mat: regex::Match = VERSION_REGEX.find(text)?;
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

    use crate::{
        github::api::{search, Repo},
        utils::{setup_client, SENPWAI_REPO},
    };

    fn repos() -> Vec<Repo> {
        vec![
            (*SENPWAI_REPO).to_owned(),
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
            let res = search(query, &setup_client())
                .await
                .expect("Successful search");
            for r in res.iter() {
                println!("{:?}", r)
            }
            results.push(res);
        }
    }

    #[tokio::test]
    async fn test_getting_installer() {
        let rel = SENPWAI_REPO
            .get_installer(&setup_client(), "2.0.7", false)
            .await
            .expect("Successfully get installer");
        let installer = rel.expect("Installer to be Some");
        assert_eq!(
            installer.url,
            "https://github.com/SenZmaKi/Senpwai/releases/download/v2.0.7/Senpwai-setup.exe"
        );
        assert_eq!(installer.version, "2.0.7");
        println!("\nResults getting installer\n");
        println!("{:?}", installer);
    }
}
