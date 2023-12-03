//! Module for interacting with the Github api
use lazy_static::lazy_static;
use regex;
use reqwest::{self, Request};
use serde_json;
use std::collections::HashMap;

const GITHUB_HOME_URL: &str = "https://github.com";
const GITHUB_API_ENTRY_POINT: &str = "https://api.github.com";

lazy_static! {
    static ref VERSION_REGEX: regex::Regex = regex::Regex::new(r"(\d+(\.\d+)*)").unwrap();
}

#[derive(Debug)]
pub struct Repo {
    title: String,
    url: String,
    name: String, // For example if the url is https://github.com/SenZmaKi/Senpwai the name is SenZmaKi/Senpwai
    desc: Option<String>,
    lang: Option<String>,
}

#[derive(Debug, Default, Clone)]
pub struct Release {
    url: String,
    version: String,
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

    fn fetch_asset_url_by_version(
        &self,
        mut version: &str,
        client: &reqwest::blocking::Client,
    ) -> Result<Option<(String, String)>, reqwest::Error> {
        let url = self.generate_endpoint("releases");
        let response = client.get(url).send()?;
        let releases: Vec<HashMap<String, serde_json::Value>> = response.json().unwrap();
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
                    let asset_url = r.get("assets_url").unwrap().as_str().unwrap().to_owned();
                    return Ok(Some((asset_url, curr_ver.to_owned())));
                }
            }
            return Ok(None);
        }
    }

    fn parse_for_best_asset(
        releases: Vec<HashMap<String, serde_json::Value>>,
        version: String,
    ) -> Option<Release> {
        let mut best_asset: Release = Release {
            url: "".to_owned(),
            version,
        };
        for asset in releases {
            let name = asset.get("name").unwrap().as_str().unwrap().to_lowercase();
            if name.ends_with(".exe") || name.ends_with(".msi") {
                let url = asset
                    .get("browser_download_url")
                    .unwrap()
                    .as_str()
                    .unwrap()
                    .to_owned();
                best_asset.url = url;
            }
        }
        if best_asset.url == "" {
            return None;
        }
        Some(best_asset)
    }
    pub fn fetch_release(
        &self,
        client: &reqwest::blocking::Client,
        version: &str,
    ) -> Result<Option<Release>, reqwest::Error> {
        let (latest_assets_url, version) = match self.fetch_asset_url_by_version(version, client)? {
            None => return Ok(None),
            Some(a) => a,
        };
        let response = client.get(latest_assets_url).send()?;
        let releases: Vec<HashMap<String, serde_json::Value>> = response.json().unwrap();
        let rel = Repo::parse_for_best_asset(releases, version);
        Ok(rel)
    }
    pub fn fetch_latest_release(
        &self,
        client: &reqwest::blocking::Client,
    ) -> Result<Option<Release>, reqwest::Error> {
        self.fetch_release(client, "latest")
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

pub fn search(
    query: &str,
    client: &reqwest::blocking::Client,
) -> Result<Vec<Repo>, reqwest::Error> {
    let url = format!("{GITHUB_API_ENTRY_POINT}/search/repositories?q={query}&per_page=10");
    let response = client.get(url).send()?;
    let json: HashMap<String, serde_json::Value> = response.json().unwrap();
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
    use crate::globals::CLIENT;
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
    }
    #[test]
    fn test_search() {
        let queries = vec!["Senpwai", "empty-repo", "zohofberibp09u0&_+*"];
        let mut results = Vec::new();
        for (idx, query) in queries.iter().enumerate() {
            println!("\nResults of Search {}\n", idx + 1);
            let res = search(query, &CLIENT).unwrap();
            for r in res.iter() {
                println!("{:?}", r)
            }
            results.push(res);
        }
        assert_eq!(results[0][0].title, "Senpwai");
        assert_eq!(results[2].len(), 0);
    }

    #[test]
    fn test_fetching_release() {
        println!("\nResults for release fetching\n");
        let repo = &REPOS[0]; // Senpwai repo
        let rel = repo.fetch_release(&CLIENT, "2.0.7").unwrap();
        let rel = rel.to_owned().unwrap();
        assert_eq!(
            rel.url,
            "https://github.com/SenZmaKi/Senpwai/releases/download/v2.0.7/Senpwai-setup.exe"
        );
        assert_eq!(rel.version, "2.0.7");
        println!("{:?}", rel);
    }
}
