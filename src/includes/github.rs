//! Module for interacting with the Github api
use lazy_static::lazy_static;
use regex;
use serde_json;
use std::collections::HashMap;

use super::install::Installer;

const GITHUB_HOME_URL: &str = "https://github.com";
const GITHUB_API_ENTRY_POINT: &str = "https://api.github.com";
lazy_static! {
    static ref VERSION_REGEX: regex::Regex = regex::Regex::new(r"(\d+(\.\d+)*)").unwrap();
}
#[derive(Debug)]
struct Repo {
    title: String,
    url: String,
    name: String, // For example if the url is https://github.com/SenZmaKi/Senpwai the name is SenZmaKi/Senpwai
    desc: Option<String>,
    lang: Option<String>,
}
impl Repo {
    fn from(title: String, url: String, desc: Option<String>, lang: Option<String>) -> Repo {
        let name = url
            .split(&format!("{}/", GITHUB_HOME_URL))
            .collect::<Vec<&str>>()[1]
            .to_owned();
        Repo {
            url,
            title,
            name,
            desc,
            lang,
        }
    }

    async fn get_assets_url_by_version(
        &self,
        version: &str,
        client: &reqwest::Client,
    ) -> Result<Option<(String, String)>, reqwest::Error> {
        let url = self.generate_endpoint("releases");
        let response = client.get(url).send().await?;
        let releases: Vec<HashMap<String, serde_json::Value>> = response.json().await?;
        let mut version = version;
        if releases == [] {
            Ok(None)
        } else {
            if version != "latest" {
                version = match Repo::parse_version(version) {
                    None => return Ok(None),
                    Some(v) => v,
                };
            }
            for r in releases {
                let curr_ver =
                    match Repo::parse_version(r.get("tag_name").unwrap().as_str().unwrap()) {
                        None => continue,
                        Some(v) => v,
                    };
                if (version == curr_ver) || (version == "latest") {
                    let assets_url = r.get("assets_url").unwrap().as_str().unwrap().to_owned();
                    return Ok(Some((assets_url, curr_ver.to_owned())));
                }
            }
            return Ok(None);
        }
    }

    fn parse_for_windows_installer(
        &self,
        assets: Vec<HashMap<String, serde_json::Value>>,
        version: &str,
    ) -> Option<Installer> {
        let mut file_extension = "".to_owned();
        let mut url = "".to_owned();
        for asset in assets {
            let name = asset.get("name").unwrap().as_str().unwrap().to_lowercase();
            let inner_file_extension = name.split(".").last().unwrap();
            if inner_file_extension == "exe" || inner_file_extension == "msi" {
                let inner_url = asset
                    .get("browser_download_url")
                    .unwrap()
                    .as_str()
                    .unwrap()
                    .to_owned();
                file_extension = inner_file_extension.to_owned();
                url = inner_url.to_owned();
            }
        }
        if url == "" {
            return None;
        }
        Some(Installer::from(&self.title, &file_extension, &url, version))
    }
    pub async fn get_installer(
        &self,
        client: &reqwest::Client,
        version: &str,
    ) -> Result<Option<Installer>, reqwest::Error> {
        let (target_assets_url, version) =
            match self.get_assets_url_by_version(version, client).await? {
                None => return Ok(None),
                Some(a) => a,
            };
        let response = client.get(target_assets_url).send().await?;
        let json = response.json::<Vec<HashMap<String, serde_json::Value>>>();
        let assets = json.await?;
        let rel = self.parse_for_windows_installer(assets, &version);
        Ok(rel)
    }
    pub async fn get_latest_installer(
        &self,
        client: &reqwest::Client,
    ) -> Result<Option<Installer>, reqwest::Error> {
        self.get_installer(client, "latest").await
    }
    fn generate_endpoint(&self, resource: &str) -> String {
        format!(
            "{}/repos/{}/{}",
            GITHUB_API_ENTRY_POINT, self.name, resource
        )
    }
    pub fn parse_version(text: &str) -> Option<&str> {
        let mat: regex::Match = VERSION_REGEX.find(text)?;
        Some(mat.as_str())
    }
}

pub async fn search(query: &str, client: &reqwest::Client) -> Result<Vec<Repo>, reqwest::Error> {
    let url = format!("{GITHUB_API_ENTRY_POINT}/search/repositories?q={query}&per_page=10");
    let response = client.get(url).send().await?;
    let json: HashMap<String, serde_json::Value> = response.json().await?;
    let items = json.get("items").unwrap().as_array().unwrap();
    let mut results = Vec::with_capacity(items.len());
    for item in items {
        let item = item.as_object().unwrap();
        let name = item.get("name").unwrap().as_str().unwrap().to_owned();
        let url = item.get("html_url").unwrap().as_str().unwrap().to_owned();
        let desc = item
            .get("description")
            .and_then(|val| val.as_str().map(|v| v.to_owned()));
        let lang = item
            .get("language")
            .and_then(|val| val.as_str().map(|v| v.to_owned()));
        results.push(Repo::from(name, url, desc, lang));
    }
    Ok(results)
}

#[cfg(test)]
mod tests {

    use super::{lazy_static, search, Repo};
    use crate::utils::CLIENT;
    lazy_static! {
        static ref REPOS: Vec<Repo> = vec![
            Repo::from(
                "Senpwai".to_owned(),
                "https://github.com/senzmaki/senpwai".to_owned(),
                Some("A desktop app for batch downloading anime".to_owned()),
                Some("Python".to_owned()),
            ),
            Repo::from(
                "NyakaMwizi".to_owned(),
                "https://github.com/senzmaki/nyakamwizi".to_owned(),
                Some("A credit card fraud detection machine learning model".to_owned()),
                Some("Jupyter Notebook".to_owned()),
            ),
            Repo::from(
                "Hatt".to_owned(),
                "https://github.com/frenchgithubuser/hatt".to_owned(),
                Some("DDL Meta search engine".to_owned()),
                Some("Go".to_owned()),
            ),
            Repo::from(
                "Swaggo".to_owned(),
                "https://github.com/swaggo/gin-swagger".to_owned(),
                None,
                Some("Go".to_owned()),
            ),
        ];
        static ref SENPWAI: &'static Repo = &REPOS[0];
    }

    #[tokio::test]
    async fn test_search() {
        let queries = vec!["Senpwai", "empty-repo", "zohofberibp09u0&_+*"];
        let mut results = Vec::new();
        for (idx, query) in queries.iter().enumerate() {
            println!("\nResults of Search {}\n", idx + 1);
            let res = search(query, &CLIENT).await.expect("Succesful search");
            for r in res.iter() {
                println!("{:?}", r)
            }
            results.push(res);
        }
        assert_eq!(results[0][0].title, "Senpwai");
        assert_eq!(results[2].len(), 0);
    }

    #[tokio::test]
    async fn test_getting_installer() {
        let rel = SENPWAI
            .get_installer(&CLIENT, "2.0.7")
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
