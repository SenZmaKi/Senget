//! Module for interacting with the Github api

use lazy_static::lazy_static;
use regex::Regex;
use reqwest;
use serde_json;
use std::collections::HashMap;

const GITHUB_HOME_URL: &str = "https://github.com";
const GITHUB_API_ENTRY_POINT: &str = "https://api.github.com";

#[derive(Debug)]
pub struct Repo {
    name: String,
    url: String,
    desc: Option<String>,
    lang: Option<String>,
    tags_url: String,
}

impl Repo {
    fn from(name: String, url: String, desc: Option<String>, lang: Option<String>) -> Repo {
        let s: Vec<&str> = url.split(GITHUB_HOME_URL).collect();
        let tags_url = format!("{}/repos{}/tags", GITHUB_API_ENTRY_POINT, s[1]);
        Repo {
            url,
            name,
            desc,
            lang,
            tags_url,
        }
    }
    fn fetch_latest_version_number(
        &self,
        client: &reqwest::blocking::Client,
    ) -> Result<Option<u16>, reqwest::Error> {
        let response = client.get(&self.tags_url).send()?;
        let json: Vec<serde_json::Value> = response.json().unwrap();
        if json.len() == 0 { // A lenght of 0 means no tags/version numbers were found since the json vector contains each tag/version number  for the repo
            return Ok(None);
        }
        let strip = |s: &str| s.replace(".","").replace( "v", "").replace( "V", "").parse();
        let latest_version: Option<u16> =
            match strip(json[0]["name"].as_str().unwrap()) {
                Ok(ver) => Some(ver),
                _ => None,
            };
        Ok(latest_version)
    }
}

pub fn search(
    query: &str,
    client: &reqwest::blocking::Client,
) -> Result<Vec<Repo>, reqwest::Error> {
    let url = format!("{GITHUB_API_ENTRY_POINT}/search/repositories?q={query}&per_page=10");
    let response = client.get(url).send()?;
    let json: HashMap<String, serde_json::Value> = response
        .json()
        .unwrap();
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
    use super::{reqwest::blocking::Client, search, Repo};
    use crate::globals::CLIENT;

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
        assert_eq!(results[0][0].name, "Senpwai");
        assert_eq!(results[2].len(), 0);
    }

    #[test]
    fn test_version_fetching() {
        let s = String::from;
        let repos = vec![
            Repo::from(
                s("Senpwai"),
                s("https://github.com/SenZmaKi/Senpwai"),
                Some(s("a desktop app for batch downloading anime")),
                Some(s("Python")),
            ),
            Repo::from(
                s("NyakaMwizi"),
                s("https://github.com/SenZmaKi/NyakaMwizi"),
                Some(s("A credit card fraud detection machine learning model")),
                Some(s("Jupyter Notebook")),
            ),
        ];

        for r in repos {
            let v = r.fetch_latest_version_number(&CLIENT).unwrap();
            if r.name == "Senpwai" {
                assert!(v.unwrap() >= 201);
            }
            let v = match v {
                Some(v) => {v.to_string()}
                _ => {s("No valid versions found")}
            };
            println!(
                "Name: {} | Latest version: {}",
                r.name,
                v
            );
        }
    }
}
